//! CORE-07: learner-controlled devices and account lifecycle.

use axum::extract::{Extension, Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{hash_password, AuthSession, AuthUser};
use crate::error::{ApiError, ApiResult};
use crate::routes::admin::{admin_headers, audit};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct DeviceReq {
    pub device_key: String,
    pub label: Option<String>,
}

pub async fn register_device(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Extension(session): Extension<AuthSession>,
    Json(req): Json<DeviceReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let key = req.device_key.trim();
    if key.is_empty() || key.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_device_key",
            "device_key must be 1-200 characters",
        ));
    }
    let label = req
        .label
        .as_deref()
        .unwrap_or("device")
        .trim()
        .chars()
        .take(100)
        .collect::<String>();
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;

    let auth_session = sqlx::query!(
        "SELECT device_id, created_at FROM auth_sessions
         WHERE token_hash = $1 AND user_id = $2 AND expires_at > now()
           AND revoked_at IS NULL FOR UPDATE",
        &session.token_hash,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;

    let existing = sqlx::query!(
        "SELECT id, revoked_at FROM user_devices
         WHERE user_id = $1 AND device_key = $2 FOR UPDATE",
        user.user_id,
        key
    )
    .fetch_optional(&mut *tx)
    .await?;

    // A revoked row can be reactivated only by a session created after the
    // revocation. Keep this check ahead of the limit check so a legacy bearer
    // cannot obscure the reason its device registration was refused.
    if existing
        .as_ref()
        .and_then(|row| row.revoked_at.as_ref())
        .is_some_and(|revoked_at| auth_session.created_at <= *revoked_at)
    {
        return Err(ApiError::conflict(
            "device_revoked",
            "sign in again before registering this revoked device",
        ));
    }

    // CORE-07: reactivating a revoked device consumes an active-device slot.
    if existing.is_none()
        || existing
            .as_ref()
            .is_some_and(|row| row.revoked_at.is_some())
    {
        let limit = sqlx::query_scalar!(
            r#"SELECT max_devices AS "max_devices!" FROM users WHERE id = $1"#,
            user.user_id
        )
        .fetch_one(&mut *tx)
        .await?;
        let active = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "active!" FROM user_devices
               WHERE user_id = $1 AND revoked_at IS NULL"#,
            user.user_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if active >= i64::from(limit) {
            return Err(ApiError::forbidden_with_details(
                "devices_exhausted",
                format!("device limit reached ({limit}) - revoke a device first"),
                serde_json::json!({ "device_limit": { "limit": limit, "active": active } }),
            ));
        }
    }

    if auth_session
        .device_id
        .as_deref()
        .is_some_and(|bound| bound != key)
    {
        return Err(ApiError::conflict(
            "device_session_conflict",
            "this session is already bound to another device",
        ));
    }
    let row = sqlx::query!(
        r#"INSERT INTO user_devices (id, user_id, device_key, label)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (user_id, device_key) DO UPDATE
             SET label = EXCLUDED.label, last_seen_at = now(), revoked_at = NULL
           RETURNING id, device_key, label, created_at, last_seen_at"#,
        Uuid::new_v4(),
        user.user_id,
        key,
        label
    )
    .fetch_one(&mut *tx)
    .await?;

    if auth_session.device_id.is_none() {
        let bound = sqlx::query!(
            "UPDATE auth_sessions SET device_id = $1
             WHERE token_hash = $2 AND user_id = $3 AND device_id IS NULL
               AND expires_at > now() AND revoked_at IS NULL",
            key,
            &session.token_hash,
            user.user_id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if bound != 1 {
            return Err(ApiError::unauthorized());
        }
    }

    tx.commit().await?;
    Ok(Json(json!({
        "device_id": row.id,
        "device_key": row.device_key,
        "label": row.label,
        "created_at": row.created_at,
        "last_seen_at": row.last_seen_at,
    })))
}

pub async fn list_devices(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT id, device_key, label, created_at, last_seen_at, revoked_at
           FROM user_devices
           WHERE user_id = $1
           ORDER BY last_seen_at DESC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let devices: Vec<_> = rows
        .into_iter()
        .map(|r| {
            json!({
                "device_id": r.id,
                "device_key": r.device_key,
                "label": r.label,
                "created_at": r.created_at,
                "last_seen_at": r.last_seen_at,
                "revoked_at": r.revoked_at,
            })
        })
        .collect();
    Ok(Json(json!({ "devices": devices })))
}

pub async fn revoke_device(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(device_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    let row = sqlx::query!(
        "UPDATE user_devices SET revoked_at = now()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
         RETURNING device_key",
        device_id,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("device_not_found"))?;
    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE user_id = $1 AND device_id = $2 AND revoked_at IS NULL",
        user.user_id,
        row.device_key
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "revoked": true })))
}

pub async fn delete_account(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now() WHERE user_id = $1",
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE user_devices SET revoked_at = now() WHERE user_id = $1",
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE users SET deleted_at = now() WHERE id = $1",
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "deleted": true })))
}

// ---- CORE-07: the single-active-session policy toggle ------------------------

#[derive(Deserialize)]
pub struct SessionPolicyReq {
    pub single_active_session: bool,
}

pub async fn get_session_policy(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let single_active_session = sqlx::query_scalar!(
        r#"SELECT single_active_session AS "single_active_session!"
           FROM users WHERE id = $1 AND deleted_at IS NULL"#,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    Ok(Json(
        json!({ "single_active_session": single_active_session }),
    ))
}

/// When on, the next login retires every prior session for this account.
pub async fn set_session_policy(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<SessionPolicyReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    sqlx::query!(
        "UPDATE users SET single_active_session = $2 WHERE id = $1 AND deleted_at IS NULL",
        user.user_id,
        req.single_active_session
    )
    .execute(&mut *tx)
    .await?;
    if req.single_active_session {
        // Turning the policy on takes effect immediately for this account.
        sqlx::query!(
            "UPDATE auth_sessions SET revoked_at = now()
             WHERE user_id = $1 AND revoked_at IS NULL",
            user.user_id
        )
        .execute(&mut *tx)
        .await?;
    }
    audit(
        &mut *tx,
        user.user_id,
        "single_active_session_policy_changed",
        "user",
        user.user_id,
        json!({ "single_active_session": req.single_active_session }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(
        json!({ "single_active_session": req.single_active_session }),
    ))
}

#[derive(Deserialize)]
pub struct AdminResetPasswordReq {
    pub new_password: String,
}

/// Operator-assisted password reset (pilot stop-gap until an email
/// delivery seam exists — see .scratch/auth-hardening/spec.md). Admin-gated
/// through the shared seam (role session or legacy token), hashes the new
/// password like registration does, and revokes every live session so a
/// stolen session does not survive the reset. Audited with the target,
/// never with the password.
pub async fn admin_reset_password(
    State(state): State<Arc<AppState>>,
    operator: AuthUser,
    headers: axum::http::HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(req): Json<AdminResetPasswordReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(&operator, admin_headers(&headers))?;
    if req.new_password.len() < 8 {
        return Err(ApiError::unprocessable(
            "weak_password",
            "password must be at least 8 characters",
        ));
    }
    let hash = hash_password(&req.new_password)?;
    let mut tx = state.pool.begin().await?;
    let updated = sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2 AND deleted_at IS NULL",
        hash,
        user_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(ApiError::not_found("user_not_found"));
    }
    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE user_id = $1 AND revoked_at IS NULL",
        user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    audit(
        &state.pool,
        operator.user_id,
        "admin_password_reset",
        "user",
        user_id,
        json!({ "sessions_revoked": true }),
    )
    .await?;
    Ok(Json(json!({ "reset": true })))
}
