//! CORE-09: guest trial (SHOULD). A visitor gets a bounded sample of real
//! published questions against an opaque client-generated key — no account,
//! no personal data. Progress migrates to the account when they sign up
//! (their notes/answers carry the same guest key until then only in the
//! client). Abuse ceiling: 5 questions per key, ever.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::seed::QuestionOption;
use crate::state::AppState;

pub const GUEST_QUESTION_CEILING: i64 = 5;

#[derive(Deserialize)]
pub struct GuestStartReq {
    /// Opaque, client-generated key (e.g. a UUID stored in localStorage).
    /// The server never learns anything identifying about the visitor.
    pub guest_key: String,
}

pub async fn start(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GuestStartReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.guest_key.len() < 16 || req.guest_key.len() > 128 {
        return Err(ApiError::unprocessable(
            "invalid_guest_key",
            "guest key must be 16-128 characters",
        ));
    }
    let existing = sqlx::query!(
        r#"SELECT COALESCE(questions_served, 0) AS "served!" FROM guest_trials
           WHERE guest_key = $1"#,
        req.guest_key
    )
    .fetch_optional(&state.pool)
    .await?;
    let served = match existing {
        Some(row) => row.served,
        None => {
            sqlx::query!(
                "INSERT INTO guest_trials (id, guest_key) VALUES ($1, $2)",
                Uuid::new_v4(),
                req.guest_key
            )
            .execute(&state.pool)
            .await?;
            0
        }
    };
    Ok(Json(json!({
        "questions_served": served,
        "ceiling": GUEST_QUESTION_CEILING,
    })))
}

/// Serve the guest their next sample question — real reviewed published
/// content (the honest trial), capped at the ceiling.
pub async fn next_question(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GuestStartReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let trial = sqlx::query!(
        "SELECT id, questions_served FROM guest_trials WHERE guest_key = $1",
        req.guest_key
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("trial_not_found"))?;
    if i64::from(trial.questions_served) >= GUEST_QUESTION_CEILING {
        return Err(ApiError::forbidden_with_details(
            "trial_finished",
            "The guest sample is finished. Create an account to keep studying.",
            json!({ "served": trial.questions_served, "ceiling": GUEST_QUESTION_CEILING }),
        ));
    }
    let q = sqlx::query!(
        r#"SELECT id, vignette, lead_in, options FROM question_versions
           WHERE status = 'published'
             AND question_display_rights_active(
                 rights_ref, source_ref, source_refs, media_refs
             )
           ORDER BY random() LIMIT 1"#,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::unprocessable("empty_pool", "No sample questions yet."))?;
    let opts: Vec<QuestionOption> =
        serde_json::from_value(q.options).map_err(|_| ApiError::internal())?;
    sqlx::query!(
        "UPDATE guest_trials SET questions_served = questions_served + 1 WHERE id = $1",
        trial.id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({
        "question_version_id": q.id,
        "vignette": q.vignette,
        "lead_in": q.lead_in,
        // §11.3: the key stays unreleased for guests; account required.
        "options": opts.iter().map(|o| serde_json::json!({"text": o.text})).collect::<Vec<_>>(),
        "served": trial.questions_served + 1,
        "ceiling": GUEST_QUESTION_CEILING,
    })))
}
