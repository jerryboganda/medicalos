//! QB-08: learner issue reports on question versions, with a fixed
//! 3-distinct-learner quarantine rule. Reports never disclose other learners'
//! identities to the reporter. Resolution (fix/reject) arrives with the
//! editorial console (ADMIN-06/QB-09); until then the resolve route answers
//! 501 honestly instead of faking a workflow.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Distinct-learner open reports that quarantine a version (QB-16 item
/// statistics replace this fixed rule in Phase 2).
pub const QUARANTINE_VOTES: i64 = 3;

fn valid_category(raw: &str) -> bool {
    matches!(
        raw,
        "wrong_answer"
            | "bad_explanation"
            | "typo"
            | "duplicate"
            | "outdated"
            | "broken_image"
            | "other"
    )
}

#[derive(Deserialize)]
pub struct ReportReq {
    pub category: String,
    pub note: Option<String>,
}

pub async fn report(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(version_id): Path<Uuid>,
    Json(req): Json<ReportReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !valid_category(&req.category) {
        return Err(ApiError::unprocessable(
            "invalid_category",
            "category must be wrong_answer|bad_explanation|typo|duplicate|outdated|broken_image|other",
        ));
    }
    let note = req.note.unwrap_or_default();
    if note.len() > 2000 {
        return Err(ApiError::unprocessable(
            "note_too_long",
            "note must be at most 2000 characters",
        ));
    }
    let version = sqlx::query!(
        "SELECT id FROM question_versions WHERE id = $1 AND status = 'published'",
        version_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;

    let inserted = sqlx::query!(
        "INSERT INTO question_reports (id, question_version_id, reporter_id, category, note)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (question_version_id, reporter_id) DO NOTHING
         RETURNING id",
        Uuid::new_v4(),
        version.id,
        user.user_id,
        req.category,
        note
    )
    .fetch_optional(&state.pool)
    .await?;
    if let Some(row) = inserted {
        let open_votes = sqlx::query!(
            r#"SELECT COALESCE(COUNT(*), 0) AS "n!"
               FROM question_reports
               WHERE question_version_id = $1 AND status = 'open'"#,
            version_id
        )
        .fetch_one(&state.pool)
        .await?
        .n;
        let mut quarantined = false;
        if open_votes >= QUARANTINE_VOTES {
            let updated = sqlx::query!(
                "UPDATE question_reports SET status = 'quarantined'
                 WHERE question_version_id = $1 AND status = 'open'",
                version_id
            )
            .execute(&state.pool)
            .await?;
            quarantined = updated.rows_affected() > 0;
        }
        return Ok(Json(serde_json::json!({
            "report_id": row.id,
            "already_recorded": false,
            "quarantined": quarantined,
        })));
    }
    let existing = sqlx::query!(
        "SELECT id, status FROM question_reports
         WHERE question_version_id = $1 AND reporter_id = $2",
        version_id,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({
        "report_id": existing.id,
        "already_recorded": true,
        "quarantined": existing.status == "quarantined",
    })))
}

pub async fn my_reports(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(version_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query!(
        "SELECT 1 AS one FROM question_versions WHERE id = $1",
        version_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    let rows = sqlx::query!(
        "SELECT id, category, status, created_at FROM question_reports
         WHERE question_version_id = $1 AND reporter_id = $2 ORDER BY created_at",
        version_id,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let quarantined = sqlx::query!(
        "SELECT 1 AS one FROM question_reports
         WHERE question_version_id = $1 AND status = 'quarantined' LIMIT 1",
        version_id
    )
    .fetch_optional(&state.pool)
    .await?
    .is_some();
    Ok(Json(serde_json::json!({
        "reports": rows.iter().map(|r| serde_json::json!({
            "id": r.id, "category": r.category,
            "status": r.status, "created_at": r.created_at,
        })).collect::<Vec<_>>(),
        "quarantined": quarantined,
    })))
}

/// Editorial resolution lands with the console (ADMIN-06/QB-09). This route
/// exists so the API surface is stable, but honestly refuses until then.
pub async fn resolve(
    State(_state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(_report_id): Path<Uuid>,
) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": {
            "code": "editor_console_pending",
            "message": "report resolution arrives with the editorial console",
        }})),
    )
}
