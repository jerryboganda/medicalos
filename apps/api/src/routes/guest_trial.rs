//! CORE-09: bounded unauthenticated sample questions with atomic conversion
//! into the ordinary learner practice record during registration.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{new_token, sha256_hex};
use crate::error::{ApiError, ApiResult};
use crate::seed::QuestionOption;
use crate::state::AppState;

const SAMPLE_LIMIT: i64 = 3;

#[derive(Deserialize)]
pub struct GuestAnswerReq {
    pub trial_token: String,
    pub item_index: i16,
    pub chosen_index: i16,
    pub idempotency_key: String,
}

fn invalid_trial() -> ApiError {
    ApiError::unprocessable(
        "guest_trial_invalid_or_expired",
        "guest trial token is invalid, expired, or already converted",
    )
}

fn feedback(
    already_recorded: bool,
    correct: bool,
    correct_index: i16,
    options: serde_json::Value,
    key_learning_point: String,
    exam_tip: Option<String>,
) -> ApiResult<serde_json::Value> {
    let options: Vec<QuestionOption> =
        serde_json::from_value(options).map_err(|_| ApiError::internal())?;
    Ok(serde_json::json!({
        "already_recorded": already_recorded,
        "correct": correct,
        "correct_index": correct_index,
        "options": options,
        "key_learning_point": key_learning_point,
        "exam_tip": exam_tip,
    }))
}

pub async fn start(State(state): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    let questions = sqlx::query_as::<_, (Uuid, String, String, String, serde_json::Value)>(
        r#"SELECT qv.id, qv.vignette, qv.lead_in, qv.difficulty, qv.options
           FROM question_versions qv
           WHERE qv.status = 'published'
             AND NOT EXISTS (
                 SELECT 1 FROM question_reports r
                 WHERE r.question_version_id = qv.id AND r.status = 'quarantined'
             )
           ORDER BY qv.high_yield DESC, qv.id ASC
           LIMIT $1"#,
    )
    .bind(SAMPLE_LIMIT)
    .fetch_all(&state.pool)
    .await?;

    if questions.is_empty() {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "guest_trial_unavailable",
            message: "sample questions are not available".into(),
            details: None,
        });
    }

    let issued = new_token(Duration::hours(24));
    let token_hash = sha256_hex(&issued.token);
    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO guest_trials (token_hash, expires_at) VALUES ($1, $2)")
        .bind(&token_hash)
        .bind(issued.expires_at)
        .execute(&mut *tx)
        .await?;

    let mut items = Vec::with_capacity(questions.len());
    for (index, (question_version_id, vignette, lead_in, difficulty, raw_options)) in
        questions.into_iter().enumerate()
    {
        let item_index = index as i16;
        sqlx::query(
            "INSERT INTO guest_trial_items
               (trial_token_hash, item_index, question_version_id)
             VALUES ($1, $2, $3)",
        )
        .bind(&token_hash)
        .bind(item_index)
        .bind(question_version_id)
        .execute(&mut *tx)
        .await?;

        let options: Vec<QuestionOption> =
            serde_json::from_value(raw_options).map_err(|_| ApiError::internal())?;
        items.push(serde_json::json!({
            "item_index": item_index,
            "question_version_id": question_version_id,
            "vignette": vignette,
            "lead_in": lead_in,
            "difficulty": difficulty,
            "options": options
                .into_iter()
                .map(|option| serde_json::json!({"text": option.text}))
                .collect::<Vec<_>>(),
            "correct_index": null,
            "key_learning_point": null,
            "exam_tip": null,
        }));
    }
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "trial_token": issued.token,
        "expires_at": issued.expires_at,
        "items": items,
    })))
}

pub async fn answer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GuestAnswerReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let raw_token = req.trial_token.trim();
    let idempotency_key = req.idempotency_key.trim();
    if raw_token.is_empty() || idempotency_key.is_empty() || idempotency_key.len() > 128 {
        return Err(ApiError::unprocessable(
            "invalid_guest_answer",
            "trial token and a short idempotency key are required",
        ));
    }

    let token_hash = sha256_hex(raw_token);
    let mut tx = state.pool.begin().await?;
    let trial = sqlx::query_as::<_, (chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>)>(
        "SELECT expires_at, converted_at FROM guest_trials WHERE token_hash = $1 FOR UPDATE",
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(invalid_trial)?;
    if trial.0 <= Utc::now() || trial.1.is_some() {
        return Err(invalid_trial());
    }

    let item = sqlx::query_as::<
        _,
        (
            Option<i16>,
            Option<bool>,
            Option<String>,
            Option<chrono::DateTime<Utc>>,
            i16,
            serde_json::Value,
            String,
            Option<String>,
        ),
    >(
        r#"SELECT gti.chosen_index, gti.correct, gti.idempotency_key, gti.answered_at,
                  qv.correct_index, qv.options, qv.key_learning_point, qv.exam_tip
           FROM guest_trial_items gti
           JOIN question_versions qv ON qv.id = gti.question_version_id
           WHERE gti.trial_token_hash = $1 AND gti.item_index = $2
           FOR UPDATE"#,
    )
    .bind(&token_hash)
    .bind(req.item_index)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("guest_trial_item_not_found"))?;

    if item.3.is_some() {
        if item.2.as_deref() != Some(idempotency_key) {
            return Err(ApiError::conflict(
                "already_answered",
                "guest trial item already answered",
            ));
        }
        tx.commit().await?;
        return Ok(Json(feedback(
            true,
            item.1.unwrap_or(false),
            item.4,
            item.5,
            item.6,
            item.7,
        )?));
    }

    let options: Vec<QuestionOption> =
        serde_json::from_value(item.5.clone()).map_err(|_| ApiError::internal())?;
    if req.chosen_index < 0 || req.chosen_index as usize >= options.len() {
        return Err(ApiError::unprocessable(
            "invalid_choice",
            "chosen_index is outside the option range",
        ));
    }

    let reused_key = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM guest_trial_items
         WHERE trial_token_hash = $1 AND idempotency_key = $2",
    )
    .bind(&token_hash)
    .bind(idempotency_key)
    .fetch_one(&mut *tx)
    .await?;
    if reused_key > 0 {
        return Err(ApiError::conflict(
            "idempotency_key_reused",
            "idempotency key already belongs to another guest answer",
        ));
    }

    let correct = req.chosen_index == item.4;
    sqlx::query(
        "UPDATE guest_trial_items
         SET chosen_index = $1, correct = $2, idempotency_key = $3, answered_at = now()
         WHERE trial_token_hash = $4 AND item_index = $5 AND answered_at IS NULL",
    )
    .bind(req.chosen_index)
    .bind(correct)
    .bind(idempotency_key)
    .bind(&token_hash)
    .bind(req.item_index)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(feedback(
        false, correct, item.4, item.5, item.6, item.7,
    )?))
}

pub async fn migrate_into_user(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    raw_token: &str,
) -> ApiResult<i64> {
    let token_hash = sha256_hex(raw_token.trim());
    let trial = sqlx::query_as::<_, (chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>)>(
        "SELECT expires_at, converted_at FROM guest_trials WHERE token_hash = $1 FOR UPDATE",
    )
    .bind(&token_hash)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(invalid_trial)?;
    if trial.0 <= Utc::now() || trial.1.is_some() {
        return Err(invalid_trial());
    }

    let answered = sqlx::query_as::<_, (i16, Uuid, i16, bool, String)>(
        r#"SELECT item_index, question_version_id, chosen_index, correct, idempotency_key
           FROM guest_trial_items
           WHERE trial_token_hash = $1
             AND answered_at IS NOT NULL
             AND chosen_index IS NOT NULL
             AND correct IS NOT NULL
             AND idempotency_key IS NOT NULL
           ORDER BY item_index"#,
    )
    .bind(&token_hash)
    .fetch_all(&mut **tx)
    .await?;

    if !answered.is_empty() {
        let session_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO practice_sessions
               (id, user_id, preset, status, submitted_at)
             VALUES ($1, $2, 'guest_trial', 'submitted', now())",
        )
        .bind(session_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

        for (item_index, question_version_id, chosen_index, correct, idempotency_key) in &answered {
            sqlx::query(
                "INSERT INTO session_items (id, session_id, item_index, question_version_id)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(Uuid::new_v4())
            .bind(session_id)
            .bind(*item_index)
            .bind(*question_version_id)
            .execute(&mut **tx)
            .await?;
            sqlx::query(
                "INSERT INTO attempts
                   (id, session_id, item_index, user_id, question_version_id,
                    chosen_index, correct, assisted, idempotency_key)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, false, $8)",
            )
            .bind(Uuid::new_v4())
            .bind(session_id)
            .bind(*item_index)
            .bind(user_id)
            .bind(*question_version_id)
            .bind(*chosen_index)
            .bind(*correct)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await?;
        }
    }

    sqlx::query("UPDATE guest_trials SET converted_at = now() WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(&mut **tx)
        .await?;
    Ok(answered.len() as i64)
}
