//! ENG-01: optional, server-authoritative engagement mechanics derived from
//! existing goals, submitted study sessions, attempts, and notification time.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use chrono::{NaiveDate, NaiveTime};
use domain_contracts::ProtectedCommitment;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::routes::practice;
use crate::state::AppState;

#[derive(Clone, Serialize)]
pub struct EngagementPreferences {
    daily_goal_enabled: bool,
    streak_enabled: bool,
    qotd_enabled: bool,
    qotd_time: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateEngagementPreferences {
    daily_goal_enabled: bool,
    streak_enabled: bool,
    qotd_enabled: bool,
    qotd_time: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct StartQotdRequest {
    takeover: Option<bool>,
}

struct GoalVersion {
    daily_minutes: Option<i32>,
    protected_commitments: Vec<ProtectedCommitment>,
    local_date: NaiveDate,
}

#[derive(Serialize)]
struct DailyGoalView {
    enabled: bool,
    target_minutes: Option<i32>,
    completed_minutes: i64,
    met: bool,
}

#[derive(Serialize)]
struct StreakView {
    enabled: bool,
    length: i32,
    freezes_held: i32,
    goal_met_days: i32,
    next_freeze_in: Option<i32>,
}

async fn ensure_preferences(pool: &sqlx::PgPool, user_id: Uuid) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO engagement_preferences (user_id) VALUES ($1)
         ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn preferences(pool: &sqlx::PgPool, user_id: Uuid) -> ApiResult<EngagementPreferences> {
    ensure_preferences(pool, user_id).await?;
    let (daily_goal_enabled, streak_enabled, qotd_enabled, qotd_time) =
        sqlx::query_as::<_, (bool, bool, bool, Option<NaiveTime>)>(
            "SELECT daily_goal_enabled, streak_enabled, qotd_enabled, qotd_time
             FROM engagement_preferences WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(EngagementPreferences {
        daily_goal_enabled,
        streak_enabled,
        qotd_enabled,
        qotd_time: qotd_time.map(|time| time.format("%H:%M").to_string()),
    })
}

fn parse_qotd_time(value: Option<&str>) -> ApiResult<Option<NaiveTime>> {
    value
        .map(|value| {
            NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| {
                ApiError::unprocessable("invalid_qotd_time", "qotd_time must use HH:MM")
            })
        })
        .transpose()
}

async fn learner_clock(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> ApiResult<(String, NaiveDate, NaiveTime)> {
    sqlx::query_as::<_, (String, NaiveDate, NaiveTime)>(
        r#"WITH requested AS (
               SELECT COALESCE(
                   (SELECT timezone FROM notification_preferences WHERE user_id = $1),
                   'UTC'
               ) AS timezone
           ), resolved AS (
               SELECT CASE
                   WHEN EXISTS (
                       SELECT 1 FROM pg_timezone_names z WHERE z.name = requested.timezone
                   ) THEN requested.timezone
                   ELSE 'UTC'
               END AS timezone
               FROM requested
           )
           SELECT timezone,
                  (CURRENT_TIMESTAMP AT TIME ZONE timezone)::date,
                  (CURRENT_TIMESTAMP AT TIME ZONE timezone)::time
           FROM resolved"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

async fn goal_versions(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    timezone: &str,
) -> ApiResult<Vec<GoalVersion>> {
    let rows = sqlx::query_as::<_, (Option<i32>, serde_json::Value, NaiveDate)>(
        "SELECT daily_minutes, protected_commitments,
                (created_at AT TIME ZONE $2)::date AS local_date
         FROM learner_goal_versions
         WHERE user_id = $1
         ORDER BY created_at, version",
    )
    .bind(user_id)
    .bind(timezone)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|(daily_minutes, protected_commitments, local_date)| {
            let protected_commitments =
                serde_json::from_value(protected_commitments).map_err(|_| ApiError::internal())?;
            Ok(GoalVersion {
                daily_minutes,
                protected_commitments,
                local_date,
            })
        })
        .collect()
}

async fn submitted_minutes_by_day(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    timezone: &str,
) -> ApiResult<BTreeMap<NaiveDate, i64>> {
    let rows = sqlx::query_as::<_, (NaiveDate, i64)>(
        r#"SELECT (submitted_at AT TIME ZONE $2)::date AS local_date,
                  COALESCE(SUM(GREATEST(
                      0,
                      FLOOR(EXTRACT(EPOCH FROM (submitted_at - created_at)))::bigint
                  )), 0)::bigint AS seconds
           FROM practice_sessions
           WHERE user_id = $1
             AND status = 'submitted'
             AND submitted_at IS NOT NULL
           GROUP BY local_date
           ORDER BY local_date"#,
    )
    .bind(user_id)
    .bind(timezone)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(date, seconds)| (date, seconds / 60))
        .collect())
}

fn engagement_progress(
    versions: &[GoalVersion],
    minutes_by_day: &BTreeMap<NaiveDate, i64>,
    local_date: NaiveDate,
    prefs: &EngagementPreferences,
) -> (DailyGoalView, StreakView) {
    let latest_target = versions.last().and_then(|version| version.daily_minutes);
    let completed_minutes = minutes_by_day.get(&local_date).copied().unwrap_or(0);
    let today_met = latest_target.is_some_and(|target| completed_minutes >= i64::from(target));
    let daily_goal = DailyGoalView {
        enabled: prefs.daily_goal_enabled,
        target_minutes: latest_target,
        completed_minutes,
        met: today_met,
    };

    let Some(first_date) = versions.first().map(|version| version.local_date) else {
        return (
            daily_goal,
            StreakView {
                enabled: prefs.streak_enabled,
                length: 0,
                freezes_held: 0,
                goal_met_days: 0,
                next_freeze_in: Some(7),
            },
        );
    };

    let mut version_index = 0_usize;
    let mut active: Option<&GoalVersion> = None;
    let mut day = first_date;
    let mut streak = 0_i32;
    let mut freezes = 0_i32;
    let mut goal_met_days = 0_i32;
    while day <= local_date {
        while version_index < versions.len() && versions[version_index].local_date <= day {
            active = Some(&versions[version_index]);
            version_index += 1;
        }
        let Some(goal) = active else {
            day = match day.succ_opt() {
                Some(next) => next,
                None => break,
            };
            continue;
        };
        let Some(target) = goal.daily_minutes else {
            day = match day.succ_opt() {
                Some(next) => next,
                None => break,
            };
            continue;
        };
        let protected = goal
            .protected_commitments
            .iter()
            .any(|commitment| commitment.date == day);
        let completed = minutes_by_day.get(&day).copied().unwrap_or(0);
        if completed >= i64::from(target) {
            streak += 1;
            goal_met_days += 1;
            if goal_met_days % 7 == 0 {
                freezes = (freezes + 1).min(2);
            }
        } else if day < local_date && !protected {
            if freezes > 0 {
                freezes -= 1;
            } else {
                streak = 0;
            }
        }
        day = match day.succ_opt() {
            Some(next) => next,
            None => break,
        };
    }
    let next_freeze_in = (freezes < 2).then_some(7 - goal_met_days.rem_euclid(7));
    (
        daily_goal,
        StreakView {
            enabled: prefs.streak_enabled,
            length: streak,
            freezes_held: freezes,
            goal_met_days,
            next_freeze_in,
        },
    )
}

async fn qotd_question(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    exam_id: Uuid,
    local_date: NaiveDate,
) -> ApiResult<Option<(Uuid, i32)>> {
    sqlx::query_as::<_, (Uuid, i32)>(
        r#"SELECT qv.id, jsonb_array_length(qv.options)::int
           FROM question_versions qv
           JOIN curriculum_nodes chapter ON chapter.id = qv.chapter_id
           WHERE chapter.exam_id = $1
             AND qv.status = 'published'
             AND NOT EXISTS (
                 SELECT 1 FROM question_reports report
                 WHERE report.question_version_id = qv.id
                   AND report.status = 'quarantined'
             )
           ORDER BY md5(qv.id::text || ':' || $2::text || ':' || $3::text)
           LIMIT 1"#,
    )
    .bind(exam_id)
    .bind(user_id)
    .bind(local_date)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

async fn community_split(
    pool: &sqlx::PgPool,
    question_version_id: Uuid,
    option_count: i32,
) -> ApiResult<serde_json::Value> {
    let rows = sqlx::query_as::<_, (i16, i64)>(
        r#"WITH latest AS (
               SELECT DISTINCT ON (user_id) user_id, chosen_index
               FROM attempts
               WHERE question_version_id = $1 AND chosen_index IS NOT NULL
               ORDER BY user_id, created_at DESC, id DESC
           )
           SELECT chosen_index, COUNT(*)::bigint
           FROM latest
           GROUP BY chosen_index
           ORDER BY chosen_index"#,
    )
    .bind(question_version_id)
    .fetch_all(pool)
    .await?;
    let counts: BTreeMap<i16, i64> = rows.into_iter().collect();
    let total: i64 = counts.values().sum();
    let options: Vec<serde_json::Value> = (0..option_count)
        .map(|index| {
            let count = counts.get(&(index as i16)).copied().unwrap_or(0);
            serde_json::json!({
                "option_index": index,
                "count": count,
                "percentage": if total == 0 { 0 } else { count * 100 / total }
            })
        })
        .collect();
    Ok(serde_json::json!({
        "total_answers": total,
        "options": options
    }))
}

async fn qotd_views(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    local_date: NaiveDate,
    local_time: NaiveTime,
    prefs: &EngagementPreferences,
) -> ApiResult<Vec<serde_json::Value>> {
    if !prefs.qotd_enabled {
        return Ok(Vec::new());
    }
    let exams = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, code, name FROM exams ORDER BY name, id",
    )
    .fetch_all(pool)
    .await?;
    let due_time = prefs
        .qotd_time
        .as_deref()
        .and_then(|time| NaiveTime::parse_from_str(time, "%H:%M").ok());
    let mut out = Vec::new();
    for (exam_id, exam_code, exam_name) in exams {
        let Some((question_version_id, option_count)) =
            qotd_question(pool, user_id, exam_id, local_date).await?
        else {
            continue;
        };
        let answered = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1 FROM attempts
                WHERE user_id = $1 AND question_version_id = $2 AND chosen_index IS NOT NULL
             )",
        )
        .bind(user_id)
        .bind(question_version_id)
        .fetch_one(pool)
        .await?;
        let split = if answered {
            community_split(pool, question_version_id, option_count).await?
        } else {
            serde_json::Value::Null
        };
        out.push(serde_json::json!({
            "exam_id": exam_id,
            "exam_code": exam_code,
            "exam_name": exam_name,
            "question_version_id": question_version_id,
            "answered": answered,
            "due": due_time.is_some_and(|time| local_time >= time) && !answered,
            "community_split": split
        }));
    }
    Ok(out)
}

pub async fn get_engagement(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let prefs = preferences(&state.pool, user.user_id).await?;
    let (timezone, local_date, local_time) = learner_clock(&state.pool, user.user_id).await?;
    let versions = goal_versions(&state.pool, user.user_id, &timezone).await?;
    let minutes = submitted_minutes_by_day(&state.pool, user.user_id, &timezone).await?;
    let (daily_goal, streak) = engagement_progress(&versions, &minutes, local_date, &prefs);
    let qotd = qotd_views(&state.pool, user.user_id, local_date, local_time, &prefs).await?;
    Ok(Json(serde_json::json!({
        "timezone": timezone,
        "local_date": local_date,
        "preferences": prefs,
        "daily_goal": daily_goal,
        "streak": streak,
        "qotd": qotd
    })))
}

pub async fn update_preferences(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<UpdateEngagementPreferences>,
) -> ApiResult<Json<EngagementPreferences>> {
    let qotd_time = parse_qotd_time(req.qotd_time.as_deref())?;
    sqlx::query(
        "INSERT INTO engagement_preferences
            (user_id, daily_goal_enabled, streak_enabled, qotd_enabled, qotd_time, updated_at)
         VALUES ($1, $2, $3, $4, $5, now())
         ON CONFLICT (user_id) DO UPDATE SET
            daily_goal_enabled = EXCLUDED.daily_goal_enabled,
            streak_enabled = EXCLUDED.streak_enabled,
            qotd_enabled = EXCLUDED.qotd_enabled,
            qotd_time = EXCLUDED.qotd_time,
            updated_at = now()",
    )
    .bind(user.user_id)
    .bind(req.daily_goal_enabled)
    .bind(req.streak_enabled)
    .bind(req.qotd_enabled)
    .bind(qotd_time)
    .execute(&state.pool)
    .await?;
    Ok(Json(EngagementPreferences {
        daily_goal_enabled: req.daily_goal_enabled,
        streak_enabled: req.streak_enabled,
        qotd_enabled: req.qotd_enabled,
        qotd_time: qotd_time.map(|time| time.format("%H:%M").to_string()),
    }))
}

pub async fn start_qotd_session(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(exam_id): Path<Uuid>,
    Json(req): Json<StartQotdRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let prefs = preferences(&state.pool, user.user_id).await?;
    if !prefs.qotd_enabled {
        return Err(ApiError::conflict(
            "qotd_disabled",
            "question of the day is disabled",
        ));
    }
    let (_, local_date, _) = learner_clock(&state.pool, user.user_id).await?;
    let Some((question_version_id, _)) =
        qotd_question(&state.pool, user.user_id, exam_id, local_date).await?
    else {
        return Err(ApiError::not_found("qotd_not_available"));
    };
    let Json(mut response) = practice::create_single_question_session(
        &state,
        user.user_id,
        question_version_id,
        req.takeover.unwrap_or(false),
    )
    .await?;
    response["exam_id"] = serde_json::json!(exam_id);
    response["question_version_id"] = serde_json::json!(question_version_id);
    response["local_date"] = serde_json::json!(local_date);
    Ok(Json(response))
}
