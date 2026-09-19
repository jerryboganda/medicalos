//! SIM-06/CAREER-02 adjacent: scenario debriefs and appeals. A debrief is
//! the run's own transcript plus outcome — evidence, not commentary.
//! Appeals record the learner's challenge; a human reviewer resolves it
//! (SIM-07) — the route records, never auto-decides.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub async fn debrief(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let run = sqlx::query!(
        r#"SELECT r.current_state, r.transcript, r.started_at, r.finished_at,
                  s.title, s.version
           FROM scenario_runs r JOIN scenarios s ON s.id = r.scenario_id
           WHERE r.id = $1 AND r.user_id = $2"#,
        run_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    Ok(Json(json!({
        "scenario": run.title,
        "scenario_version": run.version,
        "final_state": run.current_state,
        "transcript": run.transcript,
        "started_at": run.started_at,
        "finished_at": run.finished_at,
    })))
}

// ---- appeals (learner challenge; human review resolves, SIM-07) --------------

#[derive(Deserialize)]
pub struct AppealReq {
    pub question_version_id: Uuid,
    pub reason: String,
}

pub async fn submit_appeal(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<AppealReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let reason = req.reason.trim();
    if reason.len() < 10 || reason.len() > 2000 {
        return Err(ApiError::unprocessable(
            "invalid_reason",
            "reason must be 10-2000 characters",
        ));
    }
    // Only appealable if the learner actually answered this version.
    let answered = sqlx::query!(
        "SELECT 1 AS one FROM attempts
         WHERE user_id = $1 AND question_version_id = $2",
        user.user_id,
        req.question_version_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ApiError::unprocessable(
            "not_answered",
            "you can only appeal a question you have answered",
        )
    })?;
    let _ = answered;
    let appeal_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO appeals (id, user_id, question_version_id, reason, status)
         VALUES ($1, $2, $3, $4, 'open')",
        appeal_id,
        user.user_id,
        req.question_version_id,
        reason
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({
        "appeal_id": appeal_id,
        "status": "open",
        "note": "A human reviewer decides appeals; the Coach cannot."
    })))
}
