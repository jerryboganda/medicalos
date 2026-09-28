//! OPS-06 config resolution + ADMIN-06 settings surface (§19.5 baseline).

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub const DEFAULT_RETEST_INTERVAL_DAYS: [i64; 4] = [1, 3, 7, 14];
pub const DEFAULT_OFFLINE_LEASE_DAYS: i64 = 14;
pub const DEFAULT_MAX_REVIEWS_PER_DAY: i64 = 30;
pub const DEFAULT_MAX_NEW_CARDS_PER_DAY: i64 = 10;
pub const DEFAULT_COMPETITION_DIFFICULTY_POINTS: [i64; 3] = [5, 10, 15];

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "settings/AdminSettingsUpdateRequest.ts",
        rename = "AdminSettingsUpdateRequest"
    )
)]
#[serde(deny_unknown_fields)]
pub struct SettingsReq {
    pub mastery_bands: Option<Vec<i32>>,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    pub community_min_sample: Option<i64>,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    pub free_daily_questions: Option<i64>,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    pub free_daily_coach_turns: Option<i64>,
    #[cfg_attr(feature = "type-export", ts(type = "Array<number> | null"))]
    pub retest_intervals_days: Option<Vec<i64>>,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    pub offline_lease_days: Option<i64>,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    pub max_reviews_per_day: Option<i64>,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    pub max_new_cards_per_day: Option<i64>,
    #[cfg_attr(feature = "type-export", ts(type = "Array<number> | null"))]
    pub competition_difficulty_points: Option<Vec<i64>>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "settings/AdminSettings.ts",
        rename = "AdminSettings"
    )
)]
pub struct AdminSettings {
    #[cfg_attr(feature = "type-export", ts(type = "Array<number>"))]
    pub mastery_bands: Vec<i64>,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub community_min_sample: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub free_daily_questions: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub free_daily_coach_turns: i64,
    #[cfg_attr(feature = "type-export", ts(type = "Array<number>"))]
    pub retest_intervals_days: Vec<i64>,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub offline_lease_days: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub max_reviews_per_day: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub max_new_cards_per_day: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub competition_difficulty_points: [i64; 3],
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "settings/AdminSettingsResponse.ts",
        rename = "AdminSettingsResponse"
    )
)]
pub struct AdminSettingsResponse {
    pub settings: AdminSettings,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "settings/AdminSettingsUpdateResponse.ts",
        rename = "AdminSettingsUpdateResponse"
    )
)]
pub struct AdminSettingsUpdateResponse {
    pub updated: Vec<String>,
}

fn valid_mastery_bands(values: &[i32]) -> bool {
    values.len() == 2
        && (0..=3000).contains(&values[0])
        && values[0] < values[1]
        && values[1] <= 3000
}

pub fn valid_retest_intervals(values: &[i64]) -> bool {
    !values.is_empty()
        && values.len() <= 20
        && values.iter().all(|days| (1..=3650).contains(days))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

pub fn effective_retest_intervals(values: Vec<i64>) -> Vec<i64> {
    if valid_retest_intervals(&values) {
        values
    } else {
        DEFAULT_RETEST_INTERVAL_DAYS.to_vec()
    }
}

fn valid_competition_difficulty_points(values: &[i64]) -> bool {
    values.len() == 3
        && values.iter().all(|points| (1..=1000).contains(points))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

pub fn effective_competition_difficulty_points(values: Vec<i64>) -> [i64; 3] {
    if valid_competition_difficulty_points(&values) {
        [values[0], values[1], values[2]]
    } else {
        DEFAULT_COMPETITION_DIFFICULTY_POINTS
    }
}

fn validate_settings(req: &SettingsReq) -> ApiResult<()> {
    if req
        .mastery_bands
        .as_deref()
        .is_some_and(|bands| !valid_mastery_bands(bands))
    {
        return Err(ApiError::unprocessable(
            "invalid_mastery_bands",
            "mastery_bands must contain two increasing values from 0 to 3000",
        ));
    }
    if req
        .community_min_sample
        .is_some_and(|value| !(1..=1_000_000).contains(&value))
    {
        return Err(ApiError::unprocessable(
            "invalid_community_min_sample",
            "community_min_sample must be between 1 and 1000000",
        ));
    }
    if req
        .free_daily_questions
        .is_some_and(|value| !(0..=5000).contains(&value))
    {
        return Err(ApiError::unprocessable(
            "invalid_free_daily_questions",
            "free_daily_questions must be between 0 and 5000",
        ));
    }
    if req
        .free_daily_coach_turns
        .is_some_and(|value| !(0..=1000).contains(&value))
    {
        return Err(ApiError::unprocessable(
            "invalid_free_daily_coach_turns",
            "free_daily_coach_turns must be between 0 and 1000",
        ));
    }
    if req
        .offline_lease_days
        .is_some_and(|value| !(1..=30).contains(&value))
    {
        return Err(ApiError::unprocessable(
            "invalid_offline_lease_days",
            "offline_lease_days must be between 1 and 30",
        ));
    }
    if req
        .max_reviews_per_day
        .is_some_and(|value| !(0..=5000).contains(&value))
    {
        return Err(ApiError::unprocessable(
            "invalid_max_reviews_per_day",
            "max_reviews_per_day must be between 0 and 5000",
        ));
    }
    if req
        .max_new_cards_per_day
        .is_some_and(|value| !(0..=1000).contains(&value))
    {
        return Err(ApiError::unprocessable(
            "invalid_max_new_cards_per_day",
            "max_new_cards_per_day must be between 0 and 1000",
        ));
    }
    if req
        .competition_difficulty_points
        .as_deref()
        .is_some_and(|values| !valid_competition_difficulty_points(values))
    {
        return Err(ApiError::unprocessable(
            "invalid_competition_difficulty_points",
            "competition_difficulty_points must contain three increasing values from 1 to 1000",
        ));
    }
    if req
        .retest_intervals_days
        .as_deref()
        .is_some_and(|days| !valid_retest_intervals(days))
    {
        return Err(ApiError::unprocessable(
            "invalid_retest_intervals",
            "retest_intervals_days must be 1-20 increasing values from 1 to 3650",
        ));
    }
    Ok(())
}

fn stored_i64(settings: &Map<String, Value>, key: &str, default: i64) -> i64 {
    settings.get(key).and_then(Value::as_i64).unwrap_or(default)
}

fn bounded_i64(value: i64, default: i64, minimum: i64, maximum: i64) -> i64 {
    if (minimum..=maximum).contains(&value) {
        value
    } else {
        default
    }
}

pub fn i64_list_or_default(value: Option<&Value>, default: &[i64]) -> Vec<i64> {
    value
        .and_then(Value::as_array)
        .and_then(|values| values.iter().map(Value::as_i64).collect::<Option<Vec<_>>>())
        .unwrap_or_else(|| default.to_vec())
}

fn stored_i64_list(settings: &Map<String, Value>, key: &str, default: &[i64]) -> Vec<i64> {
    i64_list_or_default(settings.get(key), default)
}

pub async fn current_i64(pool: &sqlx::PgPool, key: &str, default: i64) -> ApiResult<i64> {
    let value = sqlx::query_scalar::<_, Value>("SELECT value FROM app_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(value.and_then(|value| value.as_i64()).unwrap_or(default))
}

pub async fn current_bounded_i64(
    pool: &sqlx::PgPool,
    key: &str,
    default: i64,
    minimum: i64,
    maximum: i64,
) -> ApiResult<i64> {
    let value = current_i64(pool, key, default).await?;
    Ok(if (minimum..=maximum).contains(&value) {
        value
    } else {
        default
    })
}

pub async fn current_i64_list(
    pool: &sqlx::PgPool,
    key: &str,
    default: &[i64],
) -> ApiResult<Vec<i64>> {
    let value = sqlx::query_scalar::<_, Value>("SELECT value FROM app_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(i64_list_or_default(value.as_ref(), default))
}

pub async fn current_competition_difficulty_points(pool: &sqlx::PgPool) -> ApiResult<[i64; 3]> {
    Ok(effective_competition_difficulty_points(
        current_i64_list(
            pool,
            "competition_difficulty_points",
            &DEFAULT_COMPETITION_DIFFICULTY_POINTS,
        )
        .await?,
    ))
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<SettingsReq>,
) -> ApiResult<Json<AdminSettingsUpdateResponse>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(&user, provided)?;
    validate_settings(&req)?;

    let mut pairs: Vec<(&str, Value)> = Vec::new();
    if let Some(value) = req.mastery_bands {
        pairs.push(("mastery_bands", json!(value)));
    }
    if let Some(value) = req.community_min_sample {
        pairs.push(("community_min_sample", json!(value)));
    }
    if let Some(value) = req.free_daily_questions {
        pairs.push(("free_daily_questions", json!(value)));
    }
    if let Some(value) = req.free_daily_coach_turns {
        pairs.push(("free_daily_coach_turns", json!(value)));
    }
    if let Some(value) = req.retest_intervals_days {
        pairs.push(("retest_intervals_days", json!(value)));
    }
    if let Some(value) = req.offline_lease_days {
        pairs.push(("offline_lease_days", json!(value)));
    }
    if let Some(value) = req.max_reviews_per_day {
        pairs.push(("max_reviews_per_day", json!(value)));
    }
    if let Some(value) = req.max_new_cards_per_day {
        pairs.push(("max_new_cards_per_day", json!(value)));
    }
    if let Some(value) = req.competition_difficulty_points {
        pairs.push(("competition_difficulty_points", json!(value)));
    }
    if pairs.is_empty() {
        return Err(ApiError::unprocessable(
            "empty_settings_update",
            "provide at least one supported setting",
        ));
    }

    let keys = pairs
        .iter()
        .map(|(key, _)| (*key).to_string())
        .collect::<Vec<_>>();
    let mut tx = state.pool.begin().await?;
    // Keep the before-values and audit receipt consistent across concurrent admins.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('medical-os-admin-settings', 0))")
        .execute(&mut *tx)
        .await?;
    let rows = sqlx::query("SELECT key, value FROM app_settings WHERE key = ANY($1) FOR UPDATE")
        .bind(&keys)
        .fetch_all(&mut *tx)
        .await?;
    let mut old_settings = rows
        .into_iter()
        .map(|row| (row.get::<String, _>("key"), row.get::<Value, _>("value")))
        .collect::<Map<String, Value>>();
    for (key, default) in [
        ("offline_lease_days", DEFAULT_OFFLINE_LEASE_DAYS),
        ("max_reviews_per_day", DEFAULT_MAX_REVIEWS_PER_DAY),
        ("max_new_cards_per_day", DEFAULT_MAX_NEW_CARDS_PER_DAY),
    ] {
        if pairs.iter().any(|(updated, _)| *updated == key) {
            old_settings
                .entry(key.to_string())
                .or_insert_with(|| json!(default));
        }
    }
    if pairs
        .iter()
        .any(|(updated, _)| *updated == "competition_difficulty_points")
    {
        old_settings
            .entry("competition_difficulty_points".to_string())
            .or_insert_with(|| json!(DEFAULT_COMPETITION_DIFFICULTY_POINTS));
    }
    let new_settings = pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect::<Map<String, Value>>();

    for (key, value) in &pairs {
        sqlx::query(
            "INSERT INTO app_settings (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = now()",
        )
        .bind(*key)
        .bind(value.clone())
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO audit_events (id, actor, action, entity, entity_id, old_value, new_value)
         VALUES ($1, $2, 'settings_updated', 'app_settings', $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(user.user_id)
    .bind(Option::<Uuid>::None)
    .bind(Value::Object(old_settings))
    .bind(Value::Object(new_settings))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(AdminSettingsUpdateResponse { updated: keys }))
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<AdminSettingsResponse>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(&user, provided)?;
    let rows = sqlx::query("SELECT key, value FROM app_settings")
        .fetch_all(&state.pool)
        .await?;
    let stored = rows
        .into_iter()
        .map(|row| (row.get::<String, _>("key"), row.get::<Value, _>("value")))
        .collect::<Map<String, Value>>();

    let mut mastery_bands = stored_i64_list(&stored, "mastery_bands", &[1400, 1600]);
    if mastery_bands.len() != 2
        || !(0..=3000).contains(&mastery_bands[0])
        || mastery_bands[0] >= mastery_bands[1]
        || mastery_bands[1] > 3000
    {
        mastery_bands = vec![1400, 1600];
    }
    let mut retest_intervals = stored_i64_list(
        &stored,
        "retest_intervals_days",
        &DEFAULT_RETEST_INTERVAL_DAYS,
    );
    if !valid_retest_intervals(&retest_intervals) {
        retest_intervals = DEFAULT_RETEST_INTERVAL_DAYS.to_vec();
    }
    let competition_difficulty_points = effective_competition_difficulty_points(stored_i64_list(
        &stored,
        "competition_difficulty_points",
        &DEFAULT_COMPETITION_DIFFICULTY_POINTS,
    ));
    Ok(Json(AdminSettingsResponse {
        settings: AdminSettings {
            mastery_bands,
            community_min_sample: bounded_i64(
                stored_i64(&stored, "community_min_sample", state.community_min_sample),
                state.community_min_sample,
                1,
                1_000_000,
            ),
            free_daily_questions: bounded_i64(
                stored_i64(&stored, "free_daily_questions", state.free_daily_questions),
                state.free_daily_questions,
                0,
                5000,
            ),
            free_daily_coach_turns: bounded_i64(
                stored_i64(
                    &stored,
                    "free_daily_coach_turns",
                    state.free_daily_coach_turns,
                ),
                state.free_daily_coach_turns,
                0,
                1000,
            ),
            retest_intervals_days: retest_intervals,
            offline_lease_days: bounded_i64(
                stored_i64(&stored, "offline_lease_days", DEFAULT_OFFLINE_LEASE_DAYS),
                DEFAULT_OFFLINE_LEASE_DAYS,
                1,
                30,
            ),
            max_reviews_per_day: bounded_i64(
                stored_i64(&stored, "max_reviews_per_day", DEFAULT_MAX_REVIEWS_PER_DAY),
                DEFAULT_MAX_REVIEWS_PER_DAY,
                0,
                5000,
            ),
            max_new_cards_per_day: bounded_i64(
                stored_i64(
                    &stored,
                    "max_new_cards_per_day",
                    DEFAULT_MAX_NEW_CARDS_PER_DAY,
                ),
                DEFAULT_MAX_NEW_CARDS_PER_DAY,
                0,
                1000,
            ),
            competition_difficulty_points,
        },
    }))
}

/// Learner-safe config: only non-sensitive runtime settings.
pub async fn public_settings(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<Value>> {
    let free_daily_questions = current_bounded_i64(
        &state.pool,
        "free_daily_questions",
        state.free_daily_questions,
        0,
        5000,
    )
    .await?;
    Ok(Json(
        json!({ "free_daily_questions": free_daily_questions }),
    ))
}
