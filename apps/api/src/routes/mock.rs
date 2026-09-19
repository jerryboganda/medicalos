//! EX-07: administrator-configured mock tests. Configuration is admin-token
//! gated until the editorial console (ADMIN-06) lands; learners get list +
//! start. The form is frozen at start (EX-03): questions are chosen against
//! the blueprint and snapshotted into session_items — later content edits
//! cannot alter a started mock.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::routes::practice::PoolQuestion;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateMockReq {
    pub title: String,
    pub exam_id: Uuid,
    pub blueprint: Vec<BlueprintEntry>,
    pub time_limit_seconds: Option<i64>,
    pub pass_mark_percent: Option<i32>,
    pub attempts_allowed: Option<i32>,
}

#[derive(Deserialize)]
pub struct BlueprintEntry {
    pub chapter_id: Uuid,
    pub count: i32,
}

fn require_admin(state: &AppState, token: Option<&str>) -> ApiResult<()> {
    match (&state.admin_token, token) {
        (Some(expected), Some(provided)) if expected == provided => Ok(()),
        (Some(_), _) => Err(ApiError::forbidden(
            "admin_required",
            "mock configuration requires the admin token",
        )),
        (None, _) => Err(ApiError::forbidden(
            "admin_disabled",
            "mock configuration is disabled (no ADMIN_TOKEN configured)",
        )),
    }
}

pub async fn create_mock(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateMockReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    require_admin(&state, provided)?;
    let title = req.title.trim();
    if title.is_empty() || title.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_title",
            "title must be 1-200 characters",
        ));
    }
    if req.blueprint.is_empty() || !req.blueprint.iter().all(|e| e.count >= 1 && e.count <= 200) {
        return Err(ApiError::unprocessable(
            "invalid_blueprint",
            "blueprint needs 1-200 questions per chapter entry",
        ));
    }
    let pass_mark = req.pass_mark_percent.unwrap_or(50).clamp(1, 100);
    let attempts = req.attempts_allowed.unwrap_or(1).clamp(1, 10);
    if let Some(limit) = req.time_limit_seconds {
        if !(60..=28_800).contains(&limit) {
            return Err(ApiError::unprocessable(
                "time_limit_out_of_range",
                "mock time_limit_seconds must be 60..=28800",
            ));
        }
    }
    let blueprint = serde_json::to_value(&req.blueprint).map_err(|_| ApiError::internal())?;
    let mock_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO mocks
           (id, title, exam_id, blueprint, time_limit_seconds, pass_mark_percent,
            attempts_allowed, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        mock_id,
        title,
        req.exam_id,
        blueprint,
        req.time_limit_seconds,
        pass_mark,
        attempts,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({ "mock_id": mock_id })))
}

pub async fn list_mocks(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT m.id, m.title, m.pass_mark_percent, m.attempts_allowed,
                  m.time_limit_seconds,
                  (SELECT COUNT(*) FROM mock_attempts ma
                   WHERE ma.mock_id = m.id AND ma.user_id = $1) AS "used!"
           FROM mocks m ORDER BY m.created_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let mocks: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "mock_id": m.id,
                "title": m.title,
                "pass_mark_percent": m.pass_mark_percent,
                "attempts_allowed": m.attempts_allowed,
                "attempts_used": m.used,
                "time_limit_seconds": m.time_limit_seconds,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "mocks": mocks })))
}

/// Freeze the form (EX-03): pick against the blueprint once, snapshot the
/// items — then the standard answer/submit pipeline takes over with feedback
/// deferred (§11.2 exam-style) and the §11.3 server-issued timer.
pub async fn start_mock(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(mid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let mock = sqlx::query!(
        "SELECT id, blueprint, time_limit_seconds FROM mocks WHERE id = $1",
        mid
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("mock_not_found"))?;

    let used = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM mock_attempts
           WHERE mock_id = $1 AND user_id = $2"#,
        mid,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    let allowed = sqlx::query!("SELECT attempts_allowed FROM mocks WHERE id = $1", mid)
        .fetch_one(&state.pool)
        .await?
        .attempts_allowed;
    if used >= allowed as i64 {
        return Err(ApiError::conflict(
            "attempts_exhausted",
            format!("all {} attempts for this mock are used", allowed),
        ));
    }

    let blueprint: Vec<BlueprintEntry> =
        serde_json::from_value(mock.blueprint).map_err(|_| ApiError::internal())?;
    let mut pool_questions: Vec<PoolQuestion> = Vec::new();
    for entry in &blueprint {
        let count = entry.count.clamp(0, 200) as i64;
        let rows = sqlx::query!(
            r#"SELECT id, vignette, lead_in, difficulty, options
               FROM question_versions
               WHERE status = 'published' AND chapter_id = $1
               ORDER BY random() LIMIT $2"#,
            entry.chapter_id,
            count
        )
        .fetch_all(&state.pool)
        .await?;
        if rows.len() < count as usize {
            return Err(ApiError::unprocessable(
                "insufficient_questions",
                format!(
                    "blueprint needs {} questions in one chapter, only {} are available",
                    count,
                    rows.len()
                ),
            ));
        }
        for r in rows {
            pool_questions.push(PoolQuestion {
                id: r.id,
                vignette: r.vignette,
                lead_in: r.lead_in,
                difficulty: r.difficulty,
                options: r.options,
            });
        }
    }

    let deadline = mock
        .time_limit_seconds
        .map(|limit| chrono::Utc::now() + chrono::Duration::seconds(limit as i64));
    let sid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO practice_sessions
           (id, user_id, preset, mock_id, time_limit_seconds, deadline)
         VALUES ($1, $2, 'mock', $3, $4, $5)",
        sid,
        user.user_id,
        mid,
        mock.time_limit_seconds,
        deadline
    )
    .execute(&state.pool)
    .await?;
    for (i, q) in pool_questions.iter().enumerate() {
        let idx = i as i16;
        sqlx::query!(
            "INSERT INTO session_items (id, session_id, item_index, question_version_id)
             VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            sid,
            idx,
            q.id
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(serde_json::json!({
        "session_id": sid,
        "question_count": pool_questions.len(),
    })))
}
