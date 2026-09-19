//! OPS-06 config resolution + ADMIN-06 settings surface (§19.5 baseline).
//! Settings keys mirror §19.5: mastery bands, competition scoring, community
//! sample, free-tier allowance, review caps, offline lease, integrity rules.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SettingsReq {
    pub mastery_bands: Option<Vec<i32>>,
    pub community_min_sample: Option<i64>,
    pub free_daily_questions: Option<i64>,
    pub free_daily_coach_turns: Option<i64>,
    pub retest_intervals_days: Option<Vec<i64>>,
}

/// §19.5 settings: upsert per-key. Readers resolve env default first, then
/// the stored override — keeping the runtime hot-start simple.
pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<SettingsReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    let mut updated: Vec<&str> = Vec::new();
    let pairs: Vec<(&str, serde_json::Value)> = vec![
        ("mastery_bands", req.mastery_bands.map(|b| json!(b))),
        (
            "community_min_sample",
            req.community_min_sample.map(|v| json!(v)),
        ),
        (
            "free_daily_questions",
            req.free_daily_questions.map(|v| json!(v)),
        ),
        (
            "free_daily_coach_turns",
            req.free_daily_coach_turns.map(|v| json!(v)),
        ),
        (
            "retest_intervals_days",
            req.retest_intervals_days.map(|v| json!(v)),
        ),
    ]
    .to_vec();
    for (key, value) in pairs {
        if let Some(v) = value {
            sqlx::query!(
                "INSERT INTO app_settings (key, value) VALUES ($1, $2)
                 ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = now()",
                key,
                v
            )
            .execute(&state.pool)
            .await?;
            updated.push(key);
        }
    }
    Ok(Json(json!({ "updated": updated })))
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    let rows = sqlx::query!("SELECT key, value FROM app_settings ORDER BY key")
        .fetch_all(&state.pool)
        .await?;
    let settings: serde_json::Map<String, serde_json::Value> =
        rows.into_iter().map(|r| (r.key, r.value)).collect();
    Ok(Json(json!({ "settings": settings })))
}

/// Learner-safe config: only non-sensitive runtime settings.
pub async fn public_settings(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query!(
        r#"SELECT COALESCE(value->>'free_daily_questions', '') AS "free!" FROM app_settings WHERE key = 'free_daily_questions'"#
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(Json(json!({
        "free_daily_questions": row.map(|r| r.free).unwrap_or_default(),
    })))
}

// keep Uuid import used (route arities)
#[allow(dead_code)]
fn _uuid_check(_: Option<Uuid>) {}
