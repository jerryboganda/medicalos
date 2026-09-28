//! CORE-01/07 (slice 1 scope): email + password accounts, opaque bearer
//! sessions. SSO, device limits and deletion arrive with the accounts slice.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{hash_password, issue_session, verify_password};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "auth/RegisterRequest.ts",
        rename = "RegisterRequest"
    )
)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "auth/LoginRequest.ts", rename = "LoginRequest")
)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "auth/RegisterResponse.ts",
        rename = "RegisterResponse"
    )
)]
pub struct RegisterResponse {
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    user_id: Uuid,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "auth/LoginResponse.ts", rename = "LoginResponse")
)]
pub struct LoginResponse {
    token: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterReq>,
) -> ApiResult<Json<RegisterResponse>> {
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
    Ok(Json(RegisterResponse { user_id }))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<LoginResponse>> {
    let email = req.email.trim().to_lowercase();
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users
         WHERE email = $1 AND deleted_at IS NULL",
        email
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    // Accounts created through Zitadel have no local password to check.
    let password_ok = user
        .password_hash
        .as_deref()
        .is_some_and(|hash| verify_password(&req.password, hash));
    if !password_ok {
        return Err(ApiError::unauthorized());
    }
    let token = issue_session(&state.pool, user.id).await?;
    Ok(Json(LoginResponse { token }))
}
