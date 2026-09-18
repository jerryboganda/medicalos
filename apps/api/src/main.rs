// Thin startup: config from env, schema apply, serve. All logic lives in the
// library so integration tests exercise the same router (tdd: one seam).
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    telemetry::init();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await
        .expect("connect to database");
    api::schema::apply_up(&pool).await.expect("apply schema");
    // CI/E2E helper: apply schema, seed synthetic fixtures, exit.
    if std::env::args().any(|a| a == "--seed") {
        api::seed::seed(&pool).await.expect("seed fixtures");
        tracing::info!("seeded fixture content");
        return;
    }
    let state = std::sync::Arc::new(api::state::AppState { pool });
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("bind 8080");
    tracing::info!("api listening on 8080");
    axum::serve(listener, api::router(state))
        .await
        .expect("serve");
}
