//! QB-06: learner-marked questions. Marking is private per learner and
//! feeds the "marked" session pool in the qbank builder.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;

/// POST /v1/questions/{qid}/mark — idempotent mark.
pub async fn mark_question(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(qid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query!(
        "INSERT INTO question_marks (user_id, question_version_id)
         VALUES ($1, $2)
         ON CONFLICT (user_id, question_version_id) DO NOTHING",
        user.user_id,
        qid
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "marked": true })))
}

/// DELETE /v1/questions/{qid}/mark — unmark; unmarking a non-marked
/// question succeeds (idempotent).
pub async fn unmark_question(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(qid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query!(
        "DELETE FROM question_marks
         WHERE user_id = $1 AND question_version_id = $2",
        user.user_id,
        qid
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "marked": false })))
}

/// GET /v1/me/marks — the learner's marked questions (latest first).
pub async fn my_marks(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT qm.question_version_id, qv.lead_in, qm.created_at
           FROM question_marks qm
           JOIN question_versions qv ON qv.id = qm.question_version_id
           WHERE qm.user_id = $1
           ORDER BY qm.created_at DESC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let marks: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "question_version_id": r.question_version_id,
                "lead_in": r.lead_in,
                "marked_at": r.created_at,
            })
        })
        .collect();
    Ok(Json(json!({ "marks": marks })))
}
