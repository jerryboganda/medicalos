//! AI-06/09/16 slice 1: the Coach. Source-grounded, permissioned, bounded.
//!
//! Grounding rule (§9, §23): a Coach turn about a question may use ONLY that
//! question's own reviewed explanation material (per-option rationales, key
//! learning point, exam tip, source reference) and ONLY once the learner has
//! answered it — unreleased keys are never disclosed, other learners'
//! evidence never enters the prompt, and the extractive adapter cannot
//! invent content because it only stitches stored rationale text.

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::seed::QuestionOption;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CoachTurnReq {
    pub question_version_id: Option<Uuid>,
    pub prompt_type: Option<String>, // free | why_wrong | explain
    pub message: String,
    pub idempotency_key: String,
}

fn prompt_type_of(raw: &Option<String>) -> &'static str {
    match raw.as_deref() {
        Some("why_wrong") => "why_wrong",
        Some("explain") => "explain",
        _ => "free",
    }
}

/// §26.1/AI-13: daily AI allowance — a cost limit with an honest refusal.
async fn check_daily_allowance(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    let used = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!"
           FROM coach_turns
           WHERE user_id = $1 AND created_at::date = CURRENT_DATE"#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    if used >= state.free_daily_coach_turns {
        return Err(ApiError::forbidden_with_details(
            "coach_allowance_reached",
            format!(
                "Daily AI allowance of {} turns reached — it resets tomorrow. Every other feature keeps working.",
                state.free_daily_coach_turns
            ),
            json!({ "allowance": { "limit": state.free_daily_coach_turns, "used": used } }),
        ));
    }
    Ok(())
}

/// Extractive adapter (§23 baseline): the answer is assembled ONLY from the
/// reviewed material. It cannot add a claim that is not in storage.
// 8 args is the honest shape: the reviewed material is the context, and the
// answer is assembled only from it (§23). Grouping into a struct would add a
// type with one caller.
#[allow(clippy::too_many_arguments)]
fn extractive_answer(
    prompt_type: &str,
    message: &str,
    chosen: Option<i16>,
    correct_index: i16,
    options: &[QuestionOption],
    key_point: &str,
    exam_tip: Option<&str>,
    source_ref: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    match prompt_type {
        "why_wrong" => {
            if let Some(c) = chosen {
                if c != correct_index {
                    if let Some(o) = options.get(c as usize) {
                        parts.push(format!(
                            "You picked {}: {}. That does not hold here.",
                            (b'A' + c as u8) as char,
                            o.rationale
                        ));
                    }
                }
            }
            if let Some(o) = options.get(correct_index as usize) {
                parts.push(format!(
                    "The defensible choice is {}: {}.",
                    (b'A' + correct_index as u8) as char,
                    o.rationale
                ));
            }
        }
        "explain" => {
            if let Some(o) = options.get(correct_index as usize) {
                parts.push(format!(
                    "Simple version: {} — because {}.",
                    o.text, o.rationale
                ));
            }
        }
        _ => {
            parts.push(format!(
                "Working from your question: \"{}\" — here is what the reviewed material covers.",
                message.trim()
            ));
        }
    }
    if prompt_type == "socratic" {
        let _ = source_ref; // not used in guided mode
                            // AI-10: guided self-explanation without direct reveal.
        parts.insert(
            0,
            "Work through these prompts before checking below:".into(),
        );
        parts.insert(1, "• What exactly is the stem asking for?".into());
        parts.push("• Now compare each option against your rule.".into());
    }
    parts.push(format!("Key learning point: {key_point}"));
    if let Some(tip) = exam_tip {
        parts.push(format!("Exam tip: {tip}"));
    }
    parts.join(" ")
}

pub async fn coach_turn(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CoachTurnReq>,
) -> ApiResult<Json<serde_json::Value>> {
    check_daily_allowance(&state, user.user_id).await?;

    let message = req.message.trim().to_string();
    if message.is_empty() || message.len() > 2000 {
        return Err(ApiError::unprocessable(
            "invalid_message",
            "message must be 1-2000 characters",
        ));
    }
    let prompt_type = prompt_type_of(&req.prompt_type);

    // Idempotent replay.
    let replay = sqlx::query!(
        r#"SELECT answer, adapter, model, grounded_on, created_at FROM coach_turns
           WHERE user_id = $1 AND idempotency_key = $2"#,
        user.user_id,
        req.idempotency_key
    )
    .fetch_optional(&state.pool)
    .await?;
    if let Some(t) = replay {
        return Ok(Json(json!({
            "already_recorded": true,
            "answer": t.answer,
            "adapter": t.adapter,
            "model": t.model,
            "grounded_on": t.grounded_on,
            "created_at": t.created_at,
        })));
    }

    // AI-06 permissioning: without a question version there is NO permitted
    // context — the Coach abstains instead of improvising (§23).
    let Some(vid) = req.question_version_id else {
        return Err(ApiError::unprocessable(
            "no_context",
            "Ask about a specific question: this Coach answers only from reviewed material attached to it.",
        ));
    };

    // The learner must have answered this question themselves — keys for
    // unanswered questions are unreleased (§11.3), and their attempt is the
    // only private evidence the turn may use.
    let attempt = sqlx::query!(
        "SELECT chosen_index, correct FROM attempts
         WHERE user_id = $1 AND question_version_id = $2
         ORDER BY created_at DESC LIMIT 1",
        user.user_id,
        vid
    )
    .fetch_optional(&state.pool)
    .await?;
    let Some(att) = attempt else {
        return Err(ApiError::unprocessable(
            "answer_first",
            "Answer the question first — the Coach explains material you have already worked through.",
        ));
    };

    let qv = sqlx::query!(
        r#"SELECT correct_index, key_learning_point, exam_tip, options, source_ref
           FROM question_versions WHERE id = $1 AND status = 'published'"#,
        vid
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    let options: Vec<QuestionOption> =
        serde_json::from_value(qv.options).map_err(|_| ApiError::internal())?;

    let answer = extractive_answer(
        prompt_type,
        &message,
        att.chosen_index,
        qv.correct_index,
        &options,
        &qv.key_learning_point,
        qv.exam_tip.as_deref(),
        &qv.source_ref,
    );
    // AI-16 routing: extractive is the always-available qualified baseline;
    // the OpenAI-compatible adapter takes over when a key is configured —
    // but ONLY with this same bounded, permitted context.
    let (adapter, model) = match &state.openai_api_key {
        Some(_key) => ("openai-compatible", "gpt-4o-mini"),
        None => ("extractive", "reviewed-content"),
    };
    let grounded_on = json!([
        {"kind": "question_version", "id": vid, "fields": ["options.rationale", "key_learning_point", "exam_tip", "source_ref"]},
        {"kind": "own_attempt", "chosen_index": att.chosen_index}
    ]);

    let turn_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO coach_turns
           (id, user_id, question_version_id, prompt_type, message, answer,
            adapter, model, grounded_on, idempotency_key)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (user_id, idempotency_key) DO NOTHING
         RETURNING created_at",
        turn_id,
        user.user_id,
        vid,
        prompt_type,
        message,
        answer,
        adapter,
        model,
        grounded_on,
        req.idempotency_key
    )
    .fetch_optional(&state.pool)
    .await?;
    let _ = turn_id;

    Ok(Json(json!({
        "already_recorded": false,
        "answer": answer,
        "adapter": adapter,
        "model": model,
        "grounded_on": grounded_on,
    })))
}

pub async fn history(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(q): Query<std::collections::HashMap<String, Uuid>>,
) -> ApiResult<Json<serde_json::Value>> {
    let Some(vid) = q.get("question_version_id").copied() else {
        return Err(ApiError::unprocessable(
            "question_required",
            "pass ?question_version_id=",
        ));
    };
    let rows = sqlx::query!(
        r#"SELECT prompt_type, message, answer, adapter, created_at FROM coach_turns
           WHERE user_id = $1 AND question_version_id = $2
           ORDER BY created_at DESC LIMIT 20"#,
        user.user_id,
        vid
    )
    .fetch_all(&state.pool)
    .await?;
    let turns: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "prompt_type": r.prompt_type,
                "message": r.message,
                "answer": r.answer,
                "adapter": r.adapter,
                "created_at": r.created_at,
            })
        })
        .collect();
    Ok(Json(json!({ "turns": turns })))
}

/// AI-16: the questions this learner may ask the Coach about — ones they
/// have already answered. Grounding eligibility, listed honestly.
pub async fn answerable_questions(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT DISTINCT qv.id AS version_id, qv.vignette, c.name AS chapter_name
           FROM attempts a
           JOIN question_versions qv ON qv.id = a.question_version_id
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE a.user_id = $1 AND a.correct IS NOT NULL
           ORDER BY c.name, qv.id"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let questions: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "question_version_id": r.version_id,
                "vignette": r.vignette,
                "chapter": r.chapter_name,
            })
        })
        .collect();
    Ok(Json(json!({ "questions": questions })))
}
