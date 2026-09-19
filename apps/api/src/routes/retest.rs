//! NOTE-02 collections + concepts, and SR-08 re-test queue scheduling.
//! Re-tests use the shared scheduler crate: wrong answers schedule a near
//! re-test (family-variant preferred at selection time, §13); successes
//! space out deterministically. All queries are user-scoped.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ---- NOTE-02: collections ----------------------------------------------------

#[derive(Deserialize)]
pub struct CollectionReq {
    pub name: String,
}

pub async fn create_collection(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CollectionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(ApiError::unprocessable(
            "invalid_name",
            "collection name must be 1-120 characters",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO note_collections (id, user_id, name) VALUES ($1, $2, $3)",
        id,
        user.user_id,
        name
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "collection_id": id })))
}

pub async fn list_collections(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT c.id, c.name,
                  COALESCE(COUNT(ci.note_id), 0) AS "note_count!"
           FROM note_collections c
           LEFT JOIN note_collection_items ci ON ci.collection_id = c.id
           WHERE c.user_id = $1
           GROUP BY c.id, c.name ORDER BY c.name"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let collections: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| json!({"collection_id": r.id, "name": r.name, "note_count": r.note_count}))
        .collect();
    Ok(Json(json!({ "collections": collections })))
}

#[derive(Deserialize)]
pub struct AddToCollectionReq {
    pub note_id: Uuid,
}

pub async fn add_note_to_collection(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(collection_id): Path<Uuid>,
    Json(req): Json<AddToCollectionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    // Both the collection and the note must belong to the caller.
    let owns = sqlx::query!(
        r#"SELECT
             (SELECT COALESCE(COUNT(*), 0) FROM note_collections
              WHERE id = $1 AND user_id = $2) AS "c!",
             (SELECT COALESCE(COUNT(*), 0) FROM notes
              WHERE id = $3 AND user_id = $2) AS "n!""#,
        collection_id,
        user.user_id,
        req.note_id
    )
    .fetch_one(&state.pool)
    .await?;
    if owns.c == 0 || owns.n == 0 {
        return Err(ApiError::not_found("not_found"));
    }
    sqlx::query!(
        "INSERT INTO note_collection_items (collection_id, note_id)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
        collection_id,
        req.note_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "added": true })))
}

// ---- NOTE-02: concepts --------------------------------------------------------

#[derive(Deserialize)]
pub struct ConceptReq {
    pub concept: String,
}

pub async fn tag_note_concept(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(note_id): Path<Uuid>,
    Json(req): Json<ConceptReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let concept = req.concept.trim().to_lowercase();
    if concept.is_empty() || concept.len() > 80 {
        return Err(ApiError::unprocessable(
            "invalid_concept",
            "concept must be 1-80 characters",
        ));
    }
    let owns = sqlx::query!(
        "SELECT 1 AS one FROM notes WHERE id = $1 AND user_id = $2",
        note_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("note_not_found"))?;
    let _ = owns;
    sqlx::query!(
        "INSERT INTO note_concepts (note_id, concept) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
        note_id,
        concept
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "tagged": concept })))
}

pub async fn notes_by_concept(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(concept): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT n.id, n.title, n.body FROM notes n
           JOIN note_concepts nc ON nc.note_id = n.id
           WHERE n.user_id = $1 AND nc.concept = $2
           ORDER BY n.updated_at DESC"#,
        user.user_id,
        concept.to_lowercase()
    )
    .fetch_all(&state.pool)
    .await?;
    let notes: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| json!({"note_id": r.id, "title": r.title, "body": r.body}))
        .collect();
    Ok(Json(json!({ "concept": concept, "notes": notes })))
}

// ---- SR-08: re-test queue ------------------------------------------------------

/// Intervals (days) per successive re-test pass, §13 revision-tracking:
/// wrong→wrong→correct→correct gradually contributing to mastery.
const RETEST_INTERVAL_DAYS: [i64; 4] = [1, 3, 7, 14];

#[derive(Deserialize)]
pub struct RetestResultReq {
    pub question_version_id: Uuid,
    pub correct: bool,
    pub idempotency_key: String,
}

/// Record a re-test outcome: wrong compresses the next interval, correct
/// extends it deterministically. Idempotent by key.
pub async fn retest_result(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<RetestResultReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let replay = sqlx::query!(
        "SELECT id FROM retest_history
         WHERE user_id = $1 AND idempotency_key = $2",
        user.user_id,
        req.idempotency_key
    )
    .fetch_optional(&state.pool)
    .await?;
    if replay.is_some() {
        return Ok(Json(json!({ "already_recorded": true })));
    }

    let now = Utc::now();
    let existing = sqlx::query!(
        "SELECT passes FROM retest_cards
         WHERE user_id = $1 AND question_version_id = $2",
        user.user_id,
        req.question_version_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let passes_before = existing.map(|r| r.passes).unwrap_or(0);
    let (passes, due_days) = if req.correct {
        let p = (passes_before + 1).min(RETEST_INTERVAL_DAYS.len() as i32);
        (p, RETEST_INTERVAL_DAYS[(p - 1) as usize])
    } else {
        (0, 1)
    };
    let due = now + chrono::Duration::days(due_days);

    sqlx::query!(
        "INSERT INTO retest_cards (user_id, question_version_id, passes, due, updated_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id, question_version_id) DO UPDATE SET
           passes = $3, due = $4, updated_at = $5",
        user.user_id,
        req.question_version_id,
        passes,
        due,
        now
    )
    .execute(&state.pool)
    .await?;

    sqlx::query!(
        "INSERT INTO retest_history (id, user_id, question_version_id, correct, idempotency_key)
         VALUES ($1, $2, $3, $4, $5)",
        Uuid::new_v4(),
        user.user_id,
        req.question_version_id,
        req.correct,
        req.idempotency_key
    )
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "already_recorded": false,
        "passes": passes,
        "due": due,
    })))
}

/// Due re-tests for the learner: question payload + pass count. §13: prefer
/// the family variant at selection time (handled when serving a session);
/// this list is the scheduler's queue.
pub async fn due_retests(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let now = Utc::now();
    let rows = sqlx::query!(
        r#"SELECT qv.id AS question_version_id, qv.vignette, rc.passes, rc.due
           FROM retest_cards rc
           JOIN question_versions qv ON qv.id = rc.question_version_id
           WHERE rc.user_id = $1 AND rc.due <= $2
           ORDER BY rc.due"#,
        user.user_id,
        now
    )
    .fetch_all(&state.pool)
    .await?;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "question_version_id": r.question_version_id,
                "vignette": r.vignette,
                "passes": r.passes,
                "due": r.due,
            })
        })
        .collect();
    Ok(Json(json!({ "retests": items })))
}
