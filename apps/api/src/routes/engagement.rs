//! ENG-01 (daily goal/streak/QOTD), ENG-02 (XP/achievements), COMP-01/02
//! (competitions on the shared scoring crate), INST-04 (curriculum coverage),
//! ADMIN-06 settings.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Datelike;
use competition_scoring::{rank, AnswerRecord, Difficulty, Entry, ScoringConfig};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgConnection, Row};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::seed::QuestionOption;
use crate::state::AppState;

// ---- ENG-01: daily goal, streak with freezes, question of the day -----------

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/EngagementDailyGoal.ts",
        rename = "EngagementDailyGoal"
    )
)]
pub struct EngagementDailyGoal {
    enabled: bool,
    #[cfg_attr(feature = "type-export", ts(type = "\"questions\" | \"minutes\""))]
    mode: String,
    #[cfg_attr(feature = "type-export", ts(type = "\"questions\" | \"minutes\""))]
    unit: String,
    target: i32,
    answered_today: i32,
    minutes_today: i64,
    met: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/EngagementStreak.ts",
        rename = "EngagementStreak"
    )
)]
pub struct EngagementStreak {
    enabled: bool,
    count: i32,
    freezes: i32,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/EngagementOption.ts",
        rename = "EngagementOption"
    )
)]
pub struct EngagementOption {
    text: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/EngagementCommunitySplit.ts",
        rename = "EngagementCommunitySplit"
    )
)]
pub struct EngagementCommunitySplit {
    chosen_index: i32,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    count: i64,
}

#[derive(Default, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/EngagementQotd.ts",
        rename = "EngagementQotd"
    )
)]
pub struct EngagementQotd {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "type-export",
        ts(type = "string", optional = nullable)
    )]
    exam_id: Option<Option<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    needs_exam_selection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    answered: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(type = "string", optional))]
    question_version_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    vignette: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    options: Option<Vec<EngagementOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    community_split: Option<Vec<EngagementCommunitySplit>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(type = "number", optional))]
    community_total: Option<i64>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "engagement/Engagement.ts", rename = "Engagement")
)]
pub struct Engagement {
    enabled: bool,
    available_minutes: i32,
    daily_goal: EngagementDailyGoal,
    streak: EngagementStreak,
    qotd: EngagementQotd,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/EngagementSettingsResponse.ts",
        rename = "EngagementSettingsResponse"
    )
)]
pub struct EngagementSettingsResponse {
    daily_goal_questions: i32,
    #[cfg_attr(feature = "type-export", ts(type = "\"questions\" | \"minutes\""))]
    daily_goal_mode: String,
    available_minutes: i32,
    daily_goal_enabled: bool,
    streak_enabled: bool,
    qotd_enabled: bool,
    freezes: i32,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    qotd_exam_id: Option<Uuid>,
}

#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/QotdAnswerRequest.ts",
        rename = "QotdAnswerRequest"
    )
)]
#[derive(Deserialize)]
pub struct QotdAnswerReq {
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub question_version_id: Uuid,
    pub chosen_index: i32,
    #[serde(default)]
    #[cfg_attr(
        feature = "type-export",
        ts(type = "number", optional = nullable)
    )]
    pub elapsed_ms: Option<i64>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/QotdAnswerResponse.ts",
        rename = "QotdAnswerResponse"
    )
)]
pub struct QotdAnswerResponse {
    correct: bool,
    correct_index: i32,
    community_split: Vec<EngagementCommunitySplit>,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    community_total: i64,
}

/// Ensure a settings row exists (defaults: goal on, 20 questions, streak on,
/// QOTD on, zero freezes).
async fn ensure_engagement(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    sqlx::query!(
        r#"INSERT INTO engagement_settings (user_id)
           VALUES ($1)
           ON CONFLICT (user_id) DO NOTHING"#,
        user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}

/// Global kill switch: OPS-05-style feature flag `engagement_mechanics`.
/// Missing flag means enabled (default on).
async fn engagement_global_enabled(state: &AppState) -> ApiResult<bool> {
    let row = sqlx::query!(r#"SELECT value FROM feature_flags WHERE key = 'engagement_mechanics'"#)
        .fetch_optional(&state.pool)
        .await?;
    Ok(match row {
        Some(r) => r.value.as_bool().unwrap_or(true),
        None => true,
    })
}

async fn recorded_answer_time_ms(state: &AppState, user_id: Uuid) -> ApiResult<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        r#"SELECT COALESCE((
               SELECT SUM(elapsed_ms)::BIGINT FROM attempts
               WHERE user_id = $1 AND created_at::date = CURRENT_DATE
                 AND chosen_index IS NOT NULL
           ), 0) + COALESCE((
               SELECT SUM(elapsed_ms)::BIGINT FROM qotd_answers
               WHERE user_id = $1 AND day = CURRENT_DATE
           ), 0)"#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?)
}

/// Roll today's attempt count into engagement_days and advance the streak
/// once the goal is met. Freezes bridge a one-day gap (max 2 held).
pub async fn record_daily_progress(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    if !engagement_global_enabled(state).await? {
        return Ok(());
    }
    ensure_engagement(state, user_id).await?;
    let settings = sqlx::query!(
        r#"SELECT daily_goal_questions, daily_goal_mode, daily_available_minutes,
                  daily_goal_enabled, streak_enabled, freeze_bank
           FROM engagement_settings WHERE user_id = $1"#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?;
    if !settings.daily_goal_enabled {
        return Ok(());
    }
    // Answered attempts + QOTD each count as one answered question.
    let answered: i64 = sqlx::query!(
        r#"SELECT (SELECT COUNT(*) FROM attempts
                   WHERE user_id = $1
                     AND created_at::date = CURRENT_DATE
                     AND chosen_index IS NOT NULL)
              + (SELECT COUNT(*) FROM qotd_answers
                   WHERE user_id = $1 AND day = CURRENT_DATE) AS "n!""#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    let elapsed_ms = recorded_answer_time_ms(state, user_id).await?;
    let met = if settings.daily_goal_mode == "minutes" {
        elapsed_ms >= i64::from(settings.daily_available_minutes) * 60_000
    } else {
        answered >= i64::from(settings.daily_goal_questions)
    };
    let today = sqlx::query!(
        r#"SELECT goal_met FROM engagement_days
           WHERE user_id = $1 AND day = CURRENT_DATE"#,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    if met && settings.streak_enabled {
        let already_met = today.as_ref().map(|d| d.goal_met).unwrap_or(false);
        if already_met {
            sqlx::query!(
                r#"UPDATE engagement_days
                   SET questions_answered = $2
                   WHERE user_id = $1 AND day = CURRENT_DATE"#,
                user_id,
                answered as i32
            )
            .execute(&state.pool)
            .await?;
        } else {
            // First time today's goal is met. A freeze covers one missed day;
            // multiple held freezes can bridge the same number of missed days.
            let previous = sqlx::query!(
                r#"SELECT streak_count, (CURRENT_DATE - day - 1) AS "gap_days!"
                   FROM engagement_days
                   WHERE user_id = $1 AND day < CURRENT_DATE AND goal_met = true
                   ORDER BY day DESC
                   LIMIT 1"#,
                user_id
            )
            .fetch_optional(&state.pool)
            .await?;
            let streak = if let Some(prev) = previous {
                if prev.gap_days == 0 {
                    prev.streak_count + 1
                } else if prev.gap_days > 0 && prev.gap_days <= settings.freeze_bank {
                    let used = prev.gap_days;
                    sqlx::query!(
                        r#"UPDATE engagement_settings
                           SET freeze_bank = freeze_bank - $2, updated_at = now()
                           WHERE user_id = $1"#,
                        user_id,
                        used
                    )
                    .execute(&state.pool)
                    .await?;
                    prev.streak_count + 1
                } else {
                    1
                }
            } else {
                1
            };
            if streak > 0 && streak % 7 == 0 {
                sqlx::query!(
                    r#"UPDATE engagement_settings
                       SET freeze_bank = LEAST(freeze_bank + 1, 2), updated_at = now()
                       WHERE user_id = $1"#,
                    user_id
                )
                .execute(&state.pool)
                .await?;
            }
            sqlx::query!(
                r#"INSERT INTO engagement_days (user_id, day, questions_answered, goal_met, streak_count)
                   VALUES ($1, CURRENT_DATE, $2, true, $3)
                   ON CONFLICT (user_id, day) DO UPDATE
                     SET questions_answered = EXCLUDED.questions_answered,
                         goal_met = true,
                         streak_count = EXCLUDED.streak_count"#,
                user_id,
                answered as i32,
                streak
            )
            .execute(&state.pool)
            .await?;
        }
    } else {
        sqlx::query!(
            r#"INSERT INTO engagement_days (user_id, day, questions_answered, goal_met, streak_count)
               VALUES ($1, CURRENT_DATE, $2, $3, 0)
               ON CONFLICT (user_id, day) DO UPDATE
                 SET questions_answered = EXCLUDED.questions_answered,
                     goal_met = EXCLUDED.goal_met OR engagement_days.goal_met"#,
            user_id,
            answered as i32,
            met
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(())
}

pub async fn engagement_status(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<Engagement>> {
    let global = engagement_global_enabled(&state).await?;
    ensure_engagement(&state, user.user_id).await?;
    record_daily_progress(&state, user.user_id).await?;
    let s = sqlx::query!(
        r#"SELECT daily_goal_questions, daily_goal_mode, daily_available_minutes,
                  daily_goal_enabled, streak_enabled, qotd_enabled, freeze_bank, qotd_exam_id
           FROM engagement_settings WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    let day = sqlx::query!(
        r#"SELECT questions_answered, streak_count FROM engagement_days
           WHERE user_id = $1 AND day = CURRENT_DATE"#,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let answered_today = day.as_ref().map(|d| d.questions_answered).unwrap_or(0);
    let streak = day.as_ref().map(|d| d.streak_count).unwrap_or(0);
    let elapsed_ms = recorded_answer_time_ms(&state, user.user_id).await?;
    let goal_met = if s.daily_goal_mode == "minutes" {
        elapsed_ms >= i64::from(s.daily_available_minutes) * 60_000
    } else {
        i64::from(answered_today) >= i64::from(s.daily_goal_questions)
    };
    let (goal_target, goal_unit) = if s.daily_goal_mode == "minutes" {
        (s.daily_available_minutes, "minutes")
    } else {
        (s.daily_goal_questions, "questions")
    };

    let qotd = if global && s.qotd_enabled {
        qotd_payload(&state, user.user_id, s.qotd_exam_id).await?
    } else {
        EngagementQotd::default()
    };

    Ok(Json(Engagement {
        enabled: global,
        available_minutes: s.daily_available_minutes,
        daily_goal: EngagementDailyGoal {
            enabled: s.daily_goal_enabled && global,
            mode: s.daily_goal_mode,
            unit: goal_unit.to_owned(),
            target: goal_target,
            answered_today,
            minutes_today: elapsed_ms / 60_000,
            met: goal_met,
        },
        streak: EngagementStreak {
            enabled: s.streak_enabled && global,
            count: streak,
            freezes: s.freeze_bank,
        },
        qotd,
    }))
}

pub async fn qotd(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<EngagementQotd>> {
    if !engagement_global_enabled(&state).await? {
        return Ok(Json(EngagementQotd::default()));
    }
    ensure_engagement(&state, user.user_id).await?;
    let settings = sqlx::query!(
        r#"SELECT qotd_enabled, qotd_exam_id
           FROM engagement_settings WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    if !settings.qotd_enabled {
        return Ok(Json(EngagementQotd::default()));
    }
    Ok(Json(
        qotd_payload(&state, user.user_id, settings.qotd_exam_id).await?,
    ))
}

async fn qotd_question_is_eligible(
    conn: &mut PgConnection,
    exam_id: Uuid,
    question_version_id: Uuid,
) -> ApiResult<bool> {
    Ok(sqlx::query_scalar::<_, bool>(
        r#"SELECT EXISTS (
               SELECT 1
               FROM question_versions qv
               JOIN curriculum_nodes chapter ON chapter.id = qv.chapter_id
               WHERE qv.id = $1
                 AND qv.status = 'published'
                 AND chapter.exam_id = $2
                 AND chapter.kind = 'chapter'
                 AND NOT EXISTS (
                     SELECT 1 FROM reserved_questions rq
                     WHERE rq.question_version_id = qv.id)
           )"#,
    )
    .bind(question_version_id)
    .bind(exam_id)
    .fetch_one(&mut *conn)
    .await?)
}

async fn selected_qotd_id(conn: &mut PgConnection, exam_id: Uuid) -> ApiResult<Option<Uuid>> {
    if let Some(existing) = sqlx::query!(
        "SELECT question_version_id FROM qotd_daily_questions WHERE exam_id = $1 AND day = CURRENT_DATE",
        exam_id
    )
    .fetch_optional(&mut *conn)
    .await?
    {
        return Ok(qotd_question_is_eligible(conn, exam_id, existing.question_version_id)
            .await?
            .then_some(existing.question_version_id));
    }

    // The unique (exam, day) key chooses the winner if a pool change races the
    // first request; every later read observes that same persisted version.
    sqlx::query!(
        r#"WITH eligible AS (
               SELECT qv.id
               FROM question_versions qv
               JOIN curriculum_nodes chapter ON chapter.id = qv.chapter_id
               WHERE chapter.exam_id = $1
                 AND chapter.kind = 'chapter'
                 AND qv.status = 'published'
                 AND NOT EXISTS (
                     SELECT 1 FROM reserved_questions rq
                     WHERE rq.question_version_id = qv.id)
               ORDER BY qv.id
           ), picked AS (
               SELECT id FROM eligible
               ORDER BY id
               OFFSET (
                   SELECT CASE WHEN COUNT(*) = 0 THEN 0
                               ELSE ABS(hashtext($1::text || CURRENT_DATE::text)::bigint) % COUNT(*)
                          END
                   FROM eligible
               )
               LIMIT 1
           )
           INSERT INTO qotd_daily_questions (exam_id, day, question_version_id)
           SELECT $1, CURRENT_DATE, id FROM picked
           ON CONFLICT (exam_id, day) DO NOTHING"#,
        exam_id
    )
    .execute(&mut *conn)
    .await?;

    let selected = sqlx::query!(
        "SELECT question_version_id FROM qotd_daily_questions WHERE exam_id = $1 AND day = CURRENT_DATE",
        exam_id
    )
    .fetch_optional(&mut *conn)
    .await?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    Ok(
        qotd_question_is_eligible(conn, exam_id, selected.question_version_id)
            .await?
            .then_some(selected.question_version_id),
    )
}

/// Return the exam's stable daily pick. Answer state remains learner-specific.
async fn qotd_payload(
    state: &AppState,
    user_id: Uuid,
    exam_id: Option<Uuid>,
) -> ApiResult<EngagementQotd> {
    let existing = sqlx::query!(
        r#"SELECT a.question_version_id, a.chosen_index
           FROM qotd_answers a
           WHERE a.user_id = $1 AND a.day = CURRENT_DATE"#,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    if let Some(a) = existing {
        let split = sqlx::query!(
            r#"SELECT chosen_index, COUNT(*) AS "n!"
               FROM qotd_answers
               WHERE question_version_id = $1 AND day = CURRENT_DATE
               GROUP BY chosen_index ORDER BY chosen_index"#,
            a.question_version_id
        )
        .fetch_all(&state.pool)
        .await?;
        let total: i64 = split.iter().map(|r| r.n).sum();
        return Ok(EngagementQotd {
            enabled: true,
            exam_id: Some(exam_id),
            needs_exam_selection: Some(false),
            answered: Some(true),
            available: Some(false),
            community_split: Some(
                split
                    .iter()
                    .map(|r| EngagementCommunitySplit {
                        chosen_index: r.chosen_index,
                        count: r.n,
                    })
                    .collect(),
            ),
            community_total: Some(total),
            ..EngagementQotd::default()
        });
    }
    let Some(exam_id) = exam_id else {
        return Ok(EngagementQotd {
            enabled: true,
            exam_id: Some(None),
            needs_exam_selection: Some(true),
            answered: Some(false),
            available: Some(false),
            ..EngagementQotd::default()
        });
    };
    let mut conn = state.pool.acquire().await?;
    let Some(question_id) = selected_qotd_id(&mut conn, exam_id).await? else {
        return Ok(EngagementQotd {
            enabled: true,
            exam_id: Some(Some(exam_id)),
            needs_exam_selection: Some(false),
            answered: Some(false),
            available: Some(false),
            ..EngagementQotd::default()
        });
    };
    let q = sqlx::query!(
        r#"SELECT qv.id, qv.vignette, qv.options
           FROM question_versions qv
           JOIN curriculum_nodes chapter ON chapter.id = qv.chapter_id
           WHERE qv.id = $1 AND qv.status = 'published'
             AND chapter.exam_id = $2 AND chapter.kind = 'chapter'
             AND NOT EXISTS (
                 SELECT 1 FROM reserved_questions rq
                 WHERE rq.question_version_id = qv.id)"#,
        question_id,
        exam_id
    )
    .fetch_optional(&mut *conn)
    .await?;
    let Some(q) = q else {
        return Ok(EngagementQotd {
            enabled: true,
            exam_id: Some(Some(exam_id)),
            needs_exam_selection: Some(false),
            answered: Some(false),
            available: Some(false),
            ..EngagementQotd::default()
        });
    };
    let options: Vec<QuestionOption> =
        serde_json::from_value(q.options.clone()).unwrap_or_default();
    Ok(EngagementQotd {
        enabled: true,
        exam_id: Some(Some(exam_id)),
        needs_exam_selection: Some(false),
        answered: Some(false),
        available: Some(true),
        question_version_id: Some(q.id),
        vignette: Some(q.vignette),
        options: Some(
            options
                .into_iter()
                .map(|option| EngagementOption { text: option.text })
                .collect(),
        ),
        // correct_index withheld until answered (no cheating via the payload).
        ..EngagementQotd::default()
    })
}

pub async fn answer_qotd(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<QotdAnswerReq>,
) -> ApiResult<Json<QotdAnswerResponse>> {
    if !engagement_global_enabled(&state).await? {
        return Err(ApiError::forbidden(
            "engagement_disabled",
            "engagement mechanics are disabled",
        ));
    }
    ensure_engagement(&state, user.user_id).await?;
    let mut tx = state.pool.begin().await?;
    let settings = sqlx::query!(
        r#"SELECT qotd_enabled, qotd_exam_id
           FROM engagement_settings WHERE user_id = $1 FOR UPDATE"#,
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if !settings.qotd_enabled {
        return Err(ApiError::forbidden(
            "qotd_disabled",
            "question of the day is disabled",
        ));
    }
    let exam_id = settings
        .qotd_exam_id
        .ok_or_else(|| ApiError::not_found("qotd_unavailable"))?;
    let selected = selected_qotd_id(&mut tx, exam_id)
        .await?
        .ok_or_else(|| ApiError::not_found("qotd_unavailable"))?;
    if req.question_version_id != selected {
        tx.commit().await?;
        return Err(ApiError::unprocessable(
            "qotd_mismatch",
            "question_version_id is not today's question of the day",
        ));
    }
    let q = sqlx::query!(
        r#"SELECT qv.correct_index, jsonb_array_length(qv.options) AS "n!"
           FROM question_versions qv
           JOIN curriculum_nodes chapter ON chapter.id = qv.chapter_id
           WHERE qv.id = $1 AND qv.status = 'published'
             AND chapter.exam_id = $2 AND chapter.kind = 'chapter'
             AND NOT EXISTS (
                 SELECT 1 FROM reserved_questions rq
                 WHERE rq.question_version_id = qv.id)"#,
        req.question_version_id,
        exam_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(q) = q else {
        tx.commit().await?;
        return Err(ApiError::not_found("question_not_found"));
    };
    if req.chosen_index < 0 || req.chosen_index >= q.n {
        tx.commit().await?;
        return Err(ApiError::unprocessable(
            "invalid_option",
            "chosen_index is out of range",
        ));
    }
    let elapsed_ms = req
        .elapsed_ms
        .filter(|ms| (0..=3_600_000).contains(ms))
        .unwrap_or(0);
    let inserted = sqlx::query!(
        r#"INSERT INTO qotd_answers (user_id, day, question_version_id, chosen_index, elapsed_ms)
           VALUES ($1, CURRENT_DATE, $2, $3, $4)
           ON CONFLICT (user_id, day) DO NOTHING"#,
        user.user_id,
        req.question_version_id,
        req.chosen_index,
        elapsed_ms
    )
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() == 0 {
        tx.commit().await?;
        return Err(ApiError::conflict(
            "already_answered",
            "today's question of the day is already answered",
        ));
    }
    tx.commit().await?;
    record_daily_progress(&state, user.user_id).await?;
    let split = sqlx::query!(
        r#"SELECT chosen_index, COUNT(*) AS "n!"
           FROM qotd_answers
           WHERE question_version_id = $1 AND day = CURRENT_DATE
           GROUP BY chosen_index ORDER BY chosen_index"#,
        req.question_version_id
    )
    .fetch_all(&state.pool)
    .await?;
    let total: i64 = split.iter().map(|r| r.n).sum();
    let correct = i32::from(q.correct_index) == req.chosen_index;
    Ok(Json(QotdAnswerResponse {
        correct,
        correct_index: i32::from(q.correct_index),
        community_split: split
            .iter()
            .map(|r| EngagementCommunitySplit {
                chosen_index: r.chosen_index,
                count: r.n,
            })
            .collect(),
        community_total: total,
    }))
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/EngagementSettings.ts",
        rename = "EngagementSettings"
    )
)]
pub struct EngagementSettingsReq {
    #[cfg_attr(
        feature = "type-export",
        ts(type = "number", optional = nullable)
    )]
    pub daily_goal_questions: Option<i32>,
    #[cfg_attr(
        feature = "type-export",
        ts(
            type = "\"questions\" | \"minutes\"",
            optional = nullable
        )
    )]
    pub daily_goal_mode: Option<String>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "number", optional = nullable)
    )]
    pub available_minutes: Option<i32>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "boolean", optional = nullable)
    )]
    pub daily_goal_enabled: Option<bool>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "boolean", optional = nullable)
    )]
    pub streak_enabled: Option<bool>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "boolean", optional = nullable)
    )]
    pub qotd_enabled: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    #[cfg_attr(
        feature = "type-export",
        ts(type = "string", optional = nullable)
    )]
    pub qotd_exam_id: Option<Option<Uuid>>,
}

pub async fn update_engagement_settings(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<EngagementSettingsReq>,
) -> ApiResult<Json<EngagementSettingsResponse>> {
    ensure_engagement(&state, user.user_id).await?;
    if req
        .daily_goal_questions
        .is_some_and(|t| !(1..=500).contains(&t))
    {
        return Err(ApiError::unprocessable(
            "invalid_daily_goal",
            "daily goal must be 1-500 questions",
        ));
    }
    if req
        .daily_goal_mode
        .as_deref()
        .is_some_and(|mode| !matches!(mode, "questions" | "minutes"))
    {
        return Err(ApiError::unprocessable(
            "invalid_daily_goal_mode",
            "daily goal mode must be questions or minutes",
        ));
    }
    if req
        .available_minutes
        .is_some_and(|minutes| !(5..=480).contains(&minutes))
    {
        return Err(ApiError::unprocessable(
            "invalid_daily_availability",
            "daily available minutes must be 5-480",
        ));
    }

    let qotd_exam_changed = req.qotd_exam_id.is_some();
    let qotd_exam_id = req.qotd_exam_id.flatten();
    let mut tx = state.pool.begin().await?;
    let current = sqlx::query!(
        r#"SELECT qotd_exam_id FROM engagement_settings
           WHERE user_id = $1 FOR UPDATE"#,
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if qotd_exam_changed {
        if let Some(exam_id) = qotd_exam_id {
            let exists =
                sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM exams WHERE id = $1)")
                    .bind(exam_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if !exists {
                return Err(ApiError::not_found("qotd_exam_not_found"));
            }
        }
        if current.qotd_exam_id != qotd_exam_id {
            let answered = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM qotd_answers WHERE user_id = $1 AND day = CURRENT_DATE)",
            )
            .bind(user.user_id)
            .fetch_one(&mut *tx)
            .await?;
            if answered {
                return Err(ApiError::conflict(
                    "qotd_exam_locked",
                    "the question of the day exam cannot change after today's answer",
                ));
            }
        }
    }
    sqlx::query!(
        r#"UPDATE engagement_settings SET
               daily_goal_questions = COALESCE($2, daily_goal_questions),
               daily_goal_mode = COALESCE($3, daily_goal_mode),
               daily_available_minutes = COALESCE($4, daily_available_minutes),
               daily_goal_enabled = COALESCE($5, daily_goal_enabled),
               streak_enabled = COALESCE($6, streak_enabled),
               qotd_enabled = COALESCE($7, qotd_enabled),
               qotd_exam_id = CASE WHEN $8 THEN $9 ELSE qotd_exam_id END,
               updated_at = now()
           WHERE user_id = $1"#,
        user.user_id,
        req.daily_goal_questions,
        req.daily_goal_mode.as_deref(),
        req.available_minutes,
        req.daily_goal_enabled,
        req.streak_enabled,
        req.qotd_enabled,
        qotd_exam_changed,
        qotd_exam_id
    )
    .execute(&mut *tx)
    .await?;
    let s = sqlx::query!(
        r#"SELECT daily_goal_questions, daily_goal_mode, daily_available_minutes,
                  daily_goal_enabled, streak_enabled, qotd_enabled, freeze_bank, qotd_exam_id
           FROM engagement_settings WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(EngagementSettingsResponse {
        daily_goal_questions: s.daily_goal_questions,
        daily_goal_mode: s.daily_goal_mode,
        available_minutes: s.daily_available_minutes,
        daily_goal_enabled: s.daily_goal_enabled,
        streak_enabled: s.streak_enabled,
        qotd_enabled: s.qotd_enabled,
        freezes: s.freeze_bank,
        qotd_exam_id: s.qotd_exam_id,
    }))
}

// ---- ENG-02: XP + achievements -----------------------------------------------

pub async fn my_xp(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let total = sqlx::query!(
        r#"SELECT COALESCE(SUM(points), 0) AS "total!" FROM xp_ledger WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .total;
    let achievements = sqlx::query!(
        "SELECT code, earned_at FROM achievements WHERE user_id = $1 ORDER BY earned_at",
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "xp_total": total,
        "achievements": achievements.iter().map(|a| json!({
            "code": a.code, "earned_at": a.earned_at,
        })).collect::<Vec<_>>(),
    })))
}

// ---- INST-04: curriculum coverage for institution members --------------------

/// Per-chapter coverage across an institution's learners: how many members
/// attempted, how many questions exist vs attempted per chapter. Aggregates
/// only — no individual rows (§18.3).
pub async fn institution_coverage(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(inst_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let member = sqlx::query!(
        "SELECT 1 AS one FROM institution_members
         WHERE institution_id = $1 AND user_id = $2",
        inst_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("institution_not_found"))?;
    let _ = member;

    let rows = sqlx::query!(
        r#"SELECT c.name AS chapter_name,
                  COALESCE(COUNT(DISTINCT a.user_id), 0) AS "learners!",
                  COALESCE(COUNT(*), 0) AS "attempts!"
           FROM institution_members im
           JOIN cohorts coh ON coh.institution_id = im.institution_id
           JOIN cohort_members cm ON cm.cohort_id = coh.id AND cm.user_id = im.user_id
           LEFT JOIN attempts a ON a.user_id = im.user_id
           LEFT JOIN question_versions qv ON qv.id = a.question_version_id
           LEFT JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE im.institution_id = $1
           GROUP BY c.name ORDER BY c.name"#,
        inst_id
    )
    .fetch_all(&state.pool)
    .await?;
    let coverage: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "chapter": r.chapter_name,
                "learners": r.learners,
                "attempts": r.attempts,
            })
        })
        .collect();
    Ok(Json(json!({ "coverage": coverage })))
}

/// Absent → None (setting untouched); explicit null → Some(None) (cleared);
/// a value → Some(Some(v)). Plain `Option<Option<T>>` cannot tell null from
/// absent because serde short-circuits the outer Option on null.
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

// ---- COMP-01/02: competitions on the shared scoring crate --------------------

#[derive(Deserialize)]
pub struct CreateCompetitionReq {
    pub title: String,
    pub exam_id: Uuid,
    pub question_ids: Vec<Uuid>,
    pub starts_at: chrono::DateTime<chrono::Utc>,
    pub ends_at: chrono::DateTime<chrono::Utc>,
    /// COMP-01: one_off | daily | weekly | monthly | live.
    pub cadence: Option<String>,
    /// Optional number of questions chosen from the recurring series pool.
    pub question_count: Option<usize>,
}

fn next_competition_start(
    cadence: &str,
    start: chrono::DateTime<chrono::Utc>,
    monthly_anchor_day: u32,
    monthly_anchor_end: bool,
) -> Option<chrono::DateTime<chrono::Utc>> {
    match cadence {
        "daily" => start.checked_add_signed(chrono::Duration::days(1)),
        "weekly" => start.checked_add_signed(chrono::Duration::days(7)),
        "monthly" => {
            let next_month = start
                .date_naive()
                .with_day(1)?
                .checked_add_months(chrono::Months::new(1))?;
            let last_day = next_month
                .checked_add_months(chrono::Months::new(1))?
                .pred_opt()?
                .day();
            let target_day = if monthly_anchor_end {
                last_day
            } else {
                monthly_anchor_day.min(last_day)
            };
            Some(
                chrono::NaiveDateTime::new(next_month.with_day(target_day)?, start.time())
                    .and_utc(),
            )
        }
        _ => None,
    }
}

fn choose_competition_questions(
    pool: &[Uuid],
    previous: &HashSet<Uuid>,
    count: usize,
) -> Vec<Uuid> {
    let mut fresh: Vec<Uuid> = pool
        .iter()
        .copied()
        .filter(|question_id| !previous.contains(question_id))
        .collect();
    fresh.shuffle(&mut rand::thread_rng());
    if fresh.len() < count {
        let selected: HashSet<Uuid> = fresh.iter().copied().collect();
        let mut fill: Vec<Uuid> = pool
            .iter()
            .copied()
            .filter(|question_id| !selected.contains(question_id))
            .collect();
        fill.shuffle(&mut rand::thread_rng());
        fresh.extend(fill);
    }
    fresh.truncate(count);
    fresh
}

pub async fn create_competition(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateCompetitionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    if req.title.trim().is_empty() || req.question_ids.len() < 3 || req.question_ids.len() > 500 {
        return Err(ApiError::unprocessable(
            "invalid_competition",
            "title required and question pool must contain 3-500 questions",
        ));
    }
    if req.question_ids.iter().collect::<HashSet<_>>().len() != req.question_ids.len() {
        return Err(ApiError::unprocessable(
            "invalid_competition_questions",
            "competition questions must be unique",
        ));
    }
    let valid_question_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM question_versions qv
         JOIN curriculum_nodes node ON node.id = qv.chapter_id
         WHERE qv.id = ANY($1) AND node.exam_id = $2 AND qv.status = 'published'",
    )
    .bind(&req.question_ids)
    .bind(req.exam_id)
    .fetch_one(&state.pool)
    .await?;
    if valid_question_count != req.question_ids.len() as i64 {
        return Err(ApiError::unprocessable(
            "invalid_competition_questions",
            "all competition questions must be published and belong to its exam",
        ));
    }
    if req.ends_at <= req.starts_at {
        return Err(ApiError::unprocessable(
            "invalid_window",
            "competition must end after it starts",
        ));
    }
    let cadence = req.cadence.unwrap_or_else(|| "one_off".into());
    if !matches!(
        cadence.as_str(),
        "one_off" | "daily" | "weekly" | "monthly" | "live"
    ) {
        return Err(ApiError::unprocessable(
            "invalid_cadence",
            "cadence must be one_off, daily, weekly, monthly, or live",
        ));
    }
    let recurring = matches!(cadence.as_str(), "daily" | "weekly" | "monthly");
    let question_count = req.question_count.unwrap_or(req.question_ids.len());
    if (recurring && (question_count < 3 || question_count > req.question_ids.len()))
        || (!recurring && req.question_count.is_some())
    {
        return Err(ApiError::unprocessable(
            "invalid_competition_question_count",
            "recurring question_count must be at least 3 and no larger than its pool; one-off and live events use their full question list",
        ));
    }
    let duration = req.ends_at.signed_duration_since(req.starts_at);
    let monthly_anchor_day = req.starts_at.day();
    let monthly_anchor_end = req
        .starts_at
        .date_naive()
        .succ_opt()
        .map(|next_day| next_day.month() != req.starts_at.month())
        .unwrap_or(true);
    let recurrence = if recurring {
        next_competition_start(
            &cadence,
            req.starts_at,
            monthly_anchor_day,
            monthly_anchor_end,
        )
        .ok_or_else(ApiError::internal)?
    } else {
        req.starts_at
    };
    if recurring && duration > recurrence.signed_duration_since(req.starts_at) {
        return Err(ApiError::unprocessable(
            "invalid_competition_window",
            "a recurring event must finish before its next occurrence",
        ));
    }
    if recurring && duration.num_seconds() < 1 {
        return Err(ApiError::unprocessable(
            "invalid_competition_window",
            "a recurring event must last at least one second",
        ));
    }
    if cadence == "monthly" && duration > chrono::Duration::days(28) {
        return Err(ApiError::unprocessable(
            "invalid_competition_window",
            "monthly events must last no longer than 28 days",
        ));
    }
    let id = Uuid::new_v4();
    let series_id = recurring.then(Uuid::new_v4);
    let difficulty_points =
        crate::routes::settings::current_competition_difficulty_points(&state.pool).await?;
    let first_questions = if recurring {
        choose_competition_questions(&req.question_ids, &HashSet::new(), question_count)
    } else {
        req.question_ids.clone()
    };
    let mut tx = state.pool.begin().await?;
    if let Some(series_id) = series_id {
        sqlx::query(
            "INSERT INTO competition_series
               (id, title, exam_id, question_pool, question_count, cadence,
                monthly_anchor_day, monthly_anchor_end, next_start_at, duration_seconds,
                created_by, difficulty_points)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(series_id)
        .bind(req.title.trim())
        .bind(req.exam_id)
        .bind(json!(req.question_ids))
        .bind(i32::try_from(question_count).map_err(|_| ApiError::internal())?)
        .bind(&cadence)
        .bind(i16::try_from(monthly_anchor_day).map_err(|_| ApiError::internal())?)
        .bind(monthly_anchor_end)
        .bind(recurrence)
        .bind(duration.num_seconds())
        .bind(user.user_id)
        .bind(json!(difficulty_points))
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO competitions
         (id, title, exam_id, question_ids, starts_at, ends_at, created_by, cadence, difficulty_points, series_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(id)
    .bind(req.title.trim())
    .bind(req.exam_id)
    .bind(serde_json::to_value(&first_questions).map_err(|_| ApiError::internal())?)
    .bind(req.starts_at)
    .bind(req.ends_at)
    .bind(user.user_id)
    .bind(cadence)
    .bind(json!(difficulty_points))
    .bind(series_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        json!({ "competition_id": id, "series_id": series_id }),
    ))
}

/// Materialize a bounded horizon under per-series row locks. One list request
/// processes at most ten series and three occurrences per series.
async fn materialize_competition_series(state: &AppState) -> ApiResult<()> {
    let mut tx = state.pool.begin().await?;
    let series_rows = sqlx::query(
        "SELECT id, title, exam_id, question_pool, question_count, cadence,
                monthly_anchor_day, monthly_anchor_end, next_start_at, duration_seconds,
                created_by, difficulty_points
         FROM competition_series
         WHERE active = true AND next_start_at <= now() + interval '31 days'
         ORDER BY next_start_at
         LIMIT 10
         FOR UPDATE SKIP LOCKED",
    )
    .fetch_all(&mut *tx)
    .await?;
    let now = chrono::Utc::now();
    for series in series_rows {
        let series_id: Uuid = series.try_get("id")?;
        let title: String = series.try_get("title")?;
        let exam_id: Uuid = series.try_get("exam_id")?;
        let question_pool: Vec<Uuid> = serde_json::from_value(series.try_get("question_pool")?)
            .map_err(|_| ApiError::internal())?;
        let requested_count: i32 = series.try_get("question_count")?;
        let cadence: String = series.try_get("cadence")?;
        let monthly_anchor_day: i16 = series.try_get("monthly_anchor_day")?;
        let monthly_anchor_end: bool = series.try_get("monthly_anchor_end")?;
        let mut next_start: chrono::DateTime<chrono::Utc> = series.try_get("next_start_at")?;
        let duration_seconds: i64 = series.try_get("duration_seconds")?;
        let created_by: Option<Uuid> = series.try_get("created_by")?;
        let difficulty_points: serde_json::Value = series.try_get("difficulty_points")?;
        let available: Vec<Uuid> = sqlx::query_scalar(
            "SELECT qv.id
             FROM question_versions qv
             JOIN curriculum_nodes node ON node.id = qv.chapter_id
             WHERE qv.id = ANY($1) AND node.exam_id = $2 AND qv.status = 'published'
             ORDER BY array_position($1::uuid[], qv.id)",
        )
        .bind(&question_pool)
        .bind(exam_id)
        .fetch_all(&mut *tx)
        .await?;
        let horizon = next_competition_start(
            &cadence,
            now,
            u32::try_from(monthly_anchor_day).map_err(|_| ApiError::internal())?,
            monthly_anchor_end,
        )
        .unwrap_or(now);
        for _ in 0..3 {
            if next_start > horizon {
                break;
            }
            if available.len() >= 3 {
                let previous: Option<serde_json::Value> = sqlx::query_scalar(
                    "SELECT question_ids FROM competitions
                     WHERE series_id = $1 ORDER BY starts_at DESC LIMIT 1",
                )
                .bind(series_id)
                .fetch_optional(&mut *tx)
                .await?;
                let previous: HashSet<Uuid> = previous
                    .map(serde_json::from_value::<Vec<Uuid>>)
                    .transpose()
                    .map_err(|_| ApiError::internal())?
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                let count = usize::try_from(requested_count)
                    .map_err(|_| ApiError::internal())?
                    .min(available.len());
                let selected = choose_competition_questions(&available, &previous, count);
                if selected.len() >= 3 {
                    let event_id = Uuid::new_v4();
                    let end_at = next_start
                        .checked_add_signed(chrono::Duration::seconds(duration_seconds))
                        .ok_or_else(ApiError::internal)?;
                    sqlx::query(
                        "INSERT INTO competitions
                           (id, title, exam_id, question_ids, starts_at, ends_at, created_by,
                            cadence, difficulty_points, series_id)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                         ON CONFLICT (series_id, starts_at) WHERE series_id IS NOT NULL DO NOTHING",
                    )
                    .bind(event_id)
                    .bind(&title)
                    .bind(exam_id)
                    .bind(json!(selected))
                    .bind(next_start)
                    .bind(end_at)
                    .bind(created_by)
                    .bind(&cadence)
                    .bind(&difficulty_points)
                    .bind(series_id)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            let Some(following) = next_competition_start(
                &cadence,
                next_start,
                u32::try_from(monthly_anchor_day).map_err(|_| ApiError::internal())?,
                monthly_anchor_end,
            ) else {
                sqlx::query("UPDATE competition_series SET active = false WHERE id = $1")
                    .bind(series_id)
                    .execute(&mut *tx)
                    .await?;
                break;
            };
            next_start = following;
        }
        sqlx::query("UPDATE competition_series SET next_start_at = $2 WHERE id = $1")
            .bind(series_id)
            .bind(next_start)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/CompetitionSummary.ts",
        rename = "CompetitionSummary"
    )
)]
pub struct CompetitionSummary {
    pub competition_id: Uuid,
    pub title: String,
    pub exam_id: Uuid,
    pub exam: String,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"one_off\" | \"daily\" | \"weekly\" | \"monthly\" | \"live\"")
    )]
    pub cadence: String,
    pub series_id: Option<Uuid>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub starts_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub ends_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub entered: bool,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"in_progress\" | \"submitted\" | null")
    )]
    pub attempt_status: Option<String>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/CompetitionListResponse.ts",
        rename = "CompetitionListResponse"
    )
)]
pub struct CompetitionListResponse {
    pub competitions: Vec<CompetitionSummary>,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/StartCompetitionEntryRequest.ts",
        rename = "StartCompetitionEntryRequest"
    )
)]
#[serde(deny_unknown_fields)]
pub struct StartCompetitionEntryReq {
    pub handle: String,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/AnswerCompetitionQuestionRequest.ts",
        rename = "AnswerCompetitionQuestionRequest"
    )
)]
#[serde(deny_unknown_fields)]
pub struct AnswerCompetitionQuestionReq {
    pub question_version_id: Uuid,
    pub chosen_index: i64,
    pub idempotency_key: Uuid,
    #[serde(default)]
    pub elapsed_ms: Option<i64>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/CompetitionQuestionOption.ts",
        rename = "CompetitionQuestionOption"
    )
)]
pub struct CompetitionQuestionOption {
    pub text: String,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/CompetitionQuestion.ts",
        rename = "CompetitionQuestion"
    )
)]
pub struct CompetitionQuestion {
    pub question_version_id: Uuid,
    pub question_number: usize,
    pub total_questions: usize,
    pub vignette: String,
    pub lead_in: String,
    pub options: Vec<CompetitionQuestionOption>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/CompetitionInProgressResponse.ts",
        rename = "CompetitionInProgressResponse"
    )
)]
pub struct CompetitionInProgressResponse {
    pub attempt_id: Uuid,
    #[cfg_attr(feature = "type-export", ts(type = "false"))]
    pub submitted: bool,
    pub question: CompetitionQuestion,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/CompetitionSubmittedResponse.ts",
        rename = "CompetitionSubmittedResponse"
    )
)]
pub struct CompetitionSubmittedResponse {
    pub attempt_id: Uuid,
    #[cfg_attr(feature = "type-export", ts(type = "true"))]
    pub submitted: bool,
    pub entry_id: Uuid,
    pub score: f64,
    pub questions: i64,
    pub total_time_ms: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/CompetitionAttemptStep.ts",
        rename = "CompetitionAttemptStep"
    )
)]
pub enum CompetitionAttemptStep {
    InProgress(CompetitionInProgressResponse),
    Submitted(CompetitionSubmittedResponse),
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/CompetitionLeaderboardEntry.ts",
        rename = "CompetitionLeaderboardEntry"
    )
)]
pub struct CompetitionLeaderboardEntry {
    pub rank: i64,
    pub handle: String,
    pub score: f32,
    pub accuracy: f64,
    pub questions_attempted: i64,
    pub average_response_time_ms: f64,
    pub total_time_ms: i64,
    pub is_me: bool,
    pub prize_eligible: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "engagement/CompetitionLeaderboardResponse.ts",
        rename = "CompetitionLeaderboardResponse"
    )
)]
pub struct CompetitionLeaderboardResponse {
    pub prize_reviewed: bool,
    pub status: String,
    pub entries: Vec<CompetitionLeaderboardEntry>,
}

pub async fn list_competitions(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<CompetitionListResponse>> {
    materialize_competition_series(&state).await?;
    let rows = sqlx::query(
        r#"SELECT c.id, c.title, c.exam_id, e.name AS exam, c.cadence, c.series_id,
                  c.starts_at, c.ends_at, c.status,
                  EXISTS (
                      SELECT 1 FROM competition_entries ce
                      WHERE ce.competition_id = c.id AND ce.user_id = $1
                  ) AS entered,
                  CASE WHEN EXISTS (
                      SELECT 1 FROM competition_entries ce
                      WHERE ce.competition_id = c.id AND ce.user_id = $1
                  ) THEN 'submitted' ELSE attempt.status END AS attempt_status
           FROM competitions c
           JOIN exams e ON e.id = c.exam_id
           LEFT JOIN competition_attempts attempt
             ON attempt.competition_id = c.id AND attempt.user_id = $1
           WHERE c.ends_at >= now() - interval '30 days'
              OR EXISTS (SELECT 1 FROM competition_entries ce
                         WHERE ce.competition_id = c.id AND ce.user_id = $1)
           ORDER BY (c.ends_at < now()), c.starts_at ASC
           LIMIT 100"#,
    )
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;
    let competitions: Vec<CompetitionSummary> = rows
        .iter()
        .map(|row| {
            Ok(CompetitionSummary {
                competition_id: row.try_get("id")?,
                title: row.try_get("title")?,
                exam_id: row.try_get("exam_id")?,
                exam: row.try_get("exam")?,
                cadence: row.try_get("cadence")?,
                series_id: row.try_get("series_id")?,
                starts_at: row.try_get("starts_at")?,
                ends_at: row.try_get("ends_at")?,
                status: row.try_get("status")?,
                entered: row.try_get("entered")?,
                attempt_status: row.try_get("attempt_status")?,
            })
        })
        .collect::<Result<_, sqlx::Error>>()?;
    Ok(Json(CompetitionListResponse { competitions }))
}

#[derive(Deserialize, serde::Serialize)]
struct ScoredCompetitionAnswer {
    question_version_id: Uuid,
    chosen_index: i64,
    elapsed_ms: i64,
    correct: bool,
    difficulty: String,
    #[serde(default)]
    idempotency_key: Option<Uuid>,
    #[serde(default)]
    request_body: Option<serde_json::Value>,
    #[serde(default)]
    response: Option<serde_json::Value>,
}

fn competition_difficulty(value: &str) -> ApiResult<Difficulty> {
    match value {
        "easy" => Ok(Difficulty::Easy),
        "medium" => Ok(Difficulty::Medium),
        "hard" => Ok(Difficulty::Hard),
        _ => Err(ApiError::internal()),
    }
}

async fn present_competition_question(
    connection: &mut sqlx::PgConnection,
    question_version_id: Uuid,
    option_order: &[usize],
    position: usize,
    total_questions: usize,
) -> ApiResult<CompetitionQuestion> {
    let row = sqlx::query(
        "SELECT vignette, lead_in, options, status
         FROM question_versions WHERE id = $1",
    )
    .bind(question_version_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    let status: String = row.try_get("status")?;
    if status != "published" {
        return Err(ApiError::conflict(
            "question_unavailable",
            "this competition question is no longer available",
        ));
    }
    let options: Vec<QuestionOption> =
        serde_json::from_value(row.try_get("options")?).map_err(|_| ApiError::internal())?;
    let unique_options: HashSet<usize> = option_order.iter().copied().collect();
    if options.len() < 2
        || options.len() != option_order.len()
        || unique_options.len() != options.len()
        || option_order.iter().any(|index| *index >= options.len())
    {
        return Err(ApiError::internal());
    }
    let displayed_options: Vec<CompetitionQuestionOption> = option_order
        .iter()
        .map(|index| CompetitionQuestionOption {
            text: options[*index].text.clone(),
        })
        .collect();
    Ok(CompetitionQuestion {
        question_version_id,
        question_number: position + 1,
        total_questions,
        vignette: row.try_get("vignette")?,
        lead_in: row.try_get("lead_in")?,
        options: displayed_options,
    })
}

/// Start or resume one server-timed competition attempt.
pub async fn start_competition_entry(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(comp_id): Path<Uuid>,
    Json(req): Json<StartCompetitionEntryReq>,
) -> ApiResult<Json<CompetitionAttemptStep>> {
    let handle = req.handle.trim();
    if handle.is_empty() || handle.len() > 40 {
        return Err(ApiError::unprocessable(
            "invalid_handle",
            "handle must be 1-40 characters",
        ));
    }
    // COMMUNITY-03: competition presence is part of the community — it
    // requires the opt-in profile, and the handle must be the learner's own.
    let profile = sqlx::query!(
        "SELECT handle FROM community_profiles WHERE user_id = $1",
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let Some(p) = profile else {
        return Err(ApiError::forbidden(
            "not_opted_in",
            "create a community profile to enter competitions",
        ));
    };
    if !p.handle.eq_ignore_ascii_case(handle) {
        return Err(ApiError::forbidden(
            "handle_mismatch",
            "entries must use your own community handle",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let competition = sqlx::query(
        "SELECT starts_at, ends_at, question_ids, status
         FROM competitions WHERE id = $1 FOR SHARE",
    )
    .bind(comp_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("competition_not_found"))?;
    let starts_at: chrono::DateTime<chrono::Utc> = competition.try_get("starts_at")?;
    let ends_at: chrono::DateTime<chrono::Utc> = competition.try_get("ends_at")?;
    let competition_status: String = competition.try_get("status")?;
    let now = chrono::Utc::now();
    if competition_status == "closed" {
        return Err(ApiError::conflict(
            "already_closed",
            "competition is closed",
        ));
    }
    if now < starts_at {
        return Err(ApiError::conflict(
            "not_started",
            "competition has not started",
        ));
    }
    if now > ends_at {
        return Err(ApiError::conflict(
            "already_closed",
            "competition window has closed",
        ));
    }
    let question_ids: Vec<Uuid> = serde_json::from_value(competition.try_get("question_ids")?)
        .map_err(|_| ApiError::internal())?;
    if question_ids.len() < 3 {
        return Err(ApiError::internal());
    }
    let has_entry: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM competition_entries WHERE competition_id = $1 AND user_id = $2
        )",
    )
    .bind(comp_id)
    .bind(user.user_id)
    .fetch_one(&mut *tx)
    .await?;
    if has_entry {
        return Err(ApiError::conflict(
            "already_submitted",
            "one competition entry is allowed per learner",
        ));
    }

    let mut randomized_questions = question_ids;
    randomized_questions.shuffle(&mut rand::thread_rng());
    let first_question_id = randomized_questions[0];
    let option_count: i64 = sqlx::query_scalar(
        "SELECT jsonb_array_length(options)::BIGINT FROM question_versions
         WHERE id = $1 AND status = 'published'",
    )
    .bind(first_question_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::conflict("question_unavailable", "competition question unavailable")
    })?;
    let option_count = usize::try_from(option_count).map_err(|_| ApiError::internal())?;
    if option_count < 2 {
        return Err(ApiError::internal());
    }
    let mut first_option_order: Vec<usize> = (0..option_count).collect();
    first_option_order.shuffle(&mut rand::thread_rng());
    sqlx::query(
        "INSERT INTO competition_attempts
           (id, competition_id, user_id, handle, question_ids, option_order,
            question_started_at)
         VALUES ($1, $2, $3, $4, $5, $6, clock_timestamp())
         ON CONFLICT (competition_id, user_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(comp_id)
    .bind(user.user_id)
    .bind(&p.handle)
    .bind(json!(randomized_questions))
    .bind(json!(first_option_order))
    .execute(&mut *tx)
    .await?;
    let attempt = sqlx::query(
        "SELECT id, handle, question_ids, current_index, option_order, status
         FROM competition_attempts
         WHERE competition_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(comp_id)
    .bind(user.user_id)
    .fetch_one(&mut *tx)
    .await?;
    let status: String = attempt.try_get("status")?;
    if status != "in_progress" {
        return Err(ApiError::conflict(
            "already_submitted",
            "one competition entry is allowed per learner",
        ));
    }
    let attempt_id: Uuid = attempt.try_get("id")?;
    let saved_handle: String = attempt.try_get("handle")?;
    if saved_handle != p.handle {
        sqlx::query("UPDATE competition_attempts SET handle = $2 WHERE id = $1")
            .bind(attempt_id)
            .bind(&p.handle)
            .execute(&mut *tx)
            .await?;
    }
    let randomized_questions: Vec<Uuid> = serde_json::from_value(attempt.try_get("question_ids")?)
        .map_err(|_| ApiError::internal())?;
    let current_index: i32 = attempt.try_get("current_index")?;
    let position = usize::try_from(current_index).map_err(|_| ApiError::internal())?;
    let current_question_id = *randomized_questions
        .get(position)
        .ok_or_else(ApiError::internal)?;
    let option_order: Vec<usize> = serde_json::from_value(attempt.try_get("option_order")?)
        .map_err(|_| ApiError::internal())?;
    let question = present_competition_question(
        &mut tx,
        current_question_id,
        &option_order,
        position,
        randomized_questions.len(),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(CompetitionAttemptStep::InProgress(
        CompetitionInProgressResponse {
            attempt_id,
            submitted: false,
            question,
        },
    )))
}

/// Submit one displayed option; elapsed time and scoring are server-owned.
pub async fn answer_competition_question(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(comp_id): Path<Uuid>,
    Json(req): Json<AnswerCompetitionQuestionReq>,
) -> ApiResult<Json<CompetitionAttemptStep>> {
    if req.elapsed_ms.is_some() {
        return Err(ApiError::bad_request(
            "client_timing_not_allowed",
            "elapsed_ms is measured by the server",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let attempt = sqlx::query(
        "SELECT attempt.id, attempt.handle, attempt.question_ids, attempt.current_index,
                attempt.option_order, attempt.answers, attempt.question_started_at,
                attempt.status, competition.starts_at, competition.ends_at,
                competition.difficulty_points, competition.status AS competition_status
         FROM competition_attempts attempt
         JOIN competitions competition ON competition.id = attempt.competition_id
         WHERE attempt.competition_id = $1 AND attempt.user_id = $2
         FOR UPDATE OF attempt",
    )
    .bind(comp_id)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("competition_attempt_not_found"))?;
    let attempt_id: Uuid = attempt.try_get("id")?;
    let request_body = json!({
        "question_version_id": req.question_version_id,
        "chosen_index": req.chosen_index,
    });
    let mut answers: Vec<ScoredCompetitionAnswer> =
        serde_json::from_value(attempt.try_get("answers")?).map_err(|_| ApiError::internal())?;
    if let Some(previous) = answers
        .iter()
        .find(|answer| answer.idempotency_key == Some(req.idempotency_key))
    {
        if previous.request_body.as_ref() != Some(&request_body) {
            return Err(ApiError::conflict(
                "idempotency_key_reused",
                "this answer key was already used for a different response",
            ));
        }
        return previous
            .response
            .clone()
            .ok_or_else(ApiError::internal)
            .and_then(decode_competition_attempt_step);
    }
    let attempt_status: String = attempt.try_get("status")?;
    if attempt_status == "submitted" {
        return answers
            .last()
            .and_then(|answer| answer.response.clone())
            .ok_or_else(ApiError::internal)
            .and_then(decode_competition_attempt_step);
    }
    let starts_at: chrono::DateTime<chrono::Utc> = attempt.try_get("starts_at")?;
    let ends_at: chrono::DateTime<chrono::Utc> = attempt.try_get("ends_at")?;
    let competition_status: String = attempt.try_get("competition_status")?;
    let now = chrono::Utc::now();
    if competition_status == "closed" {
        return Err(ApiError::conflict(
            "already_closed",
            "competition is closed",
        ));
    }
    if now < starts_at {
        return Err(ApiError::conflict(
            "not_started",
            "competition has not started",
        ));
    }
    if now > ends_at {
        return Err(ApiError::conflict(
            "already_closed",
            "competition window has closed",
        ));
    }
    let randomized_questions: Vec<Uuid> = serde_json::from_value(attempt.try_get("question_ids")?)
        .map_err(|_| ApiError::internal())?;
    let current_index: i32 = attempt.try_get("current_index")?;
    let position = usize::try_from(current_index).map_err(|_| ApiError::internal())?;
    if randomized_questions.get(position) != Some(&req.question_version_id) {
        return Err(ApiError::unprocessable(
            "invalid_answer_order",
            "answer the current competition question",
        ));
    }
    let option_order: Vec<usize> = serde_json::from_value(attempt.try_get("option_order")?)
        .map_err(|_| ApiError::internal())?;
    let question = sqlx::query(
        "SELECT correct_index, difficulty, options, status
         FROM question_versions WHERE id = $1",
    )
    .bind(req.question_version_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    let question_status: String = question.try_get("status")?;
    if question_status != "published" {
        return Err(ApiError::conflict(
            "question_unavailable",
            "this competition question is no longer available",
        ));
    }
    let options: Vec<QuestionOption> =
        serde_json::from_value(question.try_get("options")?).map_err(|_| ApiError::internal())?;
    let unique_options: HashSet<usize> = option_order.iter().copied().collect();
    if options.len() != option_order.len()
        || unique_options.len() != options.len()
        || option_order.iter().any(|index| *index >= options.len())
    {
        return Err(ApiError::internal());
    }
    if req.chosen_index < 0 || req.chosen_index >= option_order.len() as i64 {
        return Err(ApiError::unprocessable(
            "invalid_answer_choice",
            "chosen_index must identify a displayed option",
        ));
    }
    let chosen_position = usize::try_from(req.chosen_index).map_err(|_| ApiError::internal())?;
    let chosen_index =
        i64::try_from(option_order[chosen_position]).map_err(|_| ApiError::internal())?;
    let correct_index: i16 = question.try_get("correct_index")?;
    let correct_index = usize::try_from(correct_index).map_err(|_| ApiError::internal())?;
    if correct_index >= options.len() {
        return Err(ApiError::internal());
    }
    let difficulty: String = question.try_get("difficulty")?;
    let correct = usize::try_from(chosen_index).map_err(|_| ApiError::internal())? == correct_index;
    let question_started_at: chrono::DateTime<chrono::Utc> =
        attempt.try_get("question_started_at")?;
    let _ = competition_difficulty(&difficulty)?;
    let elapsed_ms: i64 = sqlx::query_scalar(
        "SELECT GREATEST(
            0,
            FLOOR(EXTRACT(EPOCH FROM (clock_timestamp() - $1)) * 1000)
        )::BIGINT",
    )
    .bind(question_started_at)
    .fetch_one(&mut *tx)
    .await?;
    answers.push(ScoredCompetitionAnswer {
        question_version_id: req.question_version_id,
        chosen_index,
        elapsed_ms,
        correct,
        difficulty,
        idempotency_key: Some(req.idempotency_key),
        request_body: Some(request_body.clone()),
        response: None,
    });
    let next_position = position.checked_add(1).ok_or_else(ApiError::internal)?;
    let is_final = next_position == randomized_questions.len();
    let response = if is_final {
        let point_values: Vec<i64> =
            serde_json::from_value(attempt.try_get("difficulty_points")?).unwrap_or_default();
        let points = crate::routes::settings::effective_competition_difficulty_points(point_values);
        let config = ScoringConfig {
            easy_points: points[0],
            medium_points: points[1],
            hard_points: points[2],
            ..ScoringConfig::default()
        };
        let answer_records: Vec<AnswerRecord> = answers
            .iter()
            .map(|answer| {
                Ok(AnswerRecord {
                    difficulty: competition_difficulty(&answer.difficulty)?,
                    correct: answer.correct,
                    elapsed_ms: answer.elapsed_ms,
                })
            })
            .collect::<ApiResult<_>>()?;
        let total_time_ms = answer_records
            .iter()
            .try_fold(0_i64, |total, answer| total.checked_add(answer.elapsed_ms))
            .ok_or_else(|| ApiError::unprocessable("invalid_answer_time", "total time overflow"))?;
        let window_ms = ends_at
            .signed_duration_since(starts_at)
            .num_milliseconds()
            .max(0);
        if total_time_ms > window_ms {
            return Err(ApiError::unprocessable(
                "invalid_answer_time",
                "combined answer time must fit within the competition window",
            ));
        }
        let score = rank(
            &config,
            &[Entry {
                participant_id: user.user_id.to_string(),
                answers: answer_records,
                total_time_ms,
                submitted_order: 0,
            }],
        )
        .first()
        .ok_or_else(ApiError::internal)?
        .score;
        let correct_count =
            i64::try_from(answers.iter().filter(|answer| answer.correct).count())
                .map_err(|_| ApiError::unprocessable("invalid_answer_count", "too many answers"))?;
        let attempted_count = i64::try_from(answers.len())
            .map_err(|_| ApiError::unprocessable("invalid_answer_count", "too many answers"))?;
        let average_response_time_ms = total_time_ms as f64 / attempted_count as f64;
        let stored_answers = answers
            .iter()
            .map(|answer| {
                json!({
                    "question_version_id": answer.question_version_id,
                    "chosen_index": answer.chosen_index,
                    "elapsed_ms": answer.elapsed_ms,
                })
            })
            .collect::<Vec<_>>();
        let entry_id = Uuid::new_v4();
        let handle: String = attempt.try_get("handle")?;
        let inserted = sqlx::query(
            "INSERT INTO competition_entries
               (id, competition_id, user_id, handle, answers, score, total_time_ms,
                submitted_order, correct_count, attempted_count, average_response_time_ms)
             VALUES ($1, $2, $3, $4, $5, $6, $7,
               (SELECT COALESCE(MAX(submitted_order), 0) + 1 FROM competition_entries
                WHERE competition_id = $2), $8, $9, $10)
             ON CONFLICT (competition_id, user_id) DO NOTHING",
        )
        .bind(entry_id)
        .bind(comp_id)
        .bind(user.user_id)
        .bind(handle)
        .bind(json!(stored_answers))
        .bind(score as f32)
        .bind(total_time_ms)
        .bind(correct_count)
        .bind(attempted_count)
        .bind(average_response_time_ms)
        .execute(&mut *tx)
        .await?;
        if inserted.rows_affected() == 0 {
            return Err(ApiError::conflict(
                "already_submitted",
                "one competition entry is allowed per learner",
            ));
        }
        let response = json!({
            "attempt_id": attempt_id,
            "submitted": true,
            "entry_id": entry_id,
            "score": score,
            "questions": attempted_count,
            "total_time_ms": total_time_ms,
        });
        answers.last_mut().ok_or_else(ApiError::internal)?.response = Some(response.clone());
        sqlx::query(
            "UPDATE competition_attempts
             SET answers = $2, status = 'submitted', submitted_at = clock_timestamp()
             WHERE id = $1",
        )
        .bind(attempt_id)
        .bind(json!(answers))
        .execute(&mut *tx)
        .await?;
        response
    } else {
        let next_question_id = randomized_questions[next_position];
        let next_option_count: i64 = sqlx::query_scalar(
            "SELECT jsonb_array_length(options)::BIGINT FROM question_versions
             WHERE id = $1 AND status = 'published'",
        )
        .bind(next_question_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            ApiError::conflict("question_unavailable", "competition question unavailable")
        })?;
        let next_option_count =
            usize::try_from(next_option_count).map_err(|_| ApiError::internal())?;
        let mut next_option_order: Vec<usize> = (0..next_option_count).collect();
        next_option_order.shuffle(&mut rand::thread_rng());
        let next_question = present_competition_question(
            &mut tx,
            next_question_id,
            &next_option_order,
            next_position,
            randomized_questions.len(),
        )
        .await?;
        let response = json!({
            "attempt_id": attempt_id,
            "submitted": false,
            "question": next_question,
        });
        answers.last_mut().ok_or_else(ApiError::internal)?.response = Some(response.clone());
        sqlx::query(
            "UPDATE competition_attempts
             SET current_index = $2, option_order = $3, answers = $4,
                 question_started_at = clock_timestamp()
             WHERE id = $1",
        )
        .bind(attempt_id)
        .bind(i32::try_from(next_position).map_err(|_| ApiError::internal())?)
        .bind(json!(next_option_order))
        .bind(json!(answers))
        .execute(&mut *tx)
        .await?;
        response
    };
    tx.commit().await?;
    decode_competition_attempt_step(response)
}

fn decode_competition_attempt_step(
    response: serde_json::Value,
) -> ApiResult<Json<CompetitionAttemptStep>> {
    serde_json::from_value(response)
        .map(Json)
        .map_err(|_| ApiError::internal())
}

/// Simple XP award called from the practice submit handler, with the
/// §17.2 milestone achievements (deterministic, never an optimization
/// target). Replaces the earlier per-difficulty duplicate.
pub async fn award_session_xp(
    state: &AppState,
    user_id: Uuid,
    correct_count: i64,
) -> ApiResult<()> {
    if correct_count == 0 {
        return Ok(());
    }
    let points = (correct_count * 10) as i32; // base XP per correct answer
    sqlx::query!(
        "INSERT INTO xp_ledger (id, user_id, points, reason)
         VALUES ($1, $2, $3, 'session_correct')",
        Uuid::new_v4(),
        user_id,
        points
    )
    .execute(&state.pool)
    .await?;
    let earned: i64 = sqlx::query!(
        r#"SELECT COALESCE(SUM(points), 0) AS "total!" FROM xp_ledger WHERE user_id = $1"#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?
    .total;
    for milestone in [100, 500] {
        if earned >= milestone {
            sqlx::query!(
                "INSERT INTO achievements (user_id, code) VALUES ($1, $2)
                 ON CONFLICT DO NOTHING",
                user_id,
                format!("xp_{milestone}")
            )
            .execute(&state.pool)
            .await?;
        }
    }
    Ok(())
}

/// GET /v1/me/weekly-recap — ENG-02: the learner's last 7 days from real
/// records only (XP earned, sessions submitted, accuracy, reviews done).
/// Empty weeks report zeros honestly; nothing is synthesized.
pub async fn weekly_recap(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let xp: i64 = sqlx::query!(
        r#"SELECT COALESCE(SUM(points), 0) AS "total!" FROM xp_ledger
           WHERE user_id = $1 AND created_at >= now() - interval '7 days'"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .total;
    let sessions = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM practice_sessions
           WHERE user_id = $1 AND status = 'submitted'
             AND submitted_at >= now() - interval '7 days'"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    let attempts = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!",
                  COALESCE(COUNT(*) FILTER (WHERE correct = TRUE), 0) AS "c!"
           FROM attempts
           WHERE user_id = $1 AND created_at >= now() - interval '7 days'"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    let reviews = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM review_events
           WHERE user_id = $1 AND reviewed_at >= now() - interval '7 days'"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    Ok(Json(json!({
        "days": 7,
        "xp": xp,
        "sessions_submitted": sessions,
        "questions_answered": attempts.n,
        "questions_correct": attempts.c,
        "reviews_done": reviews,
    })))
}

#[cfg(test)]
mod competition_schedule_tests {
    use super::next_competition_start;
    use chrono::{NaiveDate, TimeZone, Utc};

    #[test]
    fn monthly_series_preserve_day_or_end_of_month_anchor() {
        let jan_31 = Utc
            .with_ymd_and_hms(2024, 1, 31, 15, 30, 0)
            .single()
            .unwrap();
        let feb_29 = next_competition_start("monthly", jan_31, 31, true).unwrap();
        let mar_31 = next_competition_start("monthly", feb_29, 31, true).unwrap();
        assert_eq!(
            feb_29.date_naive(),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
        assert_eq!(
            mar_31.date_naive(),
            NaiveDate::from_ymd_opt(2024, 3, 31).unwrap()
        );
        assert_eq!(mar_31.time(), jan_31.time());

        let jan_30 = Utc
            .with_ymd_and_hms(2024, 1, 30, 15, 30, 0)
            .single()
            .unwrap();
        let feb_29 = next_competition_start("monthly", jan_30, 30, false).unwrap();
        let mar_30 = next_competition_start("monthly", feb_29, 30, false).unwrap();
        assert_eq!(
            feb_29.date_naive(),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
        assert_eq!(
            mar_30.date_naive(),
            NaiveDate::from_ymd_opt(2024, 3, 30).unwrap()
        );
    }
}
