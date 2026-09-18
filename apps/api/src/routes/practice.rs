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

struct PoolQuestion {
    id: Uuid,
    vignette: String,
    lead_in: String,
    difficulty: String,
    options: serde_json::Value,
}

async fn insert_session(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    preset: &str,
    chapter_id: Option<Uuid>,
    source_session_id: Option<Uuid>,
    pool_questions: &[PoolQuestion],
) -> ApiResult<Json<serde_json::Value>> {
    let sid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO practice_sessions (id, user_id, preset, chapter_id, source_session_id)
         VALUES ($1, $2, $3, $4, $5)",
        sid,
        user_id,
        preset,
        chapter_id,
        source_session_id
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
    match req.preset.as_str() {
        "tutor" => {
            let chapter_id = req.chapter_id.ok_or_else(|| {
                ApiError::unprocessable("chapter_required", "tutor sessions need a chapter_id")
            })?;
            // LIMIT binds as i64 in sqlx.
            let count = req.question_count.unwrap_or(10).clamp(1, 50) as i64;
            let pool_qs = sqlx::query!(
                r#"SELECT id, vignette, lead_in, difficulty, options
                   FROM question_versions
                   WHERE status = 'published' AND chapter_id = $1
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
                "tutor",
                Some(chapter_id),
                None,
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
                       WHERE qv.id IN (
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
            insert_session(&state.pool, user.user_id, "revision", None, Some(src), &qs).await
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
        "SELECT preset, chapter_id, source_session_id, status FROM practice_sessions
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
                  qv.exam_tip, a.id AS "attempt_id?", a.chosen_index, a.correct
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

#[derive(Serialize)]
pub struct AnswerResponse {
    already_recorded: bool,
    correct: Option<bool>,
    correct_index: i16,
    options: Vec<QuestionOption>,
    key_learning_point: String,
    exam_tip: Option<String>,
}

fn feedback_response(
    already: bool,
    correct: Option<bool>,
    correct_index: i16,
    options: &serde_json::Value,
    key_learning_point: &str,
    exam_tip: Option<&str>,
) -> ApiResult<AnswerResponse> {
    let opts: Vec<QuestionOption> =
        serde_json::from_value(options.clone()).map_err(|_| ApiError::internal())?;
    Ok(AnswerResponse {
        already_recorded: already,
        correct,
        correct_index,
        options: opts,
        key_learning_point: key_learning_point.to_string(),
        exam_tip: exam_tip.map(str::to_string),
    })
}

pub async fn answer(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(sid): Path<Uuid>,
    Json(req): Json<AnswerReq>,
) -> ApiResult<Json<AnswerResponse>> {
    let session = sqlx::query!(
        "SELECT status FROM practice_sessions WHERE id = $1 AND user_id = $2",
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
        return Ok(Json(feedback_response(
            true,
            r.correct,
            r.correct_index,
            &r.options,
            &r.key_learning_point,
            r.exam_tip.as_deref(),
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
        return Ok(Json(feedback_response(
            true,
            r.correct,
            r.correct_index,
            &r.options,
            &r.key_learning_point,
            r.exam_tip.as_deref(),
        )?));
    }

    Ok(Json(feedback_response(
        false,
        correct,
        item.correct_index,
        &item.options,
        &item.key_learning_point,
        item.exam_tip.as_deref(),
    )?))
}

#[derive(Serialize)]
pub struct SubmitResponse {
    total: i64,
    correct: i64,
    incorrect: i64,
    skipped: i64,
    score: i64,
}

pub async fn submit(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(sid): Path<Uuid>,
) -> ApiResult<Json<SubmitResponse>> {
    let session = sqlx::query!(
        "SELECT chapter_id, source_session_id, status FROM practice_sessions
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
    if session.source_session_id.is_none() {
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
    Ok(Json(SubmitResponse {
        total: totals.total,
        correct: totals.correct,
        incorrect: totals.incorrect,
        skipped: totals.skipped,
        score,
    }))
}
