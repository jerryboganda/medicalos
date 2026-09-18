//! SR-01/02/03 at product scope: decks, cards, the capped review queue, and
//! idempotent review events. Scheduling math lives in the shared `scheduler`
//! crate (§20.1) — this layer is authorization, persistence, and honest
//! queue reporting only.

use axum::extract::{Path, State};
use axum::Json;
use scheduler::{build_queue, from_state, to_state, QueueCard, QueueLimits, Rating, Scheduler};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

fn parse_rating(raw: &str) -> ApiResult<Rating> {
    match raw {
        "again" => Ok(Rating::Again),
        "hard" => Ok(Rating::Hard),
        "good" => Ok(Rating::Good),
        "easy" => Ok(Rating::Easy),
        other => Err(ApiError::unprocessable(
            "invalid_rating",
            format!("rating must be again|hard|good|easy, got {other}"),
        )),
    }
}

#[derive(Deserialize)]
pub struct CreateDeckReq {
    pub name: String,
}

pub async fn create_deck(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CreateDeckReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(ApiError::unprocessable(
            "invalid_name",
            "deck name must be 1-120 characters",
        ));
    }
    let deck_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO decks (id, user_id, name) VALUES ($1, $2, $3)",
        deck_id,
        user.user_id,
        name
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({ "deck_id": deck_id })))
}

#[derive(Deserialize)]
pub struct AddCardReq {
    pub front: String,
    pub back: String,
}

pub async fn add_card(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(deck_id): Path<Uuid>,
    Json(req): Json<AddCardReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let front = req.front.trim();
    let back = req.back.trim();
    if front.is_empty() || back.is_empty() || front.len() > 5000 || back.len() > 5000 {
        return Err(ApiError::unprocessable(
            "invalid_card",
            "front and back must be 1-5000 characters",
        ));
    }
    let owns = sqlx::query!(
        "SELECT 1 AS one FROM decks WHERE id = $1 AND user_id = $2",
        deck_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("deck_not_found"))?;
    let _ = owns;

    let scheduler = Scheduler::new();
    let state_json =
        serde_json::to_value(to_state(&scheduler.new_card())).map_err(|_| ApiError::internal())?;
    let card_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO cards (id, deck_id, user_id, front, back, state)
         VALUES ($1, $2, $3, $4, $5, $6)",
        card_id,
        deck_id,
        user.user_id,
        front,
        back,
        state_json
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(
        serde_json::json!({ "card_id": card_id, "due": null, "state": "new" }),
    ))
}

/// Today's review queue: due cards first (most-at-risk-first), then new
/// cards, under the SR-02 daily caps. The client never counts — it renders
/// what this endpoint returns.
pub async fn queue(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now();
    let due_rows = sqlx::query!(
        r#"SELECT id, front, back, state FROM cards
           WHERE user_id = $1 AND suspended = false
             AND (state->>'due')::timestamptz <= $2
           ORDER BY (state->>'due')::timestamptz"#,
        user.user_id,
        now
    )
    .fetch_all(&state.pool)
    .await?;
    let new_rows = sqlx::query!(
        r#"SELECT id, front, back, state FROM cards
           WHERE user_id = $1 AND suspended = false
             AND (state->>'state')::int = 0
           ORDER BY created_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;

    let due_cards: Vec<QueueCard> = due_rows
        .into_iter()
        .map(|r| {
            Ok(QueueCard {
                id: r.id.to_string(),
                card: from_state(
                    &serde_json::from_value(r.state).map_err(|_| ApiError::internal())?,
                ),
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    let new_cards: Vec<QueueCard> = new_rows
        .into_iter()
        .map(|r| {
            Ok(QueueCard {
                id: r.id.to_string(),
                card: from_state(
                    &serde_json::from_value(r.state).map_err(|_| ApiError::internal())?,
                ),
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;

    let queue = build_queue(QueueLimits::default(), due_cards, new_cards);
    let front_back = |id: &str| -> Option<(String, String)> {
        due_rows
            .iter()
            .find(|r| r.id.to_string() == id)
            .map(|r| (r.front.clone(), r.back.clone()))
            .or_else(|| {
                new_rows
                    .iter()
                    .find(|r| r.id.to_string() == id)
                    .map(|r| (r.front.clone(), r.back.clone()))
            })
    };
    let render = |cards: &[QueueCard]| -> Vec<serde_json::Value> {
        cards
            .iter()
            .filter_map(|qc| {
                front_back(&qc.id).map(|(front, back)| {
                    serde_json::json!({
                        "card_id": qc.id,
                        "front": front,
                        "back": back,
                    })
                })
            })
            .collect()
    };

    Ok(Json(serde_json::json!({
        "due": render(&queue.due),
        "new": render(&queue.new),
        "backlog_remaining": queue.backlog_remaining,
    })))
}

#[derive(Deserialize)]
pub struct ReviewEventReq {
    pub card_id: Uuid,
    pub rating: String,
    pub idempotency_key: String,
}

/// POST /v1/reviews/events (§21.2): apply one review through the shared
/// scheduler, persist the new state and the evidence. Idempotent by key.
pub async fn review_event(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<ReviewEventReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let rating = parse_rating(&req.rating)?;
    let reviewed_at = chrono::Utc::now();

    // Idempotent replay: same key on the same card returns the stored outcome.
    let replay = sqlx::query!(
        "SELECT reviewed_at FROM review_events
         WHERE card_id = $1 AND user_id = $2 AND idempotency_key = $3",
        req.card_id,
        user.user_id,
        req.idempotency_key
    )
    .fetch_optional(&state.pool)
    .await?;
    if let Some(event) = replay {
        let card = sqlx::query!(
            "SELECT state FROM cards WHERE id = $1 AND user_id = $2",
            req.card_id,
            user.user_id
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("card_not_found"))?;
        let cs: scheduler::CardState =
            serde_json::from_value(card.state).map_err(|_| ApiError::internal())?;
        return Ok(Json(serde_json::json!({
            "already_recorded": true,
            "due": cs.due,
            "reviewed_at": event.reviewed_at,
        })));
    }

    let row = sqlx::query!(
        "SELECT state FROM cards WHERE id = $1 AND user_id = $2 AND suspended = false",
        req.card_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("card_not_found"))?;
    let current: scheduler::CardState =
        serde_json::from_value(row.state).map_err(|_| ApiError::internal())?;

    let scheduler = Scheduler::new();
    let next = scheduler.review(from_state(&current), rating, reviewed_at);
    let next_state = to_state(&next);
    let state_json = serde_json::to_value(&next_state).map_err(|_| ApiError::internal())?;

    let inserted = sqlx::query!(
        "INSERT INTO review_events (id, card_id, user_id, rating, reviewed_at, idempotency_key)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT DO NOTHING
         RETURNING id",
        Uuid::new_v4(),
        req.card_id,
        user.user_id,
        req.rating,
        reviewed_at,
        req.idempotency_key
    )
    .fetch_optional(&state.pool)
    .await?;
    if inserted.is_none() {
        // Lost a race with the same key — treat as replay.
        return Ok(Json(serde_json::json!({
            "already_recorded": true,
            "due": next_state.due,
            "reviewed_at": reviewed_at,
        })));
    }
    sqlx::query!(
        "UPDATE cards SET state = $2 WHERE id = $1",
        req.card_id,
        state_json
    )
    .execute(&state.pool)
    .await?;

    Ok(Json(serde_json::json!({
        "already_recorded": false,
        "due": next_state.due,
        "reviewed_at": reviewed_at,
    })))
}
