//! SR-01/02/03 at product scope: decks, cards, the capped review queue, and
//! idempotent review events. Scheduling math lives in the shared `scheduler`
//! crate (§20.1) — this layer is authorization, persistence, and honest
//! queue reporting only.

use axum::extract::{Path, State};
use axum::Json;
use scheduler::{build_queue, from_state, to_state, QueueCard, QueueLimits, Rating, Scheduler};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgConnection, Row};
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

async fn daily_review_usage(
    conn: &mut PgConnection,
    user_id: Uuid,
    at: chrono::DateTime<chrono::Utc>,
) -> ApiResult<(i64, i64)> {
    let row = sqlx::query(
        r#"SELECT COUNT(*) FILTER (WHERE was_new) AS new_count,
                  COUNT(*) FILTER (WHERE NOT was_new) AS review_count
           FROM review_events
           WHERE user_id = $1
             AND reviewed_at >= (date_trunc('day', $2::timestamptz AT TIME ZONE 'UTC') AT TIME ZONE 'UTC')
             AND reviewed_at < ((date_trunc('day', $2::timestamptz AT TIME ZONE 'UTC') + interval '1 day') AT TIME ZONE 'UTC')"#,
    )
    .bind(user_id)
    .bind(at)
    .fetch_one(&mut *conn)
    .await?;
    Ok((row.try_get("new_count")?, row.try_get("review_count")?))
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
    /// SR-03: basic (default) | cloze | image.
    pub card_type: Option<String>,
    /// SR-04: editorial (default) | ai_draft. AI-drafted cards carry the
    /// label everywhere they render; they never masquerade as reviewed.
    pub trust: Option<String>,
    /// SR-03 cloze text with {{c1::deletion}} markers; required for cloze.
    pub cloze: Option<String>,
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
    let card_type = req.card_type.unwrap_or_else(|| "basic".into());
    if !matches!(card_type.as_str(), "basic" | "cloze" | "image") {
        return Err(ApiError::unprocessable(
            "invalid_card_type",
            "card_type must be basic, cloze, or image",
        ));
    }
    if card_type == "cloze" {
        let cloze_ok = req
            .cloze
            .as_deref()
            .is_some_and(|c| c.contains("{{c1::") && c.contains("}}"));
        if !cloze_ok {
            return Err(ApiError::unprocessable(
                "invalid_cloze",
                "cloze cards need deletion markers like {{c1::answer}}",
            ));
        }
    }
    let trust = req.trust.unwrap_or_else(|| "editorial".into());
    if !matches!(trust.as_str(), "editorial" | "ai_draft") {
        return Err(ApiError::unprocessable(
            "invalid_trust",
            "trust must be editorial or ai_draft",
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

    // SR-05: a sibling with the same normalized front already exists in this
    // deck — refuse rather than silently grow duplicates.
    let duplicate = sqlx::query!(
        "SELECT 1 AS one FROM cards
         WHERE deck_id = $1 AND lower(btrim(front)) = lower(btrim($2))",
        deck_id,
        front
    )
    .fetch_optional(&state.pool)
    .await?;
    if duplicate.is_some() {
        return Err(ApiError::conflict(
            "duplicate_card",
            "a card with the same front already exists in this deck",
        ));
    }

    let scheduler = Scheduler::new();
    let state_json =
        serde_json::to_value(to_state(&scheduler.new_card())).map_err(|_| ApiError::internal())?;
    let card_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO cards
           (id, deck_id, user_id, front, back, state, card_type, trust, cloze)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        card_id,
        deck_id,
        user.user_id,
        front,
        back,
        state_json,
        card_type,
        trust,
        req.cloze
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({
        "card_id": card_id, "due": null, "state": "new",
        "card_type": card_type, "trust": trust,
    })))
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
        r#"SELECT id, front, back, state, card_type, trust, cloze FROM cards
           WHERE user_id = $1 AND suspended = false
             AND (state->>'state')::int <> 0
             AND (state->>'due')::timestamptz <= $2
           ORDER BY (state->>'due')::timestamptz"#,
        user.user_id,
        now
    )
    .fetch_all(&state.pool)
    .await?;
    let new_rows = sqlx::query!(
        r#"SELECT id, front, back, state, card_type, trust, cloze FROM cards
           WHERE user_id = $1 AND suspended = false
             AND (state->>'state')::int = 0
           ORDER BY created_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;

    // Lookup map built from borrowed rows before they are consumed below.
    struct CardMeta {
        front: String,
        back: String,
        card_type: String,
        trust: String,
        cloze: Option<String>,
    }
    let mut meta: std::collections::HashMap<String, CardMeta> = std::collections::HashMap::new();
    // The two query! records are distinct types - fill the map per query.
    let insert_meta = |meta: &mut std::collections::HashMap<String, CardMeta>,
                       id: &Uuid,
                       front: &String,
                       back: &String,
                       card_type: &String,
                       trust: &String,
                       cloze: &Option<String>| {
        meta.insert(
            id.to_string(),
            CardMeta {
                front: front.clone(),
                back: back.clone(),
                card_type: card_type.clone(),
                trust: trust.clone(),
                cloze: cloze.clone(),
            },
        );
    };
    for r in &due_rows {
        insert_meta(
            &mut meta,
            &r.id,
            &r.front,
            &r.back,
            &r.card_type,
            &r.trust,
            &r.cloze,
        );
    }
    for r in &new_rows {
        insert_meta(
            &mut meta,
            &r.id,
            &r.front,
            &r.back,
            &r.card_type,
            &r.trust,
            &r.cloze,
        );
    }

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

    let max_reviews = crate::routes::settings::current_bounded_i64(
        &state.pool,
        "max_reviews_per_day",
        crate::routes::settings::DEFAULT_MAX_REVIEWS_PER_DAY,
        0,
        5000,
    )
    .await?;
    let max_new_cards = crate::routes::settings::current_bounded_i64(
        &state.pool,
        "max_new_cards_per_day",
        crate::routes::settings::DEFAULT_MAX_NEW_CARDS_PER_DAY,
        0,
        1000,
    )
    .await?;
    let (used_new_cards, used_reviews) = {
        let mut conn = state.pool.acquire().await?;
        daily_review_usage(&mut conn, user.user_id, now).await?
    };
    let limits = QueueLimits {
        max_reviews_per_day: usize::try_from(max_reviews.saturating_sub(used_reviews))
            .map_err(|_| ApiError::internal())?,
        max_new_per_day: usize::try_from(max_new_cards.saturating_sub(used_new_cards))
            .map_err(|_| ApiError::internal())?,
    };
    let queue = build_queue(limits, due_cards, new_cards);
    let render = |cards: &[QueueCard]| -> Vec<serde_json::Value> {
        cards
            .iter()
            .filter_map(|qc| {
                meta.get(&qc.id).map(|m| {
                    serde_json::json!({
                        "card_id": qc.id,
                        "front": m.front,
                        "back": m.back,
                        "card_type": m.card_type,
                        "cloze": m.cloze,
                        "trust": m.trust,
                        "ai_draft": m.trust == "ai_draft",
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
    apply_review(&state, user.user_id, req).await
}

/// OFF-02: shared with the offline sync endpoint — one review path, no drift.
pub async fn apply_review(
    state: &AppState,
    user_id: Uuid,
    req: ReviewEventReq,
) -> ApiResult<Json<serde_json::Value>> {
    let rating = parse_rating(&req.rating)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("user_not_found"))?;

    // Idempotent replay: same key on the same card returns the stored outcome.
    let replay = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
        "SELECT reviewed_at FROM review_events
         WHERE card_id = $1 AND user_id = $2 AND idempotency_key = $3",
    )
    .bind(req.card_id)
    .bind(user_id)
    .bind(&req.idempotency_key)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(reviewed_at) = replay {
        let card = sqlx::query("SELECT state FROM cards WHERE id = $1 AND user_id = $2")
            .bind(req.card_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| ApiError::not_found("card_not_found"))?;
        let cs: scheduler::CardState =
            serde_json::from_value(card.try_get("state")?).map_err(|_| ApiError::internal())?;
        tx.commit().await?;
        return Ok(Json(serde_json::json!({
            "already_recorded": true,
            "due": cs.due,
            "reviewed_at": reviewed_at,
        })));
    }

    let row = sqlx::query(
        "SELECT state FROM cards WHERE id = $1 AND user_id = $2 AND suspended = false FOR UPDATE",
    )
    .bind(req.card_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("card_not_found"))?;
    let current: scheduler::CardState =
        serde_json::from_value(row.try_get("state")?).map_err(|_| ApiError::internal())?;
    let was_new = current.state == 0;
    let reviewed_at = chrono::Utc::now();

    // Read both caps together inside the write transaction. The user-row lock
    // above serializes this check with every other review for this learner.
    let settings = sqlx::query(
        "SELECT key, value FROM app_settings WHERE key IN ('max_reviews_per_day', 'max_new_cards_per_day')",
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut max_reviews = crate::routes::settings::DEFAULT_MAX_REVIEWS_PER_DAY;
    let mut max_new_cards = crate::routes::settings::DEFAULT_MAX_NEW_CARDS_PER_DAY;
    for setting in settings {
        let key: String = setting.try_get("key")?;
        let value: serde_json::Value = setting.try_get("value")?;
        if let Some(value) = value.as_i64() {
            match key.as_str() {
                "max_reviews_per_day" if (0..=5000).contains(&value) => max_reviews = value,
                "max_new_cards_per_day" if (0..=1000).contains(&value) => max_new_cards = value,
                _ => {}
            }
        }
    }
    let (used_new_cards, used_reviews) = daily_review_usage(&mut *tx, user_id, reviewed_at).await?;
    let (used, cap, code, message) = if was_new {
        (
            used_new_cards,
            max_new_cards,
            "new_card_daily_cap_reached",
            "the daily new-card allowance has been used",
        )
    } else {
        (
            used_reviews,
            max_reviews,
            "daily_review_cap_reached",
            "the daily review allowance has been used",
        )
    };
    if used >= cap {
        return Err(ApiError::conflict(code, message));
    }

    let scheduler = Scheduler::new();
    let next = scheduler.review(from_state(&current), rating, reviewed_at);
    let next_state = to_state(&next);
    let state_json = serde_json::to_value(&next_state).map_err(|_| ApiError::internal())?;

    let inserted = sqlx::query(
        "INSERT INTO review_events (id, card_id, user_id, rating, reviewed_at, idempotency_key, was_new)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT DO NOTHING
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(req.card_id)
    .bind(user_id)
    .bind(&req.rating)
    .bind(reviewed_at)
    .bind(&req.idempotency_key)
    .bind(was_new)
    .fetch_optional(&mut *tx)
    .await?;
    if inserted.is_none() {
        // A competing writer may have inserted the same idempotency key.
        let replayed_at = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
            "SELECT reviewed_at FROM review_events
             WHERE card_id = $1 AND user_id = $2 AND idempotency_key = $3",
        )
        .bind(req.card_id)
        .bind(user_id)
        .bind(&req.idempotency_key)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(ApiError::internal)?;
        let card = sqlx::query("SELECT state FROM cards WHERE id = $1 AND user_id = $2")
            .bind(req.card_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
        let card_state: scheduler::CardState =
            serde_json::from_value(card.try_get("state")?).map_err(|_| ApiError::internal())?;
        tx.commit().await?;
        return Ok(Json(serde_json::json!({
            "already_recorded": true,
            "due": card_state.due,
            "reviewed_at": replayed_at,
        })));
    }

    sqlx::query("UPDATE cards SET state = $2 WHERE id = $1 AND user_id = $3")
        .bind(req.card_id)
        .bind(state_json)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "already_recorded": false,
        "due": next_state.due,
        "reviewed_at": reviewed_at,
    })))
}

// ---- SR-07: authorized deck import / export ---------------------------------

/// GET /v1/me/decks/export — every deck and card (with scheduling state) as
/// one JSON document. The format is versioned so other tools can rely on it.
pub async fn export_decks(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let decks = sqlx::query!(
        "SELECT id, name, created_at FROM decks WHERE user_id = $1 ORDER BY created_at",
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(decks.len());
    for d in &decks {
        let cards = sqlx::query!(
            r#"SELECT front, back, state, suspended, created_at FROM cards
               WHERE deck_id = $1 AND user_id = $2 ORDER BY created_at"#,
            d.id,
            user.user_id
        )
        .fetch_all(&state.pool)
        .await?;
        out.push(serde_json::json!({
            "name": d.name,
            "created_at": d.created_at,
            "cards": cards.iter().map(|c| serde_json::json!({
                "front": c.front,
                "back": c.back,
                "state": c.state,
                "suspended": c.suspended,
                "created_at": c.created_at,
            })).collect::<Vec<_>>(),
        }));
    }
    Ok(Json(
        serde_json::json!({ "format": "medical-os-decks/1", "decks": out }),
    ))
}

#[derive(Deserialize)]
pub struct ImportCard {
    pub front: String,
    pub back: String,
}

#[derive(Deserialize)]
pub struct ImportDeck {
    pub name: String,
    pub cards: Vec<ImportCard>,
}

#[derive(Deserialize)]
pub struct ImportDecksReq {
    pub decks: Vec<ImportDeck>,
}

/// POST /v1/me/decks/import — import decks from the export format. Cards
/// already known to the learner (same front) are skipped, so re-importing a
/// file never duplicates a deck; imported cards start a fresh schedule.
pub async fn import_decks(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<ImportDecksReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.decks.len() > 50 {
        return Err(ApiError::unprocessable(
            "batch_too_large",
            "at most 50 decks per import",
        ));
    }
    let mut decks_created = 0i64;
    let mut cards_created = 0i64;
    let mut cards_skipped = 0i64;
    for d in &req.decks {
        let name = d.name.trim();
        if name.is_empty() || name.len() > 120 {
            return Err(ApiError::unprocessable(
                "invalid_name",
                "deck name must be 1-120 characters",
            ));
        }
        let existing = sqlx::query!(
            "SELECT id FROM decks WHERE user_id = $1 AND name = $2",
            user.user_id,
            name
        )
        .fetch_optional(&state.pool)
        .await?;
        let deck_id = match existing {
            Some(row) => row.id,
            None => {
                let id = Uuid::new_v4();
                sqlx::query!(
                    "INSERT INTO decks (id, user_id, name) VALUES ($1, $2, $3)",
                    id,
                    user.user_id,
                    name
                )
                .execute(&state.pool)
                .await?;
                decks_created += 1;
                id
            }
        };
        for c in &d.cards {
            let known = sqlx::query!(
                "SELECT 1 AS one FROM cards WHERE user_id = $1 AND front = $2",
                user.user_id,
                c.front
            )
            .fetch_optional(&state.pool)
            .await?;
            if known.is_some() {
                cards_skipped += 1;
                continue;
            }
            let scheduler = Scheduler::new();
            let state_json = serde_json::to_value(to_state(&scheduler.new_card()))
                .map_err(|_| ApiError::internal())?;
            sqlx::query!(
                "INSERT INTO cards (id, deck_id, user_id, front, back, state)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                Uuid::new_v4(),
                deck_id,
                user.user_id,
                c.front,
                c.back,
                state_json
            )
            .execute(&state.pool)
            .await?;
            cards_created += 1;
        }
    }
    Ok(Json(serde_json::json!({
        "decks_created": decks_created,
        "cards_created": cards_created,
        "cards_skipped": cards_skipped,
    })))
}

// ---- PLAN-03: review-debt recovery numbers -----------------------------------

/// Honest debt report: what is due now, how much was actually cleared in the
/// last week, and how many days the backlog would take at that real rate.
/// No rate history means no projection.
pub async fn review_debt(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now();
    let week_ago = now - chrono::Duration::days(7);
    let due: i64 = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM cards
           WHERE user_id = $1 AND suspended = false
             AND (state->>'state')::int <> 0
             AND (state->>'due')::timestamptz <= $2"#,
        user.user_id,
        now
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    let recent: i64 = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM review_events
           WHERE user_id = $1 AND reviewed_at >= $2"#,
        user.user_id,
        week_ago
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    let backlog_days = if recent > 0 {
        Some((due as f64 / (recent as f64 / 7.0)).ceil() as i64)
    } else {
        None
    };
    Ok(Json(json!({
        "due_now": due,
        "completed_last_7_days": recent,
        "daily_rate": if recent > 0 { Some(recent as f64 / 7.0) } else { None },
        "projected_backlog_days": backlog_days,
        "note": if recent > 0 {
            "projection at your actual last-7-day rate"
        } else {
            "no review history yet - no projection without evidence"
        },
    })))
}
