//! CORE-01/07 (slice 1 scope): email + password accounts, opaque bearer
//! sessions. SSO, device limits and deletion arrive with the accounts slice.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{hash_password, new_session_token, sha256_hex, verify_password};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let email = req.email.trim().to_lowercase();
    if !email.contains('@') || email.len() < 3 || email.len() > 254 {
        return Err(ApiError::unprocessable(
            "invalid_email",
            "email is not valid",
        ));
    }
    if req.password.len() < 8 {
        return Err(ApiError::unprocessable(
            "weak_password",
            "password must be at least 8 characters",
        ));
    }
    let exists = sqlx::query!("SELECT 1 AS one FROM users WHERE email = $1", email)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_some() {
        return Err(ApiError::conflict(
            "email_taken",
            "email is already registered",
        ));
    }
    let hash = hash_password(&req.password)?;
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)",
        user_id,
        email,
        hash
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({"user_id": user_id})))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let email = req.email.trim().to_lowercase();
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE email = $1",
        email
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::unauthorized())?;
    if !verify_password(&req.password, &user.password_hash) {
        return Err(ApiError::unauthorized());
    }
    let session = new_session_token();
    sqlx::query!(
        "INSERT INTO auth_sessions (token_hash, user_id, expires_at)
         VALUES ($1, $2, $3)",
        sha256_hex(&session.token),
        user.id,
        session.expires_at
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({"token": session.token})))
}
