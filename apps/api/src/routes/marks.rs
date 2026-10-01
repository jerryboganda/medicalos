//! QB-06: learner-marked questions. Marking is private per learner and
//! feeds the "marked" session pool in the qbank builder.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "marks/MarkResponse.ts", rename = "MarkResponse")
)]
pub struct MarkResponse {
    marked: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "marks/MarkedQuestion.ts",
        rename = "MarkedQuestion"
    )
)]
pub struct MarkedQuestion {
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    question_version_id: Uuid,
    lead_in: String,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    marked_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "marks/MarkedQuestionsResponse.ts",
        rename = "MarkedQuestionsResponse"
    )
)]
pub struct MarkedQuestionsResponse {
    marks: Vec<MarkedQuestion>,
}

/// POST /v1/questions/{qid}/mark — idempotent mark.
pub async fn mark_question(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(qid): Path<Uuid>,
) -> ApiResult<Json<MarkResponse>> {
    sqlx::query!(
        "INSERT INTO question_marks (user_id, question_version_id)
         VALUES ($1, $2)
         ON CONFLICT (user_id, question_version_id) DO NOTHING",
        user.user_id,
        qid
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(MarkResponse { marked: true }))
}

/// DELETE /v1/questions/{qid}/mark — unmark; unmarking a non-marked
/// question succeeds (idempotent).
pub async fn unmark_question(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(qid): Path<Uuid>,
) -> ApiResult<Json<MarkResponse>> {
    sqlx::query!(
        "DELETE FROM question_marks
         WHERE user_id = $1 AND question_version_id = $2",
        user.user_id,
        qid
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(MarkResponse { marked: false }))
}

/// GET /v1/me/marks — the learner's marked questions (latest first).
pub async fn my_marks(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<MarkedQuestionsResponse>> {
    let rows = sqlx::query!(
        r#"SELECT qm.question_version_id, qv.lead_in, qm.created_at
           FROM question_marks qm
           JOIN question_versions qv ON qv.id = qm.question_version_id
           WHERE qm.user_id = $1
             AND qv.status = 'published'
             AND question_display_rights_active(
                 qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs
             )
           ORDER BY qm.created_at DESC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let marks = rows
        .into_iter()
        .map(|r| MarkedQuestion {
            question_version_id: r.question_version_id,
            lead_in: r.lead_in,
            marked_at: r.created_at,
        })
        .collect();
    Ok(Json(MarkedQuestionsResponse { marks }))
}
