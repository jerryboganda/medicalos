//! PLAN-01/AI-02: today's plan with cold start, honest learner evidence, and
//! revision undo (AI-07).

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::agent;
use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Serialize)]
pub struct TaskView {
    id: Uuid,
    kind: String,
    title: String,
    chapter_id: Option<Uuid>,
    question_count: i32,
    status: String,
}

#[derive(Serialize)]
pub struct RevisionView {
    id: Uuid,
    to_version: i32,
    reason_code: String,
    explanation: String,
    automatic: bool,
    undone: bool,
}

#[derive(Serialize)]
pub struct LearnerChapter {
    chapter_id: Uuid,
    chapter_name: String,
    /// Observed independent accuracy, 0-100. Null under the evidence floor —
    /// never a prediction (ADR 0004, AI-02).
    mastery_index: Option<i32>,
    evidence_level: &'static str,
    independent_count: i64,
}

#[derive(Serialize)]
pub struct TodayResponse {
    plan_id: Uuid,
    version: i32,
    tasks: Vec<TaskView>,
    revisions: Vec<RevisionView>,
    learner: Vec<LearnerChapter>,
}

pub async fn today(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<TodayResponse>> {
    let (plan_id, version) = agent::get_or_create_today(&state.pool, user.user_id).await?;
    let task_rows = sqlx::query!(
        "SELECT id, kind, title, chapter_id, question_count, status FROM plan_tasks
         WHERE plan_id = $1 ORDER BY created_at",
        plan_id
    )
    .fetch_all(&state.pool)
    .await?;
    let revision_rows = sqlx::query!(
        r#"SELECT r.id, r.to_version, r.reason_code, r.explanation, r.automatic, r.undone
           FROM plan_revisions r JOIN plans p ON p.id = r.plan_id
           WHERE p.user_id = $1 AND p.plan_date = CURRENT_DATE
           ORDER BY r.created_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let learner_rows = sqlx::query!(
        r#"SELECT qv.chapter_id, c.name AS chapter_name,
                  COALESCE(COUNT(*), 0) AS "independent_count!",
                  COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS "correct!"
           FROM attempts a
           JOIN question_versions qv ON qv.id = a.question_version_id
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE a.user_id = $1 AND a.assisted = FALSE AND a.correct IS NOT NULL
           GROUP BY qv.chapter_id, c.name
           ORDER BY c.name"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;

    let tasks = task_rows
        .into_iter()
        .map(|t| TaskView {
            id: t.id,
            kind: t.kind,
            title: t.title,
            chapter_id: t.chapter_id,
            question_count: t.question_count,
            status: t.status,
        })
        .collect();
    let revisions = revision_rows
        .into_iter()
        .map(|r| RevisionView {
            id: r.id,
            to_version: r.to_version,
            reason_code: r.reason_code,
            explanation: r.explanation,
            automatic: r.automatic,
            undone: r.undone,
        })
        .collect();
    let learner = learner_rows
        .into_iter()
        .map(|r| {
            // §8.8: show the evidence level beside the index; under roughly 10
            // independent attempts there is no meaningful index at all.
            let (mastery_index, evidence_level) = if r.independent_count < 10 {
                (None, "low_evidence")
            } else if r.independent_count < 30 {
                (
                    Some((r.correct * 100 / r.independent_count) as i32),
                    "developing",
                )
            } else {
                (
                    Some((r.correct * 100 / r.independent_count) as i32),
                    "established",
                )
            };
            LearnerChapter {
                chapter_id: r.chapter_id,
                chapter_name: r.chapter_name,
                mastery_index,
                evidence_level,
                independent_count: r.independent_count,
            }
        })
        .collect();

    Ok(Json(TodayResponse {
        plan_id,
        version,
        tasks,
        revisions,
        learner,
    }))
}

#[derive(Serialize)]
pub struct UndoResponse {
    plan_version: i32,
}

pub async fn undo_revision(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((pid, rid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<UndoResponse>> {
    let plan = sqlx::query!(
        "SELECT id, version FROM plans WHERE id = $1 AND user_id = $2",
        pid,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("plan_not_found"))?;
    let revision = sqlx::query!(
        "SELECT id, undone FROM plan_revisions WHERE id = $1 AND plan_id = $2",
        rid,
        pid
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("revision_not_found"))?;
    if revision.undone {
        return Err(ApiError::conflict(
            "already_undone",
            "revision is already undone",
        ));
    }
    // Only the newest revision can be undone (older ones would silently
    // resurrect tasks removed by later revisions).
    let latest = sqlx::query!(
        r#"SELECT r.id FROM plan_revisions r JOIN plans p ON p.id = r.plan_id
           WHERE p.user_id = $1 AND p.plan_date = CURRENT_DATE
           ORDER BY r.created_at DESC LIMIT 1"#,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    if latest.map(|l| l.id) != Some(revision.id) {
        return Err(ApiError::conflict(
            "not_latest_revision",
            "only the most recent revision can be undone",
        ));
    }

    let (new_plan_id, new_version) = agent::fork_plan(&state.pool, plan.id, plan.version).await?;
    sqlx::query!(
        "DELETE FROM plan_tasks WHERE plan_id = $1 AND added_by_revision = $2",
        new_plan_id,
        rid
    )
    .execute(&state.pool)
    .await?;
    agent::mark_revision_undone(&state.pool, rid).await?;
    Ok(Json(UndoResponse {
        plan_version: new_version,
    }))
}
