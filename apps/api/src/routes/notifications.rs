//! CORE-08 notification-policy foundation. This module owns learner
//! preferences, write-only mobile push registration, and the in-app inbox.
//! It deliberately does not pretend that APNs/FCM delivery exists.

use axum::extract::{Path, State};
use axum::Json;
use chrono::NaiveTime;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct NotificationCategories {
    pub plan_review_reminders: bool,
    pub mock_assignment: bool,
    pub competition: bool,
    pub duel_invitation: bool,
    pub report_resolved: bool,
    pub subscription_events: bool,
}

#[derive(Deserialize)]
pub struct UpdatePreferencesReq {
    pub timezone: String,
    pub quiet_start: Option<String>,
    pub quiet_end: Option<String>,
    pub categories: NotificationCategories,
}

#[derive(Deserialize)]
pub struct RegisterPushTokenReq {
    pub device_id: String,
    pub platform: String,
    pub token: String,
}

fn valid_timezone_text(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 64
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+'))
}

fn parse_quiet_hours(
    start: Option<&str>,
    end: Option<&str>,
) -> ApiResult<(Option<NaiveTime>, Option<NaiveTime>)> {
    match (start, end) {
        (None, None) => Ok((None, None)),
        (Some(start), Some(end)) => {
            let start = NaiveTime::parse_from_str(start, "%H:%M").map_err(|_| {
                ApiError::unprocessable("invalid_quiet_hours", "quiet hours must use HH:MM")
            })?;
            let end = NaiveTime::parse_from_str(end, "%H:%M").map_err(|_| {
                ApiError::unprocessable("invalid_quiet_hours", "quiet hours must use HH:MM")
            })?;
            Ok((Some(start), Some(end)))
        }
        _ => Err(ApiError::unprocessable(
            "invalid_quiet_hours",
            "quiet_start and quiet_end must both be set or both be empty",
        )),
    }
}

fn preferences_json(
    timezone: &str,
    quiet_start: Option<NaiveTime>,
    quiet_end: Option<NaiveTime>,
    plan_review_reminders: bool,
    mock_assignment: bool,
    competition: bool,
    duel_invitation: bool,
    report_resolved: bool,
    subscription_events: bool,
) -> serde_json::Value {
    serde_json::json!({
        "timezone": timezone,
        "quiet_start": quiet_start.map(|v| v.format("%H:%M").to_string()),
        "quiet_end": quiet_end.map(|v| v.format("%H:%M").to_string()),
        "categories": {
            "plan_review_reminders": plan_review_reminders,
            "mock_assignment": mock_assignment,
            "competition": competition,
            "duel_invitation": duel_invitation,
            "report_resolved": report_resolved,
            "subscription_events": subscription_events,
        }
    })
}

pub async fn get_preferences(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query!(
        "INSERT INTO notification_preferences (user_id) VALUES ($1)
         ON CONFLICT (user_id) DO NOTHING",
        user.user_id
    )
    .execute(&state.pool)
    .await?;

    let row = sqlx::query!(
        "SELECT timezone, quiet_start, quiet_end,
                plan_review_reminders, mock_assignment, competition,
                duel_invitation, report_resolved, subscription_events
         FROM notification_preferences WHERE user_id = $1",
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(preferences_json(
        &row.timezone,
        row.quiet_start,
        row.quiet_end,
        row.plan_review_reminders,
        row.mock_assignment,
        row.competition,
        row.duel_invitation,
        row.report_resolved,
        row.subscription_events,
    )))
}

pub async fn update_preferences(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<UpdatePreferencesReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let timezone = req.timezone.trim();
    if !valid_timezone_text(timezone) {
        return Err(ApiError::unprocessable(
            "invalid_timezone",
            "timezone must be a bounded IANA-style identifier",
        ));
    }
    let (quiet_start, quiet_end) =
        parse_quiet_hours(req.quiet_start.as_deref(), req.quiet_end.as_deref())?;

    let categories = req.categories;
    sqlx::query!(
        "INSERT INTO notification_preferences (
            user_id, timezone, quiet_start, quiet_end,
            plan_review_reminders, mock_assignment, competition,
            duel_invitation, report_resolved, subscription_events, updated_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,now())
         ON CONFLICT (user_id) DO UPDATE SET
            timezone = EXCLUDED.timezone,
            quiet_start = EXCLUDED.quiet_start,
            quiet_end = EXCLUDED.quiet_end,
            plan_review_reminders = EXCLUDED.plan_review_reminders,
            mock_assignment = EXCLUDED.mock_assignment,
            competition = EXCLUDED.competition,
            duel_invitation = EXCLUDED.duel_invitation,
            report_resolved = EXCLUDED.report_resolved,
            subscription_events = EXCLUDED.subscription_events,
            updated_at = now()",
        user.user_id,
        timezone,
        quiet_start,
        quiet_end,
        categories.plan_review_reminders,
        categories.mock_assignment,
        categories.competition,
        categories.duel_invitation,
        categories.report_resolved,
        categories.subscription_events,
    )
    .execute(&state.pool)
    .await?;

    Ok(Json(preferences_json(
        timezone,
        quiet_start,
        quiet_end,
        categories.plan_review_reminders,
        categories.mock_assignment,
        categories.competition,
        categories.duel_invitation,
        categories.report_resolved,
        categories.subscription_events,
    )))
}

pub async fn register_push_token(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<RegisterPushTokenReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let device_id = req.device_id.trim();
    let platform = req.platform.trim();
    let token = req.token.trim();
    if device_id.is_empty() || device_id.len() > 128 {
        return Err(ApiError::unprocessable(
            "invalid_device_id",
            "device_id must be 1 to 128 characters",
        ));
    }
    if !matches!(platform, "ios" | "android") {
        return Err(ApiError::unprocessable(
            "invalid_push_platform",
            "platform must be ios or android",
        ));
    }
    if token.is_empty() || token.len() > 4096 {
        return Err(ApiError::unprocessable(
            "invalid_push_token",
            "push token must be 1 to 4096 characters",
        ));
    }

    sqlx::query!(
        "INSERT INTO push_tokens (id, user_id, device_id, platform, token)
         VALUES ($1,$2,$3,$4,$5)
         ON CONFLICT (user_id, device_id, platform) DO UPDATE SET
            token = EXCLUDED.token,
            updated_at = now()",
        Uuid::new_v4(),
        user.user_id,
        device_id,
        platform,
        token,
    )
    .execute(&state.pool)
    .await?;

    Ok(Json(serde_json::json!({"registered": true})))
}

pub async fn inbox(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        "SELECT id, category, title, body, deep_link, campaign_key,
                promotional, created_at, read_at
         FROM notifications
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT 100",
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(serde_json::json!({
        "notifications": rows.iter().map(|row| serde_json::json!({
            "id": row.id,
            "category": row.category,
            "title": row.title,
            "body": row.body,
            "deep_link": row.deep_link,
            "campaign_key": row.campaign_key,
            "promotional": row.promotional,
            "created_at": row.created_at,
            "read_at": row.read_at,
        })).collect::<Vec<_>>()
    })))
}

pub async fn mark_read(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(notification_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query!(
        "UPDATE notifications
         SET read_at = COALESCE(read_at, now())
         WHERE id = $1 AND user_id = $2
         RETURNING read_at",
        notification_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("notification_not_found"))?;

    Ok(Json(serde_json::json!({
        "read": true,
        "read_at": row.read_at,
    })))
}
