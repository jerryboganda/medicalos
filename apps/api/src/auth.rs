//! Password hashing (Argon2id), opaque bearer/refresh tokens, and the
//! authenticated user extractor. Raw tokens exist only at issuance; durable
//! storage is SHA-256 hashed (§25).

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::{Duration, Utc};
use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub struct NewToken {
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

pub fn new_token(lifetime: Duration) -> NewToken {
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();
    NewToken {
        expires_at: Utc::now() + lifetime,
        token,
    }
}

pub fn new_access_token() -> NewToken {
    new_token(Duration::minutes(15))
}

pub fn new_refresh_token() -> NewToken {
    new_token(Duration::days(30))
}

pub fn new_challenge_token() -> NewToken {
    new_token(Duration::hours(1))
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let out = hasher.finalize();
    out.iter().map(|b| format!("{:02x}", b)).collect()
}

pub struct AuthUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
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
            "UPDATE auth_sessions
             SET last_seen_at = now()
             WHERE token_hash = $1
               AND expires_at > now()
               AND revoked_at IS NULL
             RETURNING user_id, id",
            token_hash
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(ApiError::unauthorized)?;
        Ok(AuthUser {
            user_id: row.user_id,
            session_id: row.id,
        })
    }
}
