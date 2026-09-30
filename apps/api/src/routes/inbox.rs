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

/// ENG-01 reminder tail, in-app half: one factual "question of the day is
/// ready" notification per learner per day, only when the learner opted
/// into QOTD with an exam, has not answered today, has plan reminders on,
/// and is outside their quiet hours. The engagement kill switch silences
/// everything. Remote delivery waits on the owned push plugin (§20.1).
/// Copy rules (§2.2): never shaming — the body states availability, never
/// streaks, guilt, or comparisons.
pub(crate) async fn send_daily_qotd_reminders(state: &AppState) -> ApiResult<usize> {
    if !crate::routes::engagement::engagement_global_enabled(state).await? {
        return Ok(0);
    }
    use chrono::Timelike;
    let now_hour = chrono::Utc::now().time().hour() as i32;
    // Learners who never touched their preferences keep the defaults
    // (reminders on, quiet 22–07) — hence the LEFT JOIN with COALESCE.
    // The delivered copy links to /today, where the QOTD card lives.
    let candidates = sqlx::query!(
        r#"SELECT es.user_id, es.qotd_exam_id,
                  COALESCE(np.plan_reminders, TRUE) AS "plan_reminders!",
                  COALESCE(np.quiet_hours_start, 22) AS "quiet_hours_start!",
                  COALESCE(np.quiet_hours_end, 7) AS "quiet_hours_end!"
           FROM engagement_settings es
           JOIN users u ON u.id = es.user_id AND u.deleted_at IS NULL
           LEFT JOIN notification_preferences np ON np.user_id = es.user_id
           WHERE es.qotd_enabled = TRUE
             AND es.qotd_exam_id IS NOT NULL
             AND NOT EXISTS (
                 SELECT 1 FROM qotd_answers a
                 WHERE a.user_id = es.user_id AND a.day = CURRENT_DATE)
             AND NOT EXISTS (
                 SELECT 1 FROM notifications n
                 WHERE n.user_id = es.user_id AND n.category = 'plan'
                   AND n.deep_link = '/today'
                   AND n.created_at::date = CURRENT_DATE)
           LIMIT 500"#,
    )
    .fetch_all(&state.pool)
    .await?;

    let mut conn = state.pool.acquire().await?;
    let mut sent = 0;
    for candidate in candidates {
        if in_quiet_hours(
            now_hour,
            candidate.quiet_hours_start,
            candidate.quiet_hours_end,
        ) {
            continue;
        }
        if !candidate.plan_reminders {
            continue;
        }
        // Ensure today's question exists — the same deterministic pick the
        // GET serves — so the reminder never advertises nothing.
        let Some(_) = crate::routes::engagement::selected_qotd_id(
            &mut conn,
            candidate.qotd_exam_id.expect("qotd_exam_id IS NOT NULL"),
        )
        .await?
        else {
            continue; // no eligible question today — remind of nothing
        };
        if deliver(
            state,
            candidate.user_id,
            "plan",
            "Question of the day",
            "A new question of the day is available on your Today page.",
            Some("/today"),
        )
        .await?
        {
            sent += 1;
        }
    }
    Ok(sent)
}

/// A zero-length window is no window; a wrapping window (22 → 07) covers
/// the midnight crossing.
fn in_quiet_hours(hour: i32, start: i32, end: i32) -> bool {
    if start == end {
        false
    } else if start < end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
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
        || NotificationPreferences {
            plan_reminders: true,
            mock_results: true,
            reports: true,
            content_updates: true,
            quiet_hours_start: 22,
            quiet_hours_end: 7,
        },
        |p| NotificationPreferences {
            plan_reminders: p.plan_reminders,
            mock_results: p.mock_results,
            reports: p.reports,
            content_updates: p.content_updates,
            quiet_hours_start: p.quiet_hours_start,
            quiet_hours_end: p.quiet_hours_end,
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
    if req
        .quiet_hours_start
        .is_some_and(|hour| !(0..=23).contains(&hour))
        || req
            .quiet_hours_end
            .is_some_and(|hour| !(0..=23).contains(&hour))
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
    Ok(Json(NotificationPreferencesUpdateResponse {
        updated: true,
    }))
}
