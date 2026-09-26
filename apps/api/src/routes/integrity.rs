//! Integrity event recording (§11.3/EX-08). Clients report signals; the
//! server timestamps them authoritatively and stores them as evidence for
//! the per-test integrity policy. Records never auto-punish.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct IntegrityEventReq {
    pub session_id: Option<Uuid>,
    pub signal_type: String,
    #[serde(default)]
    pub detail: serde_json::Value,
    pub client_time: Option<chrono::DateTime<chrono::Utc>>,
}

const VALID_SIGNALS: &[&str] = &[
    "background",
    "screenshot",
    "screen_record",
    "split_screen",
    "clock_change",
    "fullscreen_exit",
    "window_blur",
    "attestation_failure",
    "second_session",
];

pub async fn record_integrity_event(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<IntegrityEventReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !VALID_SIGNALS.contains(&req.signal_type.as_str()) {
        return Err(ApiError::unprocessable(
            "invalid_signal",
            format!("signal_type must be one of: {}", VALID_SIGNALS.join(", ")),
        ));
    }
    if let Some(session_id) = req.session_id {
        let session_is_owned = sqlx::query!(
            "SELECT id FROM practice_sessions WHERE id = $1 AND user_id = $2",
            session_id,
            user.user_id
        )
        .fetch_optional(&state.pool)
        .await?
        .is_some();
        if !session_is_owned {
            return Err(ApiError::not_found("session_not_found"));
        }
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO integrity_events
           (id, user_id, session_id, signal_type, detail, client_time)
         VALUES ($1, $2, $3, $4, $5, $6)",
        id,
        user.user_id,
        req.session_id,
        req.signal_type,
        req.detail,
        req.client_time
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "recorded": true, "event_id": id })))
}
