//! CORE-02: learner-owned, versioned study capacity and protected dates.

use axum::extract::State;
use axum::Json;
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const MAX_COMMITMENT_TITLE_LEN: usize = 120;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProtectedCommitment {
    title: String,
    date: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct GoalUpdateRequest {
    expected_version: i32,
    daily_minutes: Option<i32>,
    exam_date: Option<NaiveDate>,
    #[serde(default)]
    protected_commitments: Vec<ProtectedCommitment>,
}

#[derive(Debug, Deserialize)]
pub struct GoalUndoRequest {
    expected_version: i32,
}

#[derive(Debug, Serialize)]
pub struct GoalResponse {
    version: i32,
    daily_minutes: Option<i32>,
    exam_date: Option<NaiveDate>,
    protected_commitments: Vec<ProtectedCommitment>,
    can_undo: bool,
    changed: bool,
}

fn normalize(input: GoalUpdateRequest) -> ApiResult<GoalUpdateRequest> {
    if input.expected_version < 0 {
        return Err(ApiError::unprocessable(
            "invalid_goal_version",
            "expected_version must be zero or greater",
        ));
    }
    if matches!(input.daily_minutes, Some(minutes) if !(1..=1440).contains(&minutes)) {
        return Err(ApiError::unprocessable(
            "invalid_daily_minutes",
            "daily_minutes must be between 1 and 1440",
        ));
    }

    let today = Utc::now().date_naive();
    if matches!(input.exam_date, Some(date) if date < today) {
        return Err(ApiError::unprocessable(
            "invalid_exam_date",
            "exam_date cannot be in the past",
        ));
    }

    let protected_commitments = input
        .protected_commitments
        .into_iter()
        .map(|commitment| {
            let title = commitment.title.trim().to_owned();
            if title.is_empty() || title.chars().count() > MAX_COMMITMENT_TITLE_LEN {
                return Err(ApiError::unprocessable(
                    "invalid_commitment_title",
                    "commitment title must be between 1 and 120 characters",
                ));
            }
            if commitment.date < today {
                return Err(ApiError::unprocessable(
                    "invalid_commitment_date",
                    "commitment date cannot be in the past",
                ));
            }
            Ok(ProtectedCommitment {
                title,
                date: commitment.date,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(GoalUpdateRequest {
        protected_commitments,
        ..input
    })
}

fn response(
    version: i32,
    daily_minutes: Option<i32>,
    exam_date: Option<NaiveDate>,
    protected_commitments: Vec<ProtectedCommitment>,
    changed: bool,
) -> GoalResponse {
    GoalResponse {
        version,
        daily_minutes,
        exam_date,
        protected_commitments,
        can_undo: version > 1,
        changed,
    }
}

pub async fn get_goals(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<GoalResponse>> {
    let current = sqlx::query!(
        "SELECT version, daily_minutes, exam_date, protected_commitments
         FROM learner_goal_versions
         WHERE user_id = $1
         ORDER BY version DESC
         LIMIT 1",
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;

    let Some(current) = current else {
        return Ok(Json(response(0, None, None, Vec::new(), false)));
    };
    let protected_commitments =
        serde_json::from_value(current.protected_commitments).map_err(|_| ApiError::internal())?;
    Ok(Json(response(
        current.version,
        current.daily_minutes,
        current.exam_date,
        protected_commitments,
        false,
    )))
}

pub async fn put_goals(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(input): Json<GoalUpdateRequest>,
) -> ApiResult<Json<GoalResponse>> {
    let input = normalize(input)?;
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
        "SELECT version, daily_minutes, exam_date, protected_commitments
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
                false,
            )));
        }
    } else if input.daily_minutes.is_none()
        && input.exam_date.is_none()
        && input.protected_commitments.is_empty()
    {
        tx.commit().await?;
        return Ok(Json(response(0, None, None, Vec::new(), false)));
    }

    let next_version = current_version + 1;
    sqlx::query!(
        "INSERT INTO learner_goal_versions
             (user_id, version, daily_minutes, exam_date, protected_commitments)
         VALUES ($1, $2, $3, $4, $5)",
        user.user_id,
        next_version,
        input.daily_minutes,
        input.exam_date,
        protected_json
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(response(
        next_version,
        input.daily_minutes,
        input.exam_date,
        input.protected_commitments,
        true,
    )))
}

pub async fn undo_goals(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(input): Json<GoalUndoRequest>,
) -> ApiResult<Json<GoalResponse>> {
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
    sqlx::query!(
        "INSERT INTO learner_goal_versions
             (user_id, version, daily_minutes, exam_date, protected_commitments)
         VALUES ($1, $2, $3, $4, $5)",
        user.user_id,
        next_version,
        previous.daily_minutes,
        previous.exam_date,
        previous.protected_commitments
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(response(
        next_version,
        previous.daily_minutes,
        previous.exam_date,
        protected_commitments,
        true,
    )))
}
