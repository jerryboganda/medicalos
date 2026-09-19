//! QB-03/QB-04/QB-05/EX-04: practice sessions, tutor feedback, idempotent
//! durable answers, submission with evidence updates and plan revision.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::agent;
use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::seed::QuestionOption;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateSessionReq {
    pub preset: String,
    pub chapter_id: Option<Uuid>,
    pub question_count: Option<i32>,
    pub source_session_id: Option<Uuid>,
    /// EX-08: required for the timed preset, validated server-side.
    pub time_limit_seconds: Option<i64>,
}

#[derive(Serialize)]
struct ItemPayload {
    item_index: i16,
    question_version_id: Uuid,
    vignette: String,
    lead_in: String,
    difficulty: String,
    options: Vec<serde_json::Value>,
}

pub(crate) struct PoolQuestion {
    pub id: Uuid,
    pub vignette: String,
    pub lead_in: String,
    pub difficulty: String,
    pub options: serde_json::Value,
}

async fn insert_session(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    preset: &str,
    chapter_id: Option<Uuid>,
    source_session_id: Option<Uuid>,
    time_limit_seconds: Option<i32>,
    pool_questions: &[PoolQuestion],
) -> ApiResult<Json<serde_json::Value>> {
    // EX-08: the server issues the deadline — the client never sets it, and
    // answer acceptance is checked against it server-side.
    let deadline = time_limit_seconds
        .map(|limit| chrono::Utc::now() + chrono::Duration::seconds(limit as i64));
    let sid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO practice_sessions
           (id, user_id, preset, chapter_id, source_session_id, time_limit_seconds, deadline)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        sid,
        user_id,
        preset,
        chapter_id,
        source_session_id,
        time_limit_seconds,
        deadline
    )
    .execute(pool)
    .await?;
    let mut items = Vec::with_capacity(pool_questions.len());
    for (i, q) in pool_questions.iter().enumerate() {
        let idx = i as i16;
        sqlx::query!(
            "INSERT INTO session_items (id, session_id, item_index, question_version_id)
             VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            sid,
            idx,
            q.id
        )
        .execute(pool)
        .await?;
        let opts: Vec<QuestionOption> =
            serde_json::from_value(q.options.clone()).map_err(|_| ApiError::internal())?;
        items.push(ItemPayload {
            item_index: idx,
            question_version_id: q.id,
            vignette: q.vignette.clone(),
            lead_in: q.lead_in.clone(),
            difficulty: q.difficulty.clone(),
            // §11.3: no answer keys or rationales before they are permitted.
            options: opts
                .into_iter()
                .map(|o| serde_json::json!({"text": o.text}))
                .collect(),
        });
    }
    Ok(Json(serde_json::json!({"session_id": sid, "items": items})))
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CreateSessionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    // COM-01: the free-tier daily allowance is an entitlement check (§26.1) —
    // upgrade prompts may originate only from here, never from the Coach.
    // # ponytail: revision sessions are exempt (they re-practice already-
    // served items); per-tier entitlement service lands with billing.
    if req.preset.as_str() != "revision" {
        let used = sqlx::query!(
            r#"SELECT COALESCE(COUNT(*), 0) AS "n!"
               FROM attempts
               WHERE user_id = $1 AND created_at::date = CURRENT_DATE"#,
            user.user_id
        )
        .fetch_one(&state.pool)
        .await?
        .n;
        if used >= state.free_daily_questions {
            return Err(ApiError::forbidden_with_details(
                "free_allowance_reached",
                format!(
                    "Daily free allowance of {} questions reached — it resets tomorrow.",
                    state.free_daily_questions
                ),
                serde_json::json!({
                    "allowance": { "limit": state.free_daily_questions, "used": used }
                }),
            ));
        }
    }
    match req.preset.as_str() {
        "tutor" | "timed" => {
            let chapter_id = req.chapter_id.ok_or_else(|| {
                ApiError::unprocessable("chapter_required", "practice sessions need a chapter_id")
            })?;
            // LIMIT binds as i64 in sqlx.
            let count = req.question_count.unwrap_or(10).clamp(1, 50) as i64;
            // EX-08: timed sessions carry a server-issued deadline. The floor
            // is configurable so CI/E2E can run short timed sessions.
            let time_limit_seconds = if req.preset == "timed" {
                let limit = req.time_limit_seconds.ok_or_else(|| {
                    ApiError::unprocessable(
                        "time_limit_required",
                        "timed sessions need time_limit_seconds",
                    )
                })?;
                if !(state.min_time_limit_seconds..=14_400).contains(&limit) {
                    return Err(ApiError::unprocessable(
                        "time_limit_out_of_range",
                        format!(
                            "time_limit_seconds must be between {} and 14400",
                            state.min_time_limit_seconds
                        ),
                    ));
                }
                Some(limit as i32)
            } else {
                None
            };
            let pool_qs = sqlx::query!(
                r#"SELECT id, vignette, lead_in, difficulty, options
                   FROM question_versions qv
                   WHERE status = 'published' AND chapter_id = $1
                     AND NOT EXISTS (
                         SELECT 1 FROM question_reports r
                         WHERE r.question_version_id = qv.id
                           AND r.status = 'quarantined'
                     )
                   ORDER BY random() LIMIT $2"#,
                chapter_id,
                count
            )
            .fetch_all(&state.pool)
            .await?;
            if pool_qs.is_empty() {
                return Err(ApiError::unprocessable(
                    "empty_pool",
                    "No questions are available for this selection.",
                ));
            }
            let qs: Vec<PoolQuestion> = pool_qs
                .into_iter()
                .map(|r| PoolQuestion {
                    id: r.id,
                    vignette: r.vignette,
                    lead_in: r.lead_in,
                    difficulty: r.difficulty,
                    options: r.options,
                })
                .collect();
            insert_session(
                &state.pool,
                user.user_id,
                &req.preset,
                Some(chapter_id),
                None,
                time_limit_seconds,
                &qs,
            )
            .await
        }
        "revision" => {
            let src = req.source_session_id.ok_or_else(|| {
                ApiError::unprocessable(
                    "source_session_required",
                    "revision sessions need a source_session_id",
                )
            })?;
            sqlx::query!(
                "SELECT 1 AS one FROM practice_sessions
                 WHERE id = $1 AND user_id = $2 AND status = 'submitted'",
                src,
                user.user_id
            )
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| {
                ApiError::unprocessable(
                    "source_not_found",
                    "source session does not exist or is not submitted",
                )
            })?;
            let pool_qs = sqlx::query!(
                r#"SELECT id, vignette, lead_in, difficulty, options FROM (
                       SELECT DISTINCT qv.id, qv.vignette, qv.lead_in, qv.difficulty, qv.options
                       FROM question_versions qv
                       WHERE (qv.id IN (
                           SELECT question_version_id FROM attempts
                           WHERE session_id = $1 AND correct = FALSE
                       )
                       OR qv.id IN (
                           SELECT si.question_version_id FROM session_items si
                           WHERE si.session_id = $1 AND NOT EXISTS (
                               SELECT 1 FROM attempts a
                               WHERE a.session_id = si.session_id
                                 AND a.item_index = si.item_index
                           )
                       ))
                       AND NOT EXISTS (
                           SELECT 1 FROM question_reports r
                           WHERE r.question_version_id = qv.id
                             AND r.status = 'quarantined'
                       )
                   ) t
                   ORDER BY random()"#,
                src
            )
            .fetch_all(&state.pool)
            .await?;
            if pool_qs.is_empty() {
                return Err(ApiError::unprocessable(
                    "nothing_to_revise",
                    "that session has no missed questions to re-practice",
                ));
            }
            let qs: Vec<PoolQuestion> = pool_qs
                .into_iter()
                .map(|r| PoolQuestion {
                    id: r.id,
                    vignette: r.vignette,
                    lead_in: r.lead_in,
                    difficulty: r.difficulty,
                    options: r.options,
                })
                .collect();
            insert_session(
                &state.pool,
                user.user_id,
                "revision",
                None,
                Some(src),
                None,
                &qs,
            )
            .await
        }
        other => Err(ApiError::unprocessable(
            "unknown_preset",
            format!("unknown preset {other}"),
        )),
    }
}

/// Session detail for the client: full item list for the navigator, with
/// answer keys and rationales revealed only for already-answered items
/// (§11.3 — nothing unreleased reaches the client).
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(sid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let session = sqlx::query!(
        "SELECT preset, chapter_id, source_session_id, status, time_limit_seconds, deadline, mock_id
         FROM practice_sessions
         WHERE id = $1 AND user_id = $2",
        sid,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("session_not_found"))?;

    let items = sqlx::query!(
        r#"SELECT si.item_index, qv.id AS question_version_id, qv.vignette, qv.lead_in,
                  qv.difficulty, qv.options, qv.correct_index, qv.key_learning_point,
                  qv.exam_tip, a.id AS "attempt_id?", a.chosen_index, a.correct,
                  EXISTS (
                      SELECT 1 FROM question_reports r
                      WHERE r.question_version_id = qv.id
                        AND r.status IN ('open', 'quarantined')
                  ) AS "flagged!",
                  EXISTS (
                      SELECT 1 FROM question_reports r
                      WHERE r.question_version_id = qv.id
                        AND r.status = 'quarantined'
                  ) AS "quarantined!"
           FROM session_items si
           JOIN question_versions qv ON qv.id = si.question_version_id
           LEFT JOIN attempts a
             ON a.session_id = si.session_id AND a.item_index = si.item_index
           WHERE si.session_id = $1
           ORDER BY si.item_index"#,
        sid
    )
    .fetch_all(&state.pool)
    .await?;

    let mut out = Vec::with_capacity(items.len());
    for it in items {
        let opts: Vec<QuestionOption> =
            serde_json::from_value(it.options).map_err(|_| ApiError::internal())?;
        let answered = it.attempt_id.is_some();
        let public_opts: Vec<serde_json::Value> = opts
            .iter()
            .map(|o| serde_json::json!({"text": o.text}))
            .collect();
        // QB-08: honest flag state so the UI can label affected items.
        let report_status = if it.quarantined {
            serde_json::Value::String("quarantined".into())
        } else if it.flagged {
            serde_json::Value::String("open".into())
        } else {
            serde_json::Value::Null
        };
        let mut item = serde_json::json!({
            "item_index": it.item_index,
            "question_version_id": it.question_version_id,
            "vignette": it.vignette,
            "lead_in": it.lead_in,
            "difficulty": it.difficulty,
            "options": public_opts,
            "answered": answered,
            "chosen_index": it.chosen_index,
            "correct": it.correct,
            "correct_index": null,
            "key_learning_point": null,
            "exam_tip": null,
            "report_status": report_status,
        });
        if answered {
            item["correct_index"] = serde_json::json!(it.correct_index);
            item["options"] = serde_json::json!(opts);
            item["key_learning_point"] = serde_json::json!(it.key_learning_point);
            item["exam_tip"] = serde_json::json!(it.exam_tip);
        }
        out.push(item);
    }

    Ok(Json(serde_json::json!({
        "session_id": sid,
        "preset": session.preset,
        "chapter_id": session.chapter_id,
        "source_session_id": session.source_session_id,
        "status": session.status,
        "mock_id": session.mock_id,
        "time_limit_seconds": session.time_limit_seconds,
        // EX-08: the client derives its countdown from these two values, so
        // changing the device clock never extends the timer.
        "deadline": session.deadline,
        "server_now": chrono::Utc::now(),
        "items": out,
    })))
}

#[derive(Deserialize)]
pub struct AnswerReq {
    pub item_index: i16,
    pub chosen_index: Option<i16>,
    pub confidence: Option<String>,
    pub idempotency_key: String,
}

pub async fn answer(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(sid): Path<Uuid>,
    Json(req): Json<AnswerReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let session = sqlx::query!(
        "SELECT status, deadline, preset FROM practice_sessions WHERE id = $1 AND user_id = $2",
        sid,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("session_not_found"))?;
    if session.status != "open" {
        return Err(ApiError::conflict(
            "session_closed",
            "session already submitted",
        ));
    }
    // EX-08: the server-side deadline is the single source of truth — once it
    // passes, no new answers are recorded and the client auto-submits.
    if let Some(deadline) = session.deadline {
        if chrono::Utc::now() > deadline {
            return Err(ApiError::conflict(
                "session_expired",
                "Time is up. The session submits with what you answered.",
            ));
        }
    }

    // Idempotent replay: same key, same stored answer, no duplicate attempt.
    let replay = sqlx::query!(
        r#"SELECT a.chosen_index, a.correct, qv.correct_index, qv.options,
                  qv.key_learning_point, qv.exam_tip
           FROM attempts a JOIN question_versions qv ON qv.id = a.question_version_id
           WHERE a.session_id = $1 AND a.idempotency_key = $2"#,
        sid,
        req.idempotency_key
    )
    .fetch_optional(&state.pool)
    .await?;
    if let Some(r) = replay {
        return Ok(Json(response_for_preset(
            &session.preset,
            true,
            r.correct,
            r.correct_index,
            &r.options,
            r.key_learning_point,
            r.exam_tip,
        )?));
    }

    let item = sqlx::query!(
        r#"SELECT qv.id, qv.correct_index, qv.options, qv.key_learning_point, qv.exam_tip
           FROM session_items si JOIN question_versions qv ON qv.id = si.question_version_id
           WHERE si.session_id = $1 AND si.item_index = $2"#,
        sid,
        req.item_index
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("unknown_item"))?;

    let already = sqlx::query!(
        "SELECT 1 AS one FROM attempts WHERE session_id = $1 AND item_index = $2",
        sid,
        req.item_index
    )
    .fetch_optional(&state.pool)
    .await?;
    if already.is_some() {
        return Err(ApiError::conflict(
            "already_answered",
            "this item already has an answer (first answer wins)",
        ));
    }

    if let Some(c) = req.chosen_index {
        let n_opts = item
            .options
            .as_array()
            .map(|a| a.len())
            .ok_or_else(ApiError::internal)?;
        if c < 0 || c as usize >= n_opts {
            return Err(ApiError::unprocessable(
                "invalid_option",
                "chosen_index is out of range",
            ));
        }
    }
    if let Some(conf) = &req.confidence {
        if conf != "sure" && conf != "unsure" {
            return Err(ApiError::unprocessable(
                "invalid_confidence",
                "confidence must be 'sure' or 'unsure'",
            ));
        }
    }
    let correct = req.chosen_index.map(|c| c == item.correct_index);

    let inserted = sqlx::query!(
        r#"INSERT INTO attempts
             (id, session_id, item_index, user_id, question_version_id,
              chosen_index, correct, confidence, assisted, idempotency_key)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false, $9)
           ON CONFLICT DO NOTHING
           RETURNING id"#,
        Uuid::new_v4(),
        sid,
        req.item_index,
        user.user_id,
        item.id,
        req.chosen_index,
        correct,
        req.confidence,
        req.idempotency_key
    )
    .fetch_optional(&state.pool)
    .await?;
    if inserted.is_none() {
        // Lost a race with the same key: serve the stored answer.
        let r = sqlx::query!(
            r#"SELECT a.chosen_index, a.correct, qv.correct_index, qv.options,
                      qv.key_learning_point, qv.exam_tip
               FROM attempts a JOIN question_versions qv ON qv.id = a.question_version_id
               WHERE a.session_id = $1 AND a.idempotency_key = $2"#,
            sid,
            req.idempotency_key
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::conflict("already_answered", "item already answered"))?;
        return Ok(Json(response_for_preset(
            &session.preset,
            true,
            r.correct,
            r.correct_index,
            &r.options,
            r.key_learning_point,
            r.exam_tip,
        )?));
    }

    Ok(Json(response_for_preset(
        &session.preset,
        false,
        correct,
        item.correct_index,
        &item.options,
        item.key_learning_point.clone(),
        item.exam_tip.clone(),
    )?))
}

/// §11.2 one-session vocabulary: tutor reveals immediately; exam-style
/// presets (mock) defer everything until submission — not even the key is
/// released early.
fn response_for_preset(
    preset: &str,
    already: bool,
    correct: Option<bool>,
    correct_index: i16,
    options: &serde_json::Value,
    key_learning_point: String,
    exam_tip: Option<String>,
) -> ApiResult<serde_json::Value> {
    if preset == "mock" {
        return Ok(serde_json::json!({
            "already_recorded": already,
            "recorded": true,
        }));
    }
    let opts: Vec<QuestionOption> =
        serde_json::from_value(options.clone()).map_err(|_| ApiError::internal())?;
    Ok(serde_json::json!({
        "already_recorded": already,
        "correct": correct,
        "correct_index": correct_index,
        "options": opts,
        "key_learning_point": key_learning_point,
        "exam_tip": exam_tip,
    }))
}

pub async fn submit(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(sid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let session = sqlx::query!(
        "SELECT chapter_id, source_session_id, status, preset, mock_id FROM practice_sessions
         WHERE id = $1 AND user_id = $2",
        sid,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("session_not_found"))?;
    if session.status != "open" {
        return Err(ApiError::conflict(
            "session_closed",
            "session already submitted",
        ));
    }

    let totals = sqlx::query!(
        r#"SELECT
             COUNT(*) AS "total!",
             COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS "correct!",
             COALESCE(COUNT(*) FILTER (WHERE a.chosen_index IS NOT NULL AND a.correct = FALSE), 0) AS "incorrect!",
             COALESCE(COUNT(*) FILTER (WHERE a.id IS NULL OR a.chosen_index IS NULL), 0) AS "skipped!"
           FROM session_items si
           LEFT JOIN attempts a
             ON a.session_id = si.session_id AND a.item_index = si.item_index
           WHERE si.session_id = $1"#,
        sid
    )
    .fetch_one(&state.pool)
    .await?;

    let updated = sqlx::query!(
        "UPDATE practice_sessions SET status = 'submitted', submitted_at = now()
         WHERE id = $1 AND status = 'open'",
        sid
    )
    .execute(&state.pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "session_closed",
            "session already submitted",
        ));
    }

    agent::mark_matching_task_done(
        &state.pool,
        user.user_id,
        session.chapter_id,
        session.source_session_id,
    )
    .await?;
    agent::update_learner_state(&state.pool, user.user_id, sid).await?;
    // Revision plans come only from self-directed practice; mocks and
    // revision sessions never spawn them (anti-loop, §8.6).
    if session.preset == "tutor" {
        agent::maybe_create_revision(
            &state.pool,
            user.user_id,
            sid,
            totals.incorrect,
            totals.skipped,
        )
        .await?;
    }

    let score = if totals.total == 0 {
        0
    } else {
        totals.correct * 100 / totals.total
    };

    // QB-15: expected-score comparison for self-built sessions, computed
    // from community correct-rates of the exact questions served. Percentile
    // is reserved for fixed forms (mocks). Hidden below the min sample.
    let expected_score =
        expected_score_for_session(&state.pool, sid, state.community_min_sample).await?;

    let mut body = serde_json::json!({
        "total": totals.total,
        "correct": totals.correct,
        "incorrect": totals.incorrect,
        "skipped": totals.skipped,
        "score": score,
        "expected_score": expected_score,
        "mock": null,
    });

    if let Some(mock_id) = session.mock_id {
        let mock = sqlx::query!("SELECT pass_mark_percent FROM mocks WHERE id = $1", mock_id)
            .fetch_one(&state.pool)
            .await?;
        let passed = score >= mock.pass_mark_percent as i64;
        sqlx::query!(
            "INSERT INTO mock_attempts
               (id, mock_id, user_id, session_id, score_percent, passed)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (mock_id, user_id, session_id) DO NOTHING",
            Uuid::new_v4(),
            mock_id,
            user.user_id,
            sid,
            score as i32,
            passed
        )
        .execute(&state.pool)
        .await?;
        // §11.8: percentile among takers of the SAME form, only once the
        // sample is meaningful — otherwise an honest null.
        let takers = sqlx::query!(
            r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM mock_attempts WHERE mock_id = $1"#,
            mock_id
        )
        .fetch_one(&state.pool)
        .await?
        .n;
        let below = sqlx::query!(
            r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM mock_attempts
               WHERE mock_id = $1 AND score_percent < $2"#,
            mock_id,
            score as i32
        )
        .fetch_one(&state.pool)
        .await?
        .n;
        let percentile = if takers >= state.community_min_sample && takers > 1 {
            Some((below * 100 / (takers - 1)) as i32)
        } else {
            None
        };
        let breakdown = sqlx::query!(
            r#"SELECT c.name AS chapter_name,
                      COALESCE(COUNT(*), 0) AS "total!",
                      COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS "correct!"
               FROM session_items si
               JOIN question_versions qv ON qv.id = si.question_version_id
               JOIN curriculum_nodes c ON c.id = qv.chapter_id
               LEFT JOIN attempts a
                 ON a.session_id = si.session_id AND a.item_index = si.item_index
               WHERE si.session_id = $1
               GROUP BY c.name ORDER BY c.name"#,
            sid
        )
        .fetch_all(&state.pool)
        .await?;
        body["mock"] = serde_json::json!({
            "score_percent": score,
            "passed": passed,
            "pass_mark_percent": mock.pass_mark_percent,
            "percentile": percentile,
            "takers": takers,
            "breakdown": breakdown.iter().map(|b| serde_json::json!({
                "chapter": b.chapter_name,
                "total": b.total,
                "correct": b.correct,
            })).collect::<Vec<_>>(),
        });
    }

    Ok(Json(body))
}

/// Mean community correct-rate over the session's questions that already have
/// enough attempts (QB-15). None until at least one question qualifies.
async fn expected_score_for_session(
    pool: &sqlx::PgPool,
    sid: Uuid,
    min_sample: i64,
) -> ApiResult<Option<i64>> {
    let rows = sqlx::query!(
        r#"SELECT qv.id,
                  COALESCE(COUNT(*), 0) AS "attempts!",
                  COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS "correct!"
           FROM session_items si
           JOIN question_versions qv ON qv.id = si.question_version_id
           LEFT JOIN attempts a
             ON a.question_version_id = si.question_version_id
            AND a.correct IS NOT NULL
           WHERE si.session_id = $1
           GROUP BY qv.id"#,
        sid
    )
    .fetch_all(pool)
    .await?;
    let mut rates: Vec<f64> = Vec::new();
    for r in rows {
        if r.attempts >= min_sample {
            rates.push(r.correct as f64 * 100.0 / r.attempts as f64);
        }
    }
    if rates.is_empty() {
        return Ok(None);
    }
    let mean = rates.iter().sum::<f64>() / rates.len() as f64;
    Ok(Some(mean.round() as i64))
}

/// QB-15: per-question community statistics. Aggregates never identify a
/// learner; the numbers stay hidden until the minimum sample is met, and the
/// response says so instead of showing nothing.
pub async fn community_stats(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(vid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let totals = sqlx::query!(
        r#"SELECT
             COALESCE(COUNT(*), 0) AS "attempts!",
             COALESCE(COUNT(*) FILTER (WHERE correct = TRUE), 0) AS "correct!"
           FROM attempts
           WHERE question_version_id = $1 AND chosen_index IS NOT NULL"#,
        vid
    )
    .fetch_one(&state.pool)
    .await?;
    let distribution = sqlx::query!(
        r#"SELECT chosen_index, COALESCE(COUNT(*), 0) AS "picks!"
           FROM attempts
           WHERE question_version_id = $1 AND chosen_index IS NOT NULL
           GROUP BY chosen_index ORDER BY chosen_index"#,
        vid
    )
    .fetch_all(&state.pool)
    .await?;
    let revealed = totals.attempts >= state.community_min_sample;
    let correct_rate_percent = if revealed && totals.attempts > 0 {
        Some((totals.correct * 100 / totals.attempts) as i32)
    } else {
        None
    };
    Ok(Json(serde_json::json!({
        "min_sample": state.community_min_sample,
        "attempts": totals.attempts,
        "revealed": revealed,
        "correct_rate_percent": correct_rate_percent,
        "option_distribution": if revealed {
            serde_json::Value::from(distribution.iter().map(|d| serde_json::json!({
                "index": d.chosen_index,
                "picks": d.picks,
            })).collect::<Vec<_>>())
        } else {
            serde_json::Value::Null
        },
    })))
}
