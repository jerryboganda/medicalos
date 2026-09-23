//! Password hashing (Argon2id), opaque bearer tokens, and the authenticated
//! user extractor. Tokens are stored hashed (SHA-256) — the webview and the
//! database never hold the raw token beyond the response (§25).

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::{Duration, Utc};
use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub struct NewSession {
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

pub fn hash_password(password: &str) -> ApiResult<String> {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::Argon2;
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| ApiError::internal())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::PasswordVerifier;
    use argon2::Argon2;
    match argon2::PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn new_session_token() -> NewSession {
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();
    NewSession {
        expires_at: Utc::now() + Duration::days(30),
        token,
    }
}

/// Create the shared application session for password and OIDC logins.
/// Locking the user row makes single-active-session revocation atomic.
pub async fn issue_session(pool: &sqlx::PgPool, user_id: Uuid) -> ApiResult<String> {
    let mut tx = pool.begin().await?;
    let user = sqlx::query(
        "SELECT single_active_session FROM users
         WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    let single_active: bool = user.try_get("single_active_session")?;
    if single_active {
        sqlx::query(
            "UPDATE auth_sessions SET revoked_at = now()
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }
    let session = new_session_token();
    sqlx::query(
        "INSERT INTO auth_sessions (token_hash, user_id, expires_at)
         VALUES ($1, $2, $3)",
    )
    .bind(sha256_hex(&session.token))
    .bind(user_id)
    .bind(session.expires_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(session.token)
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let out = hasher.finalize();
    out.iter().map(|b| format!("{:02x}", b)).collect()
}

pub struct AuthUser {
    pub user_id: Uuid,
}

impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> ApiResult<Self> {
        let raw = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(ApiError::unauthorized)?;
        let token_hash = sha256_hex(raw);
        let row = sqlx::query!(
            "SELECT user_id FROM auth_sessions WHERE token_hash = $1 AND expires_at > now() AND revoked_at IS NULL",
            token_hash
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(ApiError::unauthorized)?;
        Ok(AuthUser {
            user_id: row.user_id,
        })
    }
}
