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
        // Same-origin production prefix: nginx serves the API under /api/
        // (medicalos.polytronx.com/api/* -> medicalos-api:8080/*).
        .route("/api/healthz", get(health_json))
        .route("/api/version.json", get(version))
        .route("/v1/auth/register", post(routes::auth::register))
        .route("/v1/auth/login", post(routes::auth::login))
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
        .route(
            "/v1/mocks",
            post(routes::mock::create_mock).get(routes::mock::list_mocks),
        )
        .route("/v1/mocks/{mid}/start", post(routes::mock::start_mock))
        .route(
            "/v1/questions/versions/{vid}/community-stats",
            get(routes::practice::community_stats),
        )
        .route("/v1/decks", post(routes::review::create_deck))
        .route("/v1/decks/{deck_id}/cards", post(routes::review::add_card))
        .route("/v1/reviews/queue", get(routes::review::queue))
        .route("/v1/reviews/events", post(routes::review::review_event))
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/me/today", get(routes::today::today))
        .route(
            "/api/v1/practice/sessions",
            post(routes::practice::create_session),
        )
        .route(
            "/api/v1/practice/sessions/{sid}",
            get(routes::practice::get_session),
        )
        .route(
            "/api/v1/practice/sessions/{sid}/answers",
            post(routes::practice::answer),
        )
        .route(
            "/api/v1/practice/sessions/{sid}/submit",
            post(routes::practice::submit),
        )
        .route(
            "/api/v1/plans/{pid}/revisions/{rid}/undo",
            post(routes::today::undo_revision),
        )
        .route(
            "/api/v1/questions/versions/{vid}/reports",
            post(routes::reports::report).get(routes::reports::my_reports),
        )
        .route(
            "/api/v1/reports/{rid}/resolve",
            post(routes::reports::resolve),
        )
        .route(
            "/api/v1/mocks",
            post(routes::mock::create_mock).get(routes::mock::list_mocks),
        )
        .route("/api/v1/mocks/{mid}/start", post(routes::mock::start_mock))
        .route(
            "/api/v1/questions/versions/{vid}/community-stats",
            get(routes::practice::community_stats),
        )
        .route("/api/v1/decks", post(routes::review::create_deck))
        .route(
            "/api/v1/decks/{deck_id}/cards",
            post(routes::review::add_card),
        )
        .route("/api/v1/reviews/queue", get(routes::review::queue))
        .route("/api/v1/reviews/events", post(routes::review::review_event))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn health_json() -> axum::Json<serde_json::Value> {
    // JSON twin for the plain-text healthz (lets the deploy probe parse it).
    axum::Json(serde_json::json!({"status": "ok"}))
}

async fn version() -> axum::Json<serde_json::Value> {
    // Served at /api/version.json in production so the deploy pipeline can
    // prove the exact commit landed. SHA stamped at image build time via
    // MEDICALOS_BUILD_SHA (Docker build-arg); option_env! keeps local/CI
    // builds compiling without it.
    axum::Json(serde_json::json!({
        "sha": option_env!("MEDICALOS_BUILD_SHA").unwrap_or("dev"),
    }))
}
