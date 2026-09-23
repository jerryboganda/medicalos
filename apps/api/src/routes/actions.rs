//! QB-17 completion: session result actions — retry same session,
//! practice-incorrect-only, and per-chapter weak-chapter targeting.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ResultActionReq {
    pub action: String, // retry | practice_incorrect
}

/// POST /v1/sessions/{sid}/action — creates a follow-up session from the
/// result actions of §11.9 (retry, practice incorrect, practice weak).
pub async fn session_action(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(sid): Path<Uuid>,
    Json(req): Json<ResultActionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let session = sqlx::query!(
        "SELECT preset, chapter_id, status FROM practice_sessions
         WHERE id = $1 AND user_id = $2",
        sid,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("session_not_found"))?;
    if session.status != "submitted" {
        return Err(ApiError::conflict(
            "not_submitted",
            "session must be submitted before result actions",
        ));
    }

    match req.action.as_str() {
        "retry" => {
            // Same chapter, fresh session.
            let chapter = session.chapter_id.ok_or_else(|| {
                ApiError::unprocessable("no_chapter", "session has no chapter context")
            })?;
            let items = sqlx::query!(
                r#"SELECT id, vignette, lead_in, difficulty, options
                   FROM question_versions
                   WHERE status = 'published' AND chapter_id = $1
                   ORDER BY random() LIMIT 10"#,
                chapter
            )
            .fetch_all(&state.pool)
            .await?;
            if items.is_empty() {
                return Err(ApiError::unprocessable(
                    "empty_pool",
                    "No questions available.",
                ));
            }
            let new_sid = Uuid::new_v4();
            sqlx::query!(
                "INSERT INTO practice_sessions (id, user_id, preset, chapter_id)
                 VALUES ($1, $2, 'tutor', $3)",
                new_sid,
                user.user_id,
                chapter
            )
            .execute(&state.pool)
            .await?;
            for (i, r) in items.iter().enumerate() {
                let idx = i as i16;
                sqlx::query!(
                    "INSERT INTO session_items (id, session_id, item_index, question_version_id)
                     VALUES ($1, $2, $3, $4)",
                    Uuid::new_v4(),
                    new_sid,
                    idx,
                    r.id
                )
                .execute(&state.pool)
                .await?;
            }
            Ok(Json(
                json!({"session_id": new_sid, "item_count": items.len()}),
            ))
        }
        "practice_incorrect" => {
            // Same as revision: incorrect + skipped only.
            let src = sid;
            let pool_qs = sqlx::query!(
                r#"SELECT DISTINCT qv.id FROM question_versions qv
                   WHERE qv.status = 'published' AND (qv.id IN (
                       SELECT question_version_id FROM attempts
                       WHERE session_id = $1 AND correct = FALSE
                   )
                   OR qv.id IN (
                       SELECT si.question_version_id FROM session_items si
                       WHERE si.session_id = $1 AND NOT EXISTS (
                           SELECT 1 FROM attempts a
                           WHERE a.session_id = si.session_id AND a.item_index = si.item_index
                       )
                   ))"#,
                src
            )
            .fetch_all(&state.pool)
            .await?;
            if pool_qs.is_empty() {
                return Err(ApiError::unprocessable(
                    "nothing_to_practice",
                    "No incorrect or skipped questions in that session.",
                ));
            }
            let new_sid = Uuid::new_v4();
            sqlx::query!(
                "INSERT INTO practice_sessions (id, user_id, preset, source_session_id)
                 VALUES ($1, $2, 'revision', $3)",
                new_sid,
                user.user_id,
                src
            )
            .execute(&state.pool)
            .await?;
            for (i, r) in pool_qs.iter().enumerate() {
                let idx = i as i16;
                sqlx::query!(
                    "INSERT INTO session_items (id, session_id, item_index, question_version_id)
                     VALUES ($1, $2, $3, $4)",
                    Uuid::new_v4(),
                    new_sid,
                    idx,
                    r.id
                )
                .execute(&state.pool)
                .await?;
            }
            Ok(Json(
                json!({"session_id": new_sid, "item_count": pool_qs.len()}),
            ))
        }
        _ => Err(ApiError::unprocessable(
            "unknown_action",
            format!("unknown action {}", req.action),
        )),
    }
}
