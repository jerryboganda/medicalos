//! ENG-01 (daily goal/streak/QOTD), ENG-02 (XP/achievements), COMP-01/02
//! (competitions on the shared scoring crate), INST-04 (curriculum coverage),
//! ADMIN-06 settings.

use axum::extract::{Path, State};
use axum::Json;
use competition_scoring::{AnswerRecord, Difficulty, ScoringConfig};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ---- ENG-01: daily goal, streak with freezes, question of the day -----------

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

/// Roll today's attempt count into engagement_days and advance the streak
/// once the goal is met. Freezes bridge a one-day gap (max 2 held).
pub async fn record_daily_progress(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    if !engagement_global_enabled(state).await? {
        return Ok(());
    }
    ensure_engagement(state, user_id).await?;
    let settings = sqlx::query!(
        r#"SELECT daily_goal_questions, daily_goal_enabled, streak_enabled, freeze_bank
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
    let met = answered >= settings.daily_goal_questions as i64;
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

#[axum::debug_handler]
pub async fn engagement_status(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let global = engagement_global_enabled(&state).await?;
    ensure_engagement(&state, user.user_id).await?;
    record_daily_progress(&state, user.user_id).await?;
    let s = sqlx::query!(
        r#"SELECT daily_goal_questions, daily_goal_enabled, streak_enabled, qotd_enabled, freeze_bank
           FROM engagement_settings WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    let day = sqlx::query!(
        r#"SELECT questions_answered, goal_met, streak_count FROM engagement_days
           WHERE user_id = $1 AND day = CURRENT_DATE"#,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let answered_today = day.as_ref().map(|d| d.questions_answered).unwrap_or(0);
    let goal_met = day.as_ref().map(|d| d.goal_met).unwrap_or(false);
    let streak = day.as_ref().map(|d| d.streak_count).unwrap_or(0);

    let qotd = if global && s.qotd_enabled {
        qotd_payload(&state, user.user_id).await?
    } else {
        json!({ "enabled": false })
    };

    Ok(Json(json!({
        "enabled": global,
        "daily_goal": {
            "enabled": s.daily_goal_enabled && global,
            "target": s.daily_goal_questions,
            "answered_today": answered_today,
            "met": goal_met,
        },
        "streak": {
            "enabled": s.streak_enabled && global,
            "count": streak,
            "freezes": s.freeze_bank,
        },
        "qotd": qotd,
    })))
}

pub async fn qotd(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    if !engagement_global_enabled(&state).await? {
        return Ok(Json(json!({ "enabled": false })));
    }
    ensure_engagement(&state, user.user_id).await?;
    let settings = sqlx::query!(
        r#"SELECT qotd_enabled FROM engagement_settings WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    if !settings.qotd_enabled {
        return Ok(Json(json!({ "enabled": false })));
    }
    Ok(Json(qotd_payload(&state, user.user_id).await?))
}

async fn selected_qotd_id(state: &AppState, user_id: Uuid) -> ApiResult<Option<Uuid>> {
    let count: i64 = sqlx::query!(
        r#"SELECT COUNT(*) AS "n!" FROM question_versions WHERE status = 'published'"#
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    if count == 0 {
        return Ok(None);
    }
    let seed = sqlx::query!(
        r#"SELECT hashtext($1::text || CURRENT_DATE::text) AS "h!"#,
        user_id.to_string()
    )
    .fetch_one(&state.pool)
    .await?
    .h;
    let offset = (seed as i64).rem_euclid(count);
    let selected = sqlx::query!(
        r#"SELECT id
           FROM question_versions
           WHERE status = 'published'
           ORDER BY id
           OFFSET $1 LIMIT 1"#,
        offset
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(selected.map(|q| q.id))
}

/// Deterministic one-question-per-day pick: stable hash of (user, day) over
/// published questions. Answered state and community split after answering.
async fn qotd_payload(state: &AppState, user_id: Uuid) -> ApiResult<serde_json::Value> {
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
        return Ok(json!({
            "enabled": true,
            "answered": true,
            "community_split": split.iter().map(|r| json!({
                "chosen_index": r.chosen_index,
                "count": r.n,
            })).collect::<Vec<_>>(),
            "community_total": total,
        }));
    }
    let Some(question_id) = selected_qotd_id(state, user_id).await? else {
        return Ok(json!({ "enabled": true, "answered": false, "available": false }));
    };
    let q = sqlx::query!(
        r#"SELECT qv.id, qv.vignette, qv.options, qv.correct_index
           FROM question_versions qv
           WHERE qv.id = $1 AND qv.status = 'published'"#,
        question_id
    )
    .fetch_one(&state.pool)
    .await?;
    let options: Vec<serde_json::Value> =
        serde_json::from_value(q.options.clone()).unwrap_or_default();
    Ok(json!({
        "enabled": true,
        "answered": false,
        "available": true,
        "question_version_id": q.id,
        "vignette": q.vignette,
        "options": options.iter().map(|o| json!({ "text": o["text"] })).collect::<Vec<_>>(),
        // correct_index withheld until answered (no cheating via the payload).
    }))
}

#[derive(Deserialize)]
pub struct QotdAnswerReq {
    pub question_version_id: Uuid,
    pub chosen_index: i32,
}

pub async fn answer_qotd(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<QotdAnswerReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !engagement_global_enabled(&state).await? {
        return Err(ApiError::forbidden(
            "engagement_disabled",
            "engagement mechanics are disabled",
        ));
    }
    ensure_engagement(&state, user.user_id).await?;
    let settings = sqlx::query!(
        r#"SELECT qotd_enabled FROM engagement_settings WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    if !settings.qotd_enabled {
        return Err(ApiError::forbidden(
            "qotd_disabled",
            "question of the day is disabled",
        ));
    }
    let selected = selected_qotd_id(&state, user.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("qotd_unavailable"))?;
    if req.question_version_id != selected {
        return Err(ApiError::unprocessable(
            "qotd_mismatch",
            "question_version_id is not today's question of the day",
        ));
    }
    let q = sqlx::query!(
        r#"SELECT correct_index, jsonb_array_length(options) AS "n!"
           FROM question_versions WHERE id = $1 AND status = 'published'"#,
        req.question_version_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    if req.chosen_index < 0 || req.chosen_index as i32 >= q.n {
        return Err(ApiError::unprocessable(
            "invalid_option",
            "chosen_index is out of range",
        ));
    }
    let inserted = sqlx::query!(
        r#"INSERT INTO qotd_answers (user_id, day, question_version_id, chosen_index)
           VALUES ($1, CURRENT_DATE, $2, $3)
           ON CONFLICT (user_id, day) DO NOTHING"#,
        user.user_id,
        req.question_version_id,
        req.chosen_index
    )
    .execute(&state.pool)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "already_answered",
            "today's question of the day is already answered",
        ));
    }
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
    let correct = req.chosen_index == q.correct_index;
    Ok(Json(json!({
        "correct": correct,
        "correct_index": q.correct_index,
        "community_split": split.iter().map(|r| json!({
            "chosen_index": r.chosen_index,
            "count": r.n,
        })).collect::<Vec<_>>(),
        "community_total": total,
    })))
}

#[derive(Deserialize)]
pub struct EngagementSettingsReq {
    pub daily_goal_questions: Option<i32>,
    pub daily_goal_enabled: Option<bool>,
    pub streak_enabled: Option<bool>,
    pub qotd_enabled: Option<bool>,
}

pub async fn update_engagement_settings(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<EngagementSettingsReq>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_engagement(&state, user.user_id).await?;
    if let Some(t) = req.daily_goal_questions {
        if !(1..=500).contains(&t) {
            return Err(ApiError::unprocessable(
                "invalid_daily_goal",
                "daily goal must be 1-500 questions",
            ));
        }
        sqlx::query!(
            r#"UPDATE engagement_settings SET daily_goal_questions = $2, updated_at = now()
               WHERE user_id = $1"#,
            user.user_id,
            t
        )
        .execute(&state.pool)
        .await?;
    }
    if let Some(v) = req.daily_goal_enabled {
        sqlx::query!(
            r#"UPDATE engagement_settings SET daily_goal_enabled = $2, updated_at = now()
               WHERE user_id = $1"#,
            user.user_id,
            v
        )
        .execute(&state.pool)
        .await?;
    }
    if let Some(v) = req.streak_enabled {
        sqlx::query!(
            r#"UPDATE engagement_settings SET streak_enabled = $2, updated_at = now()
               WHERE user_id = $1"#,
            user.user_id,
            v
        )
        .execute(&state.pool)
        .await?;
    }
    if let Some(v) = req.qotd_enabled {
        sqlx::query!(
            r#"UPDATE engagement_settings SET qotd_enabled = $2, updated_at = now()
               WHERE user_id = $1"#,
            user.user_id,
            v
        )
        .execute(&state.pool)
        .await?;
    }
    let s = sqlx::query!(
        r#"SELECT daily_goal_questions, daily_goal_enabled, streak_enabled, qotd_enabled, freeze_bank
           FROM engagement_settings WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(json!({
        "daily_goal_questions": s.daily_goal_questions,
        "daily_goal_enabled": s.daily_goal_enabled,
        "streak_enabled": s.streak_enabled,
        "qotd_enabled": s.qotd_enabled,
        "freezes": s.freeze_bank,
    })))
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

// ---- COMP-01/02: competitions on the shared scoring crate --------------------

#[derive(Deserialize)]
pub struct CreateCompetitionReq {
    pub title: String,
    pub exam_id: Uuid,
    pub question_ids: Vec<Uuid>,
    pub starts_at: chrono::DateTime<chrono::Utc>,
    pub ends_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_competition(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateCompetitionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    if req.title.trim().is_empty() || req.question_ids.len() < 3 {
        return Err(ApiError::unprocessable(
            "invalid_competition",
            "title required and at least 3 questions",
        ));
    }
    if req.ends_at <= req.starts_at {
        return Err(ApiError::unprocessable(
            "invalid_window",
            "competition must end after it starts",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO competitions (id, title, exam_id, question_ids, starts_at, ends_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        id,
        req.title.trim(),
        req.exam_id,
        serde_json::to_value(&req.question_ids).map_err(|_| ApiError::internal())?,
        req.starts_at,
        req.ends_at,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "competition_id": id })))
}

pub async fn list_competitions(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT c.id, c.title, c.starts_at, c.ends_at, c.status,
                  (SELECT COUNT(*) FROM competition_entries ce
                   WHERE ce.competition_id = c.id AND ce.user_id = $1) AS "entered!"
           FROM competitions c ORDER BY c.starts_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let comps: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "competition_id": r.id, "title": r.title, "starts_at": r.starts_at,
                "ends_at": r.ends_at, "status": r.status, "entered": r.entered,
            })
        })
        .collect();
    Ok(Json(json!({ "competitions": comps })))
}

#[derive(Deserialize)]
pub struct SubmitEntryReq {
    /// Ordered answers aligned with the competition's question_ids:
    /// [{question_version_id, chosen_index, elapsed_ms}]
    pub answers: Vec<serde_json::Value>,
    pub handle: String,
    pub total_time_ms: i64,
}

/// Score one entry with the shared scoring crate and rank it against the
/// competition's entries (COMP-02: guess penalty, capped speed bonus,
/// tie-break ladder). Integrity review (COMP-04) gates prize claims later.
pub async fn submit_competition_entry(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(comp_id): Path<Uuid>,
    Json(req): Json<SubmitEntryReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let comp = sqlx::query!(
        "SELECT starts_at, ends_at, status FROM competitions WHERE id = $1",
        comp_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("competition_not_found"))?;
    let now = chrono::Utc::now();
    if now < comp.starts_at {
        return Err(ApiError::conflict(
            "not_started",
            "competition has not started",
        ));
    }
    if now > comp.ends_at {
        return Err(ApiError::conflict(
            "already_closed",
            "competition window has closed",
        ));
    }
    let handle = req.handle.trim();
    if handle.is_empty() || handle.len() > 40 {
        return Err(ApiError::unprocessable(
            "invalid_handle",
            "handle must be 1-40 characters",
        ));
    }

    // Score with the shared crate: one AnswerRecord per question.
    let cfg = ScoringConfig::default();
    let mut records = Vec::with_capacity(req.answers.len());
    let mut total_time = 0i64;
    for a in &req.answers {
        let vid = a
            .get("question_version_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                ApiError::unprocessable("invalid_answer", "missing question_version_id")
            })?;
        let chosen = a
            .get("chosen_index")
            .and_then(|v| v.as_i64())
            .map(|v| v as i16)
            .ok_or_else(|| ApiError::unprocessable("invalid_answer", "missing chosen_index"))?;
        let elapsed = a
            .get("elapsed_ms")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ApiError::unprocessable("invalid_answer", "missing elapsed_ms"))?;
        // Look up the reviewed key + difficulty for this version.
        let qv = sqlx::query!(
            "SELECT correct_index, difficulty FROM question_versions WHERE id = $1",
            vid
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("question_not_found"))?;
        let difficulty = match qv.difficulty.as_str() {
            "easy" => Difficulty::Easy,
            "hard" => Difficulty::Hard,
            _ => Difficulty::Medium,
        };
        records.push((vid, difficulty, chosen == qv.correct_index, elapsed));
        total_time += elapsed;
    }

    // Score via the crate and persist the entry (§31.1: scoring lives in
    // the shared crate, never re-implemented here).
    let answer_records: Vec<AnswerRecord> = records
        .iter()
        .map(|(_, difficulty, correct, elapsed)| AnswerRecord {
            difficulty: *difficulty,
            correct: *correct,
            elapsed_ms: *elapsed,
        })
        .collect();
    let score: f64 = answer_records.iter().map(|r| cfg.item_score(r)).sum();

    let entry_id = Uuid::new_v4();
    let score_f32 = score as f32;
    sqlx::query!(
        "INSERT INTO competition_entries
           (id, competition_id, user_id, handle, answers, score, total_time_ms, submitted_order)
         VALUES ($1, $2, $3, $4, $5, $6, $7,
           (SELECT COALESCE(MAX(submitted_order), 0) + 1 FROM competition_entries
            WHERE competition_id = $2))",
        entry_id,
        comp_id,
        user.user_id,
        handle,
        serde_json::to_value(&req.answers).map_err(|_| ApiError::internal())?,
        score_f32,
        total_time
    )
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "entry_id": entry_id,
        "score": score,
        "questions": records.len(),
    })))
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
