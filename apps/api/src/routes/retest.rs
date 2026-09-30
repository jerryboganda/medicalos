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

#[derive(Deserialize)]
pub struct RetestResultReq {
    pub question_version_id: Uuid,
    pub session_id: Option<Uuid>,
    pub item_index: Option<i16>,
    pub idempotency_key: String,
}

/// Grade a submitted practice answer on the server. One attempt supplies
/// one durable receipt; the receipt and scheduler update commit together.
pub async fn retest_result(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<RetestResultReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let (Some(session_id), Some(item_index)) = (req.session_id, req.item_index) else {
        return Err(ApiError::unprocessable(
            "retest_evidence_required",
            "identify a submitted practice session and item",
        ));
    };
    let key = req.idempotency_key.trim();
    if key.is_empty() || key.len() > 200 || item_index < 0 {
        return Err(ApiError::unprocessable(
            "invalid_retest_request",
            "use a nonnegative item index and an idempotency key of 1-200 characters",
        ));
    }
    let raw_intervals = crate::routes::settings::current_i64_list(
        &state.pool,
        "retest_intervals_days",
        &crate::routes::settings::DEFAULT_RETEST_INTERVAL_DAYS,
    )
    .await?;
    let intervals = crate::routes::settings::effective_retest_intervals(raw_intervals);
    let mut tx = state.pool.begin().await?;
    // Same session-then-user order as practice submission. The user lock
    // serializes different evidence/keys without blocking FK key-share reads.
    let session = sqlx::query!(
        "SELECT status, preset, submitted_at FROM practice_sessions
         WHERE id = $1 AND user_id = $2 FOR NO KEY UPDATE",
        session_id,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("retest_evidence_not_found"))?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL FOR NO KEY UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    let submitted_at = session
        .submitted_at
        .filter(|_| session.status == "submitted");
    let Some(submitted_at) = submitted_at else {
        return Err(ApiError::conflict(
            "retest_session_not_submitted",
            "submit the practice session before recording a re-test",
        ));
    };
    if !matches!(session.preset.as_str(), "tutor" | "timed" | "revision") {
        return Err(ApiError::unprocessable(
            "invalid_retest_session",
            "re-test evidence must come from practice, not an examination",
        ));
    }
    let replay = sqlx::query!(
        r#"SELECT rh.question_version_id, rh.result_payload,
                  a.session_id AS "session_id?", a.item_index AS "item_index?"
           FROM retest_history rh LEFT JOIN attempts a ON a.id = rh.attempt_id
           WHERE rh.user_id = $1 AND rh.idempotency_key = $2
           ORDER BY rh.created_at DESC LIMIT 1"#,
        user.user_id,
        key
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(replay) = replay {
        if replay.question_version_id != req.question_version_id
            || replay.session_id != Some(session_id)
            || replay.item_index != Some(item_index)
        {
            return Err(ApiError::conflict(
                "retest_key_conflict",
                "this key already identifies different or legacy evidence",
            ));
        }
        let mut result = replay.result_payload.ok_or_else(ApiError::internal)?;
        result["already_recorded"] = json!(true);
        tx.commit().await?;
        return Ok(Json(result));
    }

    let attempt = sqlx::query!(
        r#"SELECT a.id, a.question_version_id, a.chosen_index, a.confidence,
                  (a.assisted OR si.hint_used) AS "assisted!", qv.correct_index
           FROM attempts a
           JOIN session_items si ON si.session_id = a.session_id AND si.item_index = a.item_index
           JOIN question_versions qv ON qv.id = a.question_version_id
           JOIN questions q ON q.id = qv.question_id
           JOIN question_versions card ON card.id = $3
           JOIN questions cq ON cq.id = card.question_id
           WHERE a.user_id = $1 AND a.session_id = $2 AND a.item_index = $4
             AND qv.status = 'published' AND card.status = 'published'
             AND (qv.id = card.id OR (q.family_id IS NOT NULL AND q.family_id = cq.family_id))
           FOR SHARE OF a"#,
        user.user_id,
        session_id,
        req.question_version_id,
        item_index
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("retest_evidence_not_found"))?;
    let used = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM retest_history WHERE attempt_id = $1) AS "used!""#,
        attempt.id
    )
    .fetch_one(&mut *tx)
    .await?;
    if used {
        return Err(ApiError::conflict(
            "retest_attempt_used",
            "this answer already supplied a re-test receipt",
        ));
    }
    let existing = sqlx::query!(
        "SELECT passes, updated_at, enrolled_session_id FROM retest_cards
         WHERE user_id = $1 AND question_version_id = $2 FOR UPDATE",
        user.user_id,
        req.question_version_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if existing.as_ref().is_some_and(|card| {
        card.updated_at > submitted_at && card.enrolled_session_id != Some(session_id)
    }) {
        return Err(ApiError::conflict(
            "retest_evidence_obsolete",
            "submit fresh evidence after the current re-test card update",
        ));
    }
    let correct = attempt.chosen_index == Some(attempt.correct_index);
    let rating = if correct && !attempt.assisted {
        if attempt.confidence.as_deref() == Some("sure") {
            "good"
        } else {
            "hard"
        }
    } else {
        "again"
    };
    // Legacy client-reported passes do not seed a trusted receipt chain.
    let passes_before = existing
        .filter(|r| r.enrolled_session_id.is_some())
        .map(|r| r.passes.max(0))
        .unwrap_or(0);
    let (passes, due_days) = if rating == "good" {
        let p = passes_before.saturating_add(1).min(intervals.len() as i32);
        (p, intervals[(p - 1) as usize])
    } else if rating == "hard" {
        (passes_before, 1)
    } else {
        (0, 1)
    };
    let now = sqlx::query_scalar!(r#"SELECT clock_timestamp() AS "now!""#)
        .fetch_one(&mut *tx)
        .await?;
    let due = now + chrono::Duration::days(due_days);
    let result = json!({
        "already_recorded": false,
        "card_version_id": req.question_version_id,
        "question_version_id": attempt.question_version_id,
        "correct": correct,
        "rating": rating,
        "passes": passes,
        "due": due,
    });

    sqlx::query!(
        "INSERT INTO retest_cards (user_id, question_version_id, passes, due, updated_at, enrolled_session_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (user_id, question_version_id) DO UPDATE SET
           passes = $3, due = $4, updated_at = $5,
           enrolled_session_id = COALESCE(retest_cards.enrolled_session_id, EXCLUDED.enrolled_session_id)",
        user.user_id,
        req.question_version_id,
        passes,
        due,
        now,
        session_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO retest_history (id, user_id, question_version_id, correct, idempotency_key, attempt_id, result_payload)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        Uuid::new_v4(),
        user.user_id,
        req.question_version_id,
        correct,
        key,
        attempt.id,
        result
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(result))
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
           WHERE rc.user_id = $1 AND rc.due <= $2 AND qv.status = 'published'
           ORDER BY rc.due"#,
        user.user_id,
        now
    )
    .fetch_all(&state.pool)
    .await?;
    // QB-02/SR-08 (§13): prefer an unattempted published sibling variant.
    // The base query only returns published cards, so the fallback is safe.
    let mut items: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
    for r in rows {
        let variant = sqlx::query!(
            r#"SELECT qv.id, qv.vignette
               FROM question_versions qv
               JOIN questions q ON q.id = qv.question_id
               WHERE q.family_id = (
                       SELECT q2.family_id FROM questions q2
                       JOIN question_versions qv2 ON qv2.question_id = q2.id
                       WHERE qv2.id = $1)
                 AND qv.id <> $1 AND qv.status = 'published'
                 AND NOT EXISTS (
                     SELECT 1 FROM attempts a
                     WHERE a.question_version_id = qv.id AND a.user_id = $2)
                 AND NOT EXISTS (
                     SELECT 1 FROM reserved_questions rq
                     WHERE rq.question_version_id = qv.id)
               ORDER BY qv.version
               LIMIT 1"#,
            r.question_version_id,
            user.user_id
        )
        .fetch_optional(&state.pool)
        .await?;
        let (served_id, served_vignette, swapped) = match &variant {
            Some(v) => (v.id, v.vignette.clone(), true),
            None => (r.question_version_id, r.vignette.clone(), false),
        };
        items.push(json!({
            "question_version_id": served_id,
            "vignette": served_vignette,
            "passes": r.passes,
            "due": r.due,
            "card_version_id": r.question_version_id,
            "served_variant": swapped,
        }));
    }
    Ok(Json(json!({ "retests": items })))
}
