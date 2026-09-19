//! CORE-07: verified personal accounts, device-aware rotating sessions,
//! recovery, and in-app deletion initiation. External mail/social identity
//! adapters are intentionally separate boundaries.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{
    hash_password, new_access_token, new_challenge_token, new_refresh_token, sha256_hex,
    verify_password, AuthUser,
};
use crate::error::{ApiError, ApiResult};
use crate::routes::guest_trial;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
    pub guest_trial_token: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
    pub device_id: String,
    pub device_name: String,
}

#[derive(Deserialize)]
pub struct TokenReq {
    pub token: String,
}

#[derive(Deserialize)]
pub struct RefreshReq {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordReq {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordReq {
    pub token: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct DeleteAccountReq {
    pub password: String,
}

fn normalized_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn validate_email(email: &str) -> ApiResult<()> {
    if !email.contains('@') || email.len() < 3 || email.len() > 254 {
        return Err(ApiError::unprocessable(
            "invalid_email",
            "email is not valid",
        ));
    }
    Ok(())
}

fn validate_password(password: &str) -> ApiResult<()> {
    if password.len() < 8 {
        return Err(ApiError::unprocessable(
            "weak_password",
            "password must be at least 8 characters",
        ));
    }
    Ok(())
}

fn validate_device(device_id: &str, device_name: &str) -> ApiResult<(String, String)> {
    let device_id = device_id.trim();
    let device_name = device_name.trim();
    if device_id.is_empty() || device_id.len() > 128 {
        return Err(ApiError::unprocessable(
            "invalid_device_id",
            "device_id must be between 1 and 128 characters",
        ));
    }
    if device_name.is_empty() || device_name.len() > 120 {
        return Err(ApiError::unprocessable(
            "invalid_device_name",
            "device_name must be between 1 and 120 characters",
        ));
    }
    Ok((device_id.to_string(), device_name.to_string()))
}

fn require_auth_challenge_delivery(state: &AppState) -> ApiResult<()> {
    // ponytail: the test-token seam is the only current delivery path; replace
    // this guard when a real mail adapter is configured.
    if !state.expose_test_auth_tokens {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "auth_email_delivery_unavailable",
            message: "account email delivery is not configured".into(),
            details: None,
        });
    }
    Ok(())
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let email = normalized_email(&req.email);
    validate_email(&email)?;
    validate_password(&req.password)?;
    require_auth_challenge_delivery(&state)?;

    let hash = hash_password(&req.password)?;
    let user_id = Uuid::new_v4();
    let challenge = new_challenge_token();
    let mut tx = state.pool.begin().await?;

    let inserted = sqlx::query!(
        "INSERT INTO users (id, email, password_hash)
         VALUES ($1, $2, $3)
         ON CONFLICT (email) DO NOTHING
         RETURNING id",
        user_id,
        email,
        hash
    )
    .fetch_optional(&mut *tx)
    .await?;
    if inserted.is_none() {
        return Err(ApiError::conflict(
            "email_taken",
            "email is already registered",
        ));
    }

    let guest_answers_migrated = if let Some(token) = req.guest_trial_token.as_deref() {
        guest_trial::migrate_into_user(&mut tx, user_id, token).await?
    } else {
        0
    };

    sqlx::query!(
        "INSERT INTO email_verification_challenges (token_hash, user_id, expires_at)
         VALUES ($1, $2, $3)",
        sha256_hex(&challenge.token),
        user_id,
        challenge.expires_at
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let mut body = serde_json::json!({
        "user_id": user_id,
        "verification_required": true,
        "guest_answers_migrated": guest_answers_migrated
    });
    if state.expose_test_auth_tokens {
        body["verification_token"] = serde_json::json!(challenge.token);
    }
    Ok(Json(body))
}

pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TokenReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    let challenge = sqlx::query!(
        "UPDATE email_verification_challenges
         SET used_at = now()
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
         RETURNING user_id",
        sha256_hex(req.token.trim())
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::unprocessable(
            "verification_invalid_or_expired",
            "verification token is invalid or expired",
        )
    })?;

    sqlx::query!(
        "UPDATE users
         SET email_verified_at = COALESCE(email_verified_at, now())
         WHERE id = $1",
        challenge.user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({"verified": true})))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let email = normalized_email(&req.email);
    let (device_id, device_name) = validate_device(&req.device_id, &req.device_name)?;
    let mut tx = state.pool.begin().await?;

    let user = sqlx::query!(
        "SELECT id, password_hash, email_verified_at, deletion_requested_at
         FROM users WHERE email = $1 FOR UPDATE",
        email
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    if !verify_password(&req.password, &user.password_hash) {
        return Err(ApiError::unauthorized());
    }
    if user.deletion_requested_at.is_some() {
        return Err(ApiError::forbidden(
            "account_deletion_pending",
            "account deletion is pending",
        ));
    }
    if user.email_verified_at.is_none() {
        return Err(ApiError::forbidden(
            "email_not_verified",
            "verify your email before signing in",
        ));
    }

    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE user_id = $1 AND device_id = $2
           AND revoked_at IS NULL AND refresh_expires_at > now()",
        user.id,
        device_id
    )
    .execute(&mut *tx)
    .await?;

    let active = sqlx::query!(
        r#"SELECT COUNT(*) AS "n!" FROM auth_sessions
           WHERE user_id = $1
             AND revoked_at IS NULL
             AND refresh_expires_at > now()"#,
        user.id
    )
    .fetch_one(&mut *tx)
    .await?
    .n;
    if active >= 2 {
        return Err(ApiError::conflict(
            "device_limit_reached",
            "this account already has two active devices",
        ));
    }

    let access = new_access_token();
    let refresh = new_refresh_token();
    let session_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO auth_sessions
           (token_hash, user_id, expires_at, id, device_id, device_name,
            refresh_token_hash, refresh_expires_at, last_seen_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())",
        sha256_hex(&access.token),
        user.id,
        access.expires_at,
        session_id,
        device_id,
        device_name,
        sha256_hex(&refresh.token),
        refresh.expires_at
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "token": access.token,
        "refresh_token": refresh.token,
        "session_id": session_id,
        "expires_at": access.expires_at
    })))
}

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let old_hash = sha256_hex(req.refresh_token.trim());
    let access = new_access_token();
    let refresh = new_refresh_token();
    let row = sqlx::query!(
        "UPDATE auth_sessions
         SET token_hash = $1,
             refresh_token_hash = $2,
             expires_at = $3,
             refresh_expires_at = $4,
             last_seen_at = now()
         WHERE refresh_token_hash = $5
           AND revoked_at IS NULL
           AND refresh_expires_at > now()
         RETURNING id",
        sha256_hex(&access.token),
        sha256_hex(&refresh.token),
        access.expires_at,
        refresh.expires_at,
        old_hash
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;

    Ok(Json(serde_json::json!({
        "token": access.token,
        "refresh_token": refresh.token,
        "session_id": row.id,
        "expires_at": access.expires_at
    })))
}

pub async fn sessions(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        "SELECT id, device_id, device_name, created_at, last_seen_at, expires_at, refresh_expires_at
         FROM auth_sessions
         WHERE user_id = $1
           AND revoked_at IS NULL
           AND refresh_expires_at > now()
         ORDER BY last_seen_at DESC",
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let sessions: Vec<_> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "session_id": row.id,
                "device_id": row.device_id,
                "device_name": row.device_name,
                "created_at": row.created_at,
                "last_seen_at": row.last_seen_at,
                "expires_at": row.expires_at,
                "refresh_expires_at": row.refresh_expires_at,
                "current": row.id == user.session_id
            })
        })
        .collect();
    Ok(Json(serde_json::json!({"sessions": sessions})))
}

pub async fn sign_out_others(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE user_id = $1 AND id <> $2
           AND revoked_at IS NULL AND refresh_expires_at > now()",
        user.user_id,
        user.session_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({"revoked": result.rows_affected()})))
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
        user.session_id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({"signed_out": true})))
}

pub async fn forgot_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ForgotPasswordReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    require_auth_challenge_delivery(&state)?;
    let email = normalized_email(&req.email);
    let user = sqlx::query!(
        "SELECT id FROM users
         WHERE email = $1
           AND email_verified_at IS NOT NULL
           AND deletion_requested_at IS NULL",
        email
    )
    .fetch_optional(&state.pool)
    .await?;

    let mut body = serde_json::json!({"accepted": true});
    if let Some(user) = user {
        let challenge = new_challenge_token();
        let mut tx = state.pool.begin().await?;
        sqlx::query!(
            "UPDATE password_reset_challenges SET used_at = now()
             WHERE user_id = $1 AND used_at IS NULL",
            user.id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO password_reset_challenges (token_hash, user_id, expires_at)
             VALUES ($1, $2, $3)",
            sha256_hex(&challenge.token),
            user.id,
            challenge.expires_at
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        if state.expose_test_auth_tokens {
            body["reset_token"] = serde_json::json!(challenge.token);
        }
    }
    Ok((StatusCode::ACCEPTED, Json(body)))
}

pub async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResetPasswordReq>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_password(&req.password)?;
    let new_hash = hash_password(&req.password)?;
    let mut tx = state.pool.begin().await?;
    let challenge = sqlx::query!(
        "UPDATE password_reset_challenges
         SET used_at = now()
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
         RETURNING user_id",
        sha256_hex(req.token.trim())
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::unprocessable(
            "reset_invalid_or_expired",
            "password reset token is invalid or expired",
        )
    })?;
    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2",
        new_hash,
        challenge.user_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE user_id = $1 AND revoked_at IS NULL",
        challenge.user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({"reset": true})))
}

pub async fn request_account_deletion(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<DeleteAccountReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let mut tx = state.pool.begin().await?;
    let account = sqlx::query!(
        "SELECT password_hash FROM users WHERE id = $1 FOR UPDATE",
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if !verify_password(&req.password, &account.password_hash) {
        return Err(ApiError::unauthorized());
    }
    sqlx::query!(
        "UPDATE users SET deletion_requested_at = COALESCE(deletion_requested_at, now())
         WHERE id = $1",
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE user_id = $1 AND revoked_at IS NULL",
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"status": "pending"})),
    ))
}
