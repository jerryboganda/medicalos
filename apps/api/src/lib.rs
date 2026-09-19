//! Phase 1 slice 1: the connected question loop (see
//! .scratch/phase-1-slice-1/spec.md). The HTTP API is the tested seam.

pub mod agent;
pub mod auth;
pub mod error;
pub mod routes;
pub mod schema;
pub mod seed;
pub mod state;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

pub fn router(state: Arc<state::AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/auth/register", post(routes::auth::register))
        .route("/v1/auth/login", post(routes::auth::login))
        .route(
            "/v1/me/goals",
            get(routes::goals::get_goals).put(routes::goals::put_goals),
        )
        .route("/v1/me/goals/undo", post(routes::goals::undo_goals))
        .route("/v1/me/contexts", get(routes::tenancy::my_contexts))
        .route("/v1/tenant/context", get(routes::tenancy::tenant_context))
        .route("/v1/me/today", get(routes::today::today))
        .route(
            "/v1/practice/sessions",
            post(routes::practice::create_session),
        )
        .route(
            "/v1/practice/sessions/{sid}",
            get(routes::practice::get_session),
        )
        .route(
            "/v1/practice/sessions/{sid}/answers",
            post(routes::practice::answer),
        )
        .route(
            "/v1/practice/sessions/{sid}/submit",
            post(routes::practice::submit),
        )
        .route(
            "/v1/plans/{pid}/revisions/{rid}/undo",
            post(routes::today::undo_revision),
        )
        .route(
            "/v1/questions/versions/{vid}/reports",
            post(routes::reports::report).get(routes::reports::my_reports),
        )
        .route("/v1/reports/{rid}/resolve", post(routes::reports::resolve))
        .route("/v1/decks", post(routes::review::create_deck))
        .route("/v1/decks/{deck_id}/cards", post(routes::review::add_card))
        .route("/v1/reviews/queue", get(routes::review::queue))
        .route("/v1/reviews/events", post(routes::review::review_event))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}
