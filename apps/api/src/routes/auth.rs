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

/// §6.3 login throttling: 10 consecutive wrong passwords lock the account,
/// backing off exponentially (2^(n-10) minutes) and capped at 15.
const LOGIN_LOCK_THRESHOLD: i32 = 10;
const LOGIN_LOCK_CAP_MINUTES: i32 = 15;

fn lock_minutes(failures: i32) -> i32 {
    (1i32 << (failures - LOGIN_LOCK_THRESHOLD).clamp(0, 20)).min(LOGIN_LOCK_CAP_MINUTES)
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<LoginResponse>> {
    let email = req.email.trim().to_lowercase();
    // Throttle first: a locked account never reaches the password verify.
    let throttle = sqlx::query!(
        "SELECT consecutive_failures, locked_until FROM login_throttle WHERE email = $1",
        email
    )
    .fetch_optional(&state.pool)
    .await?;
    if let Some(row) = &throttle {
        if let Some(locked_until) = row.locked_until {
            let now = chrono::Utc::now();
            if locked_until > now {
                let retry_after = (locked_until - now).num_seconds().max(1);
                return Err(ApiError::locked(
                    "login_locked",
                    "too many failed sign-in attempts; try again later",
                    serde_json::json!({ "retry_after_seconds": retry_after }),
                ));
            }
        }
    }
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users
         WHERE email = $1 AND deleted_at IS NULL",
        email
    )
    .fetch_optional(&state.pool)
    .await?;
    // Accounts created through Zitadel have no local password to check.
    let password_ok = user.as_ref().is_some_and(|user| {
        user.password_hash
            .as_deref()
            .is_some_and(|hash| verify_password(&req.password, hash))
    });
    if !password_ok {
        // Count every failed attempt against the address, whether or not the
        // account exists (the response stays 401 either way).
        let failures = throttle
            .as_ref()
            .map(|row| row.consecutive_failures + 1)
            .unwrap_or(1);
        let locked_until = (failures >= LOGIN_LOCK_THRESHOLD)
            .then(|| chrono::Utc::now() + chrono::Duration::minutes(lock_minutes(failures).into()));
        sqlx::query!(
            "INSERT INTO login_throttle (email, consecutive_failures, locked_until, updated_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (email) DO UPDATE SET
                consecutive_failures = $2,
                locked_until = $3,
                updated_at = now()",
            email,
            failures,
            locked_until
        )
        .execute(&state.pool)
        .await?;
        return Err(ApiError::unauthorized());
    }
    let user = user.expect("checked above");
    let token = issue_session(&state.pool, user.id).await?;
    // A successful sign-in forgives the failure history.
    sqlx::query!("DELETE FROM login_throttle WHERE email = $1", email)
        .execute(&state.pool)
        .await?;
    Ok(Json(LoginResponse { token }))
}
