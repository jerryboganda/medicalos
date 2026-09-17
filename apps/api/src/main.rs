// Minimal API skeleton (Phase 0): proves the Rust service builds and runs in
// CI. Endpoints, database, and OpenAPI generation arrive with the Phase 1
// vertical slice (plan §28).
use axum::{routing::get, Router};

async fn healthz() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    telemetry::init();
    let app = Router::new().route("/healthz", get(healthz));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("bind 8080");
    tracing::info!("api listening on 8080");
    axum::serve(listener, app).await.expect("serve");
}
