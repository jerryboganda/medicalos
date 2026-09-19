//! CORE-08: in-app notification inbox + per-category preferences (§6.4).
//! Email stays account-only; remote push registers tokens when the owned
//! plugin lands (§20.1) — the inbox and preference surface ship now, and
//! reminder copy rules (never shaming, §2.2) live with the senders.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Create an in-app notification, honouring the learner's per-category
/// preference. Callers own the copy: never shaming, factual (§6.4, §2.2).
pub async fn deliver(
    state: &AppState,
    user_id: Uuid,
    category: &str,
    title: &str,
    body: &str,
    deep_link: Option<&str>,
) -> ApiResult<bool> {
    let allowed = match category {
        "plan" => sqlx::query_scalar!(
            "SELECT plan_reminders FROM notification_preferences WHERE user_id = $1",
            user_id
        ),
        "mock_result" => sqlx::query_scalar!(
            "SELECT mock_results FROM notification_preferences WHERE user_id = $1",
            user_id
        ),
        "report" => sqlx::query_scalar!(
            "SELECT reports FROM notification_preferences WHERE user_id = $1",
            user_id
        ),
        _ => sqlx::query_scalar!(
            "SELECT true AS \"ok!\" WHERE NOT EXISTS (
                SELECT 1 FROM notification_preferences WHERE user_id = $1
             )",
            user_id
        ),
    }
    .fetch_optional(&state.pool)
    .await?;
    // No preference row = defaults (all on). An explicit false mutes.
    if allowed == Some(false) {
        return Ok(false);
    }
    sqlx::query!(
        "INSERT INTO notifications (id, user_id, category, title, body, deep_link)
         VALUES ($1, $2, $3, $4, $5, $6)",
        Uuid::new_v4(),
        user_id,
        category,
        title,
        body,
        deep_link
    )
    .execute(&state.pool)
    .await?;
    Ok(true)
}

pub async fn inbox(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT id, category, title, body, deep_link, read_at, created_at
           FROM notifications WHERE user_id = $1
           ORDER BY created_at DESC LIMIT 100"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "category": r.category,
                "title": r.title,
                "body": r.body,
                "deep_link": r.deep_link,
                "read": r.read_at.is_some(),
                "created_at": r.created_at,
            })
        })
        .collect();
    Ok(Json(json!({ "notifications": items })))
}

pub async fn mark_read(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(note_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let updated = sqlx::query!(
        "UPDATE notifications SET read_at = now()
         WHERE id = $1 AND user_id = $2 AND read_at IS NULL",
        note_id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::not_found("notification_not_found"));
    }
    Ok(Json(json!({ "read": true })))
}

#[derive(Deserialize)]
pub struct PrefReq {
    pub plan_reminders: Option<bool>,
    pub mock_results: Option<bool>,
    pub reports: Option<bool>,
    pub quiet_hours_start: Option<i32>,
    pub quiet_hours_end: Option<i32>,
}

pub async fn update_preferences(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<PrefReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if let Some(s) = req.quiet_hours_start {
        if !(0..=23).contains(&s) || req.quiet_hours_end.is_some_and(|e| !(0..=23).contains(&e)) {
            return Err(ApiError::unprocessable(
                "invalid_quiet_hours",
                "quiet hours must be hours of day (0-23)",
            ));
        }
    }
    sqlx::query!(
        "INSERT INTO notification_preferences
           (user_id, plan_reminders, mock_results, reports, quiet_hours_start, quiet_hours_end)
         VALUES ($1,
                 COALESCE($2, true), COALESCE($3, true), COALESCE($4, true),
                 COALESCE($5, 22), COALESCE($6, 7))
         ON CONFLICT (user_id) DO UPDATE SET
           plan_reminders = COALESCE($2, notification_preferences.plan_reminders),
           mock_results = COALESCE($3, notification_preferences.mock_results),
           reports = COALESCE($4, notification_preferences.reports),
           quiet_hours_start = COALESCE($5, notification_preferences.quiet_hours_start),
           quiet_hours_end = COALESCE($6, notification_preferences.quiet_hours_end)",
        user.user_id,
        req.plan_reminders,
        req.mock_results,
        req.reports,
        req.quiet_hours_start,
        req.quiet_hours_end
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "updated": true })))
}
