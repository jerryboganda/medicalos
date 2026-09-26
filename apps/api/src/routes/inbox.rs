//! CORE-08: in-app notification inbox + per-category preferences (§6.4).
//! Email stays account-only; remote push registers tokens when the owned
//! plugin lands (§20.1) — the inbox and preference surface ship now, and
//! reminder copy rules (never shaming, §2.2) live with the senders.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "inbox/NotificationPreferences.ts",
        rename = "NotificationPreferences"
    )
)]
pub struct NotificationPreferences {
    plan_reminders: bool,
    mock_results: bool,
    reports: bool,
    content_updates: bool,
    quiet_hours_start: i32,
    quiet_hours_end: i32,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "inbox/NotificationItem.ts",
        rename = "NotificationItem"
    )
)]
pub struct NotificationItem {
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    id: Uuid,
    category: String,
    title: String,
    body: String,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    deep_link: Option<String>,
    read: bool,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "inbox/NotificationInboxResponse.ts",
        rename = "NotificationInboxResponse"
    )
)]
pub struct NotificationInboxResponse {
    notifications: Vec<NotificationItem>,
    preferences: NotificationPreferences,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "inbox/NotificationPreferencesUpdateResponse.ts",
        rename = "NotificationPreferencesUpdateResponse"
    )
)]
pub struct NotificationPreferencesUpdateResponse {
    #[cfg_attr(feature = "type-export", ts(type = "true"))]
    updated: bool,
}

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
        "content_update" => sqlx::query_scalar!(
            "SELECT content_updates FROM notification_preferences WHERE user_id = $1",
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
) -> ApiResult<Json<NotificationInboxResponse>> {
    let rows = sqlx::query!(
        r#"SELECT id, category, title, body, deep_link, read_at, created_at
           FROM notifications WHERE user_id = $1
           ORDER BY created_at DESC LIMIT 100"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let stored_preferences = sqlx::query!(
        r#"SELECT plan_reminders, mock_results, reports, content_updates,
                  quiet_hours_start, quiet_hours_end
           FROM notification_preferences WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let preferences = stored_preferences.map_or_else(
        || {
            NotificationPreferences {
                plan_reminders: true,
                mock_results: true,
                reports: true,
                content_updates: true,
                quiet_hours_start: 22,
                quiet_hours_end: 7,
            }
        },
        |p| {
            NotificationPreferences {
                plan_reminders: p.plan_reminders,
                mock_results: p.mock_results,
                reports: p.reports,
                content_updates: p.content_updates,
                quiet_hours_start: p.quiet_hours_start,
                quiet_hours_end: p.quiet_hours_end,
            }
        },
    );
    let notifications = rows
        .into_iter()
        .map(|r| NotificationItem {
            id: r.id,
            category: r.category,
            title: r.title,
            body: r.body,
            deep_link: r.deep_link,
            read: r.read_at.is_some(),
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(NotificationInboxResponse {
        notifications,
        preferences,
    }))
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
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "inbox/NotificationPreferencesUpdateRequest.ts",
        rename = "NotificationPreferencesUpdateRequest"
    )
)]
pub struct PrefReq {
    #[cfg_attr(
        feature = "type-export",
        ts(type = "boolean", optional = nullable)
    )]
    pub plan_reminders: Option<bool>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "boolean", optional = nullable)
    )]
    pub mock_results: Option<bool>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "boolean", optional = nullable)
    )]
    pub reports: Option<bool>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "boolean", optional = nullable)
    )]
    pub content_updates: Option<bool>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "number", optional = nullable)
    )]
    pub quiet_hours_start: Option<i32>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "number", optional = nullable)
    )]
    pub quiet_hours_end: Option<i32>,
}

pub async fn update_preferences(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<PrefReq>,
) -> ApiResult<Json<NotificationPreferencesUpdateResponse>> {
    if req.quiet_hours_start.is_some_and(|hour| !(0..=23).contains(&hour))
        || req.quiet_hours_end.is_some_and(|hour| !(0..=23).contains(&hour))
    {
        return Err(ApiError::unprocessable(
            "invalid_quiet_hours",
            "quiet hours must be hours of day (0-23)",
        ));
    }
    sqlx::query!(
        "INSERT INTO notification_preferences
           (user_id, plan_reminders, mock_results, reports, content_updates,
            quiet_hours_start, quiet_hours_end)
         VALUES ($1,
                 COALESCE($2, true), COALESCE($3, true), COALESCE($4, true),
                 COALESCE($5, true), COALESCE($6, 22), COALESCE($7, 7))
         ON CONFLICT (user_id) DO UPDATE SET
           plan_reminders = COALESCE($2, notification_preferences.plan_reminders),
           mock_results = COALESCE($3, notification_preferences.mock_results),
           reports = COALESCE($4, notification_preferences.reports),
           content_updates = COALESCE($5, notification_preferences.content_updates),
           quiet_hours_start = COALESCE($6, notification_preferences.quiet_hours_start),
           quiet_hours_end = COALESCE($7, notification_preferences.quiet_hours_end)",
        user.user_id,
        req.plan_reminders,
        req.mock_results,
        req.reports,
        req.content_updates,
        req.quiet_hours_start,
        req.quiet_hours_end
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(NotificationPreferencesUpdateResponse { updated: true }))
}
