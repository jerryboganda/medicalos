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
    let min_time_limit: i64 = std::env::var("MIN_TIME_LIMIT_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let free_daily_questions: i64 = std::env::var("FREE_DAILY_QUESTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let state = std::sync::Arc::new(api::state::AppState {
        pool,
        min_time_limit_seconds: min_time_limit,
        free_daily_questions,
        community_min_sample: std::env::var("COMMUNITY_MIN_SAMPLE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20),
        admin_token: std::env::var("ADMIN_TOKEN").ok().filter(|t| !t.is_empty()),
        free_mock_attempts: std::env::var("FREE_MOCK_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
        free_analytics_drills: std::env::var("FREE_ANALYTICS_DRILLS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5),
        free_daily_coach_turns: std::env::var("FREE_DAILY_COACH_TURNS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20),
        openai_api_key: std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|t| !t.is_empty()),
        openai_base_url: std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
        pack_signing_key: std::env::var("PACK_SIGNING_KEY").ok(),
    });
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("bind 8080");
    tracing::info!("api listening on 8080");
    axum::serve(listener, api::router(state))
        .await
        .expect("serve");
}
