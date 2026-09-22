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
            "answer" => {
                apply::<AnswerPayload, _, _>(&state, &user, &event, |state, user, payload| {
                    practice::apply_answer(state, user, payload.session_id, payload.rest)
                })
                .await
            }
            "review" => {
                apply::<review::ReviewEventReq, _, _>(&state, &user, &event, |state, user, payload| {
                    review::apply_review(state, user, payload)
                })
                .await
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

async fn apply<T, F, Fut>(
    state: &Arc<AppState>,
    user: &AuthUser,
    event: &SyncEvent,
    f: F,
) -> serde_json::Value
where
    T: serde::de::DeserializeOwned,
    F: FnOnce(&AppState, Uuid, T) -> Fut,
    Fut: std::future::Future<Output = ApiResult<Json<serde_json::Value>>>,
{
    let payload: T = match serde_json::from_value(event.payload.clone()) {
        Ok(p) => p,
        Err(_) => {
            return json!({
                "event_id": event.event_id,
                "status": "rejected",
                "code": "invalid_payload",
            })
        }
    };
    match f(state, user.user_id, payload).await {
        Ok(mut value) => {
            value.0["event_id"] = json!(event.event_id);
            value.0["status"] = json!("applied_or_duplicate");
            value.0
        }
        Err(e) => json!({
            "event_id": event.event_id,
            "status": "rejected",
            "code": e.code,
            "message": e.message,
        }),
    }
}
