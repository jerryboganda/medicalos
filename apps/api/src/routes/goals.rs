//! CORE-02: versioned goals with protected commitments. Protected
//! commitments are NEVER agent-mutable (§9.3) — this module is the only
//! writer and it is the learner.

use axum::extract::State;
use axum::Json;
use chrono::DateTime;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct GoalReq {
    pub exam_id: Option<Uuid>,
    pub target_note: String,
    pub exam_date: Option<chrono::NaiveDate>,
}

pub async fn create_goal(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<GoalReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let target = req.target_note.trim();
    if target.is_empty() || target.len() > 500 {
        return Err(ApiError::unprocessable(
            "invalid_goal",
            "goal must be 1-500 characters",
        ));
    }
    // One active goal: the previous one is retired (history retained).
    sqlx::query!(
        "UPDATE goals SET retired = true WHERE user_id = $1 AND NOT retired",
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO goals (id, user_id, exam_id, target_note, exam_date)
         VALUES ($1, $2, $3, $4, $5)",
        id,
        user.user_id,
        req.exam_id,
        target,
        req.exam_date
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "goal_id": id })))
}

pub async fn list_goals(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let goals = sqlx::query!(
        r#"SELECT id, target_note, exam_date, version, retired, created_at
           FROM goals WHERE user_id = $1 ORDER BY created_at DESC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let commitments = sqlx::query!(
        r#"SELECT id, label, starts_at, ends_at FROM protected_commitments
           WHERE user_id = $1 ORDER BY starts_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "goals": goals.iter().map(|g| json!({
            "goal_id": g.id,
            "target_note": g.target_note,
            "exam_date": g.exam_date,
            "version": g.version,
            "retired": g.retired,
            "created_at": g.created_at,
        })).collect::<Vec<_>>(),
        "protected_commitments": commitments.iter().map(|c| json!({
            "commitment_id": c.id,
            "label": c.label,
            "starts_at": c.starts_at,
            "ends_at": c.ends_at,
        })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub struct CommitmentReq {
    pub label: String,
    pub starts_at: DateTime<chrono::Utc>,
    pub ends_at: DateTime<chrono::Utc>,
}

pub async fn add_commitment(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CommitmentReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let label = req.label.trim();
    if label.is_empty() || label.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_label",
            "label must be 1-200 characters",
        ));
    }
    if req.ends_at <= req.starts_at {
        return Err(ApiError::unprocessable(
            "invalid_window",
            "commitment must end after it starts",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO protected_commitments (id, user_id, label, starts_at, ends_at)
         VALUES ($1, $2, $3, $4, $5)",
        id,
        user.user_id,
        label,
        req.starts_at,
        req.ends_at
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "commitment_id": id })))
}

pub async fn remove_commitment(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let deleted = sqlx::query!(
        "DELETE FROM protected_commitments WHERE id = $1 AND user_id = $2",
        id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    if deleted.rows_affected() == 0 {
        return Err(ApiError::not_found("commitment_not_found"));
    }
    Ok(Json(json!({ "removed": true })))
}
