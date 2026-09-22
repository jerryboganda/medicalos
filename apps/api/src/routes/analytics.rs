//! OPS-05: the pseudonymous product-analytics stream (master plan
//! Appendix B). Separate from learning evidence by construction: this table
//! is never read by the learner model, and the receiving endpoint enforces
//! the taxonomy plus the no-identifiers rule.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// The closed event vocabulary (Appendix B). Anything outside it is
/// rejected rather than stored under a free-form name.
const TAXONOMY: &[&str] = &[
    "app_open",
    "onboarding_step_completed",
    "baseline_completed",
    "activation_achieved",
    "session_started",
    "question_answered",
    "explanation_viewed",
    "session_submitted",
    "review_completed",
    "plan_revision_viewed",
    "plan_revision_undone",
    "coach_turn",
    "daily_goal_met",
    "streak_extended",
    "streak_lost",
    "mock_completed",
    "progress_viewed",
    "paywall_viewed",
    "purchase_completed",
    "subscription_renewed",
    "subscription_cancelled",
    "pack_downloaded",
    "sync_completed",
    "notification_opened",
    "question_reported",
    "competition_joined",
    "duel_created",
    "duel_completed",
    "share_card_sent",
    "integrity_event",
];

/// Rule 1: pseudonymous keys only. Property keys that routinely carry direct
/// identifiers are rejected outright.
const FORBIDDEN_PROPERTY_KEYS: &[&str] = &["email", "user_id", "token", "password"];

#[derive(Deserialize)]
pub struct AnalyticsBatchReq {
    /// Pseudonymous client ID (not the account id).
    pub anonymous_id: String,
    pub events: Vec<AnalyticsEventReq>,
}

#[derive(Deserialize)]
pub struct AnalyticsEventReq {
    pub name: String,
    #[serde(default)]
    pub properties: serde_json::Value,
}

pub async fn receive_events(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Json(req): Json<AnalyticsBatchReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let anon = req.anonymous_id.trim();
    if anon.is_empty() || anon.len() > 64 {
        return Err(ApiError::unprocessable(
            "invalid_anonymous_id",
            "anonymous_id must be 1-64 characters and pseudonymous",
        ));
    }
    if req.events.is_empty() || req.events.len() > 100 {
        return Err(ApiError::unprocessable(
            "invalid_batch",
            "1-100 events per batch",
        ));
    }
    let mut accepted = 0usize;
    for event in &req.events {
        if !TAXONOMY.contains(&event.name.as_str()) {
            return Err(ApiError::unprocessable(
                "unknown_event",
                "event names must come from the analytics taxonomy (Appendix B)",
            ));
        }
        let props = match event.properties.as_object() {
            Some(map) => map,
            None => {
                return Err(ApiError::unprocessable(
                    "invalid_properties",
                    "properties must be an object",
                ))
            }
        };
        for key in FORBIDDEN_PROPERTY_KEYS {
            if props.contains_key(*key) {
                return Err(ApiError::unprocessable(
                    "identifier_in_properties",
                    "the analytics stream is pseudonymous — no direct identifiers",
                ));
            }
        }
        sqlx::query!(
            "INSERT INTO analytics_events (anonymous_id, name, properties)
             VALUES ($1, $2, $3)",
            anon,
            event.name,
            event.properties
        )
        .execute(&state.pool)
        .await?;
        accepted += 1;
    }
    Ok(Json(json!({ "accepted": accepted })))
}
