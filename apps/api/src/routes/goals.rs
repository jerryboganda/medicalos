//! CORE-02: learner-owned, versioned study capacity and protected dates.

use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use domain_contracts::{
    validate_goal_update, validate_goal_version, GoalUpdate, GoalValidationError,
    ProtectedCommitment,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct GoalUndoRequest {
    expected_version: i32,
}

#[derive(Debug, Serialize)]
pub struct GoalResponse {
    version: i32,
    daily_minutes: Option<i32>,
    exam_date: Option<chrono::NaiveDate>,
    protected_commitments: Vec<ProtectedCommitment>,
    created_at: Option<DateTime<Utc>>,
    can_undo: bool,
    changed: bool,
}

fn validation_error(error: GoalValidationError) -> ApiError {
    match error {
        GoalValidationError::InvalidVersion => ApiError::unprocessable(
            "invalid_goal_version",
            "expected_version must be zero or greater",
        ),
        GoalValidationError::InvalidDailyMinutes => ApiError::unprocessable(
            "invalid_daily_minutes",
            "daily_minutes must be between 1 and 1440",
        ),
        GoalValidationError::PastExamDate => {
            ApiError::unprocessable("invalid_exam_date", "exam_date cannot be in the past")
        }
        GoalValidationError::InvalidCommitmentTitle => ApiError::unprocessable(
            "invalid_commitment_title",
            "commitment title must be between 1 and 120 characters",
        ),
        GoalValidationError::PastCommitmentDate => ApiError::unprocessable(
            "invalid_commitment_date",
            "commitment date cannot be in the past",
        ),
    }
}

fn response(
    version: i32,
    daily_minutes: Option<i32>,
    exam_date: Option<chrono::NaiveDate>,
    protected_commitments: Vec<ProtectedCommitment>,
    created_at: Option<DateTime<Utc>>,
    changed: bool,
) -> GoalResponse {
    GoalResponse {
        version,
        daily_minutes,
        exam_date,
        protected_commitments,
        created_at,
        can_undo: version > 1,
        changed,
    }
}

pub async fn get_goals(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<GoalResponse>> {
    let current = sqlx::query!(
        "SELECT version, daily_minutes, exam_date, protected_commitments, created_at
         FROM learner_goal_versions
         WHERE user_id = $1
         ORDER BY version DESC
         LIMIT 1",
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    let Some(current) = current else {
        return Ok(Json(response(0, None, None, Vec::new(), None, false)));
    };
    let protected_commitments =
        serde_json::from_value(current.protected_commitments).map_err(|_| ApiError::internal())?;
    Ok(Json(response(
        current.version,
        current.daily_minutes,
        current.exam_date,
        protected_commitments,
        Some(current.created_at),
        false,
    )))
}

pub async fn put_goals(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(input): Json<GoalUpdate>,
) -> ApiResult<Json<GoalResponse>> {
    let input = validate_goal_update(input, Utc::now().date_naive()).map_err(validation_error)?;
    let protected_json =
        serde_json::to_value(&input.protected_commitments).map_err(|_| ApiError::internal())?;
    let mut tx = state.pool.begin().await?;

    sqlx::query!(
        "SELECT id FROM users WHERE id = $1 FOR UPDATE",
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let current = sqlx::query!(
        "SELECT version, daily_minutes, exam_date, protected_commitments, created_at
         FROM learner_goal_versions
         WHERE user_id = $1
         ORDER BY version DESC
         LIMIT 1",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let current_version = current.as_ref().map_or(0, |row| row.version);
    if input.expected_version != current_version {
        return Err(ApiError::conflict(
            "goal_version_conflict",
            "goals changed since this edit was loaded",
        ));
    }

    if let Some(current) = current {
        if current.daily_minutes == input.daily_minutes
            && current.exam_date == input.exam_date
            && current.protected_commitments == protected_json
        {
            tx.commit().await?;
            return Ok(Json(response(
                current.version,
                current.daily_minutes,
                current.exam_date,
                input.protected_commitments,
                Some(current.created_at),
                false,
            )));
        }
    } else if input.daily_minutes.is_none()
        && input.exam_date.is_none()
        && input.protected_commitments.is_empty()
    {
        tx.commit().await?;
        return Ok(Json(response(0, None, None, Vec::new(), None, false)));
    }

    let next_version = current_version + 1;
    let inserted = sqlx::query!(
        "INSERT INTO learner_goal_versions
             (user_id, version, daily_minutes, exam_date, protected_commitments)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING created_at",
        user.user_id,
        next_version,
        input.daily_minutes,
        input.exam_date,
        protected_json
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(response(
        next_version,
        input.daily_minutes,
        input.exam_date,
        input.protected_commitments,
        Some(inserted.created_at),
        true,
    )))
}

pub async fn undo_goals(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(input): Json<GoalUndoRequest>,
) -> ApiResult<Json<GoalResponse>> {
    validate_goal_version(input.expected_version).map_err(validation_error)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "SELECT id FROM users WHERE id = $1 FOR UPDATE",
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let current = sqlx::query!(
        "SELECT version FROM learner_goal_versions
         WHERE user_id = $1
         ORDER BY version DESC
         LIMIT 1",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let current_version = current.as_ref().map_or(0, |row| row.version);
    if input.expected_version != current_version {
        return Err(ApiError::conflict(
            "goal_version_conflict",
            "goals changed since this edit was loaded",
        ));
    }
    if current_version <= 1 {
        return Err(ApiError::conflict(
            "goal_nothing_to_undo",
            "there is no earlier saved goal version to restore",
        ));
    }

    let previous_version = current_version - 1;
    let previous = sqlx::query!(
        "SELECT daily_minutes, exam_date, protected_commitments
         FROM learner_goal_versions
         WHERE user_id = $1 AND version = $2",
        user.user_id,
        previous_version
    )
    .fetch_one(&mut *tx)
    .await?;
    let protected_commitments = serde_json::from_value(previous.protected_commitments.clone())
        .map_err(|_| ApiError::internal())?;
    let next_version = current_version + 1;
    let inserted = sqlx::query!(
        "INSERT INTO learner_goal_versions
             (user_id, version, daily_minutes, exam_date, protected_commitments)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING created_at",
        user.user_id,
        next_version,
        previous.daily_minutes,
        previous.exam_date,
        previous.protected_commitments
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(response(
        next_version,
        previous.daily_minutes,
        previous.exam_date,
        protected_commitments,
        Some(inserted.created_at),
        true,
    )))
}
