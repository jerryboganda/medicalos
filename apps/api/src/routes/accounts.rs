//! CORE-07: learner-controlled devices and account lifecycle.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct DeviceReq {
    pub device_key: String,
    pub label: Option<String>,
}

pub async fn register_device(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
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
    let id = Uuid::new_v4();
    let row = sqlx::query!(
        r#"INSERT INTO user_devices (id, user_id, device_key, label)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (user_id, device_key) DO UPDATE
             SET label = EXCLUDED.label, last_seen_at = now(), revoked_at = NULL
           RETURNING id, device_key, label, created_at, last_seen_at"#,
        id,
        user.user_id,
        key,
        label
    )
    .fetch_one(&state.pool)
    .await?;
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
    let row = sqlx::query!(
        r#"UPDATE user_devices
           SET revoked_at = now()
           WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
           RETURNING device_key"#,
        device_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("device_not_found"))?;
    sqlx::query!(
        r#"UPDATE auth_sessions
           SET revoked_at = now()
           WHERE user_id = $1 AND device_id = $2 AND revoked_at IS NULL"#,
        user.user_id,
        row.device_key
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "revoked": true })))
}

pub async fn delete_account(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
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
