//! OFF-02: idempotent event reconciliation. Offline-recorded events arrive
//! as a batch with client-generated event ids; each event is applied through
//! the SAME code path as its online endpoint (practice::apply_answer,
//! review::apply_review), so replaying a queue can never double-count. The
//! per-event outcomes make the whole batch safely retryable.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::routes::{practice, review};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SyncEvent {
    /// Client-generated id, echoed back so the queue can be pruned.
    pub event_id: String,
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Deserialize)]
pub struct SyncReq {
    pub events: Vec<SyncEvent>,
}

#[derive(Deserialize)]
struct AnswerPayload {
    session_id: Uuid,
    #[serde(flatten)]
    rest: practice::AnswerReq,
}

/// POST /v1/sync/events — apply a batch of offline events. Every event is
/// independent: one bad event never blocks the rest of the queue.
pub async fn sync_events(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<SyncReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.events.len() > 100 {
        return Err(crate::error::ApiError::unprocessable(
            "batch_too_large",
            "at most 100 events per sync batch",
        ));
    }
    let mut results = Vec::with_capacity(req.events.len());
    for event in req.events {
        let outcome = match event.kind.as_str() {
            "answer" => match serde_json::from_value::<AnswerPayload>(event.payload.clone()) {
                Ok(payload) => finish(
                    &event.event_id,
                    practice::apply_answer(&state, user.user_id, payload.session_id, payload.rest)
                        .await,
                ),
                Err(_) => invalid_payload(&event.event_id),
            },
            "review" => {
                match serde_json::from_value::<review::ReviewEventReq>(event.payload.clone()) {
                    Ok(payload) => finish(
                        &event.event_id,
                        review::apply_review(&state, user.user_id, payload).await,
                    ),
                    Err(_) => invalid_payload(&event.event_id),
                }
            }
            _ => json!({
                "event_id": event.event_id,
                "status": "rejected",
                "code": "unknown_kind",
            }),
        };
        results.push(outcome);
    }
    Ok(Json(json!({ "results": results })))
}

fn invalid_payload(event_id: &str) -> serde_json::Value {
    json!({
        "event_id": event_id,
        "status": "rejected",
        "code": "invalid_payload",
    })
}

fn finish(event_id: &str, result: ApiResult<Json<serde_json::Value>>) -> serde_json::Value {
    match result {
        Ok(mut value) => {
            value.0["event_id"] = json!(event_id);
            value.0["status"] = json!("applied_or_duplicate");
            value.0
        }
        Err(e) => json!({
            "event_id": event_id,
            "status": "rejected",
            "code": e.code,
            "message": e.message,
        }),
    }
}
