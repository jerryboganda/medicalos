// Thin startup: config from env, schema apply, serve. All logic lives in the
// library so integration tests exercise the same router (tdd: one seam).
use sqlx::postgres::PgPoolOptions;

fn env_opt(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Platform sign-in is on only when all three client settings are present.
fn zitadel_from_env() -> Option<api::state::ZitadelConfig> {
    Some(api::state::ZitadelConfig {
        issuer: env_opt("ZITADEL_ISSUER")?,
        client_id: env_opt("ZITADEL_CLIENT_ID")?,
        client_secret: env_opt("ZITADEL_CLIENT_SECRET")?,
        project_id: env_opt("ZITADEL_PROJECT_ID"),
        google_idp_id: env_opt("ZITADEL_IDP_GOOGLE"),
        apple_idp_id: env_opt("ZITADEL_IDP_APPLE"),
    })
}

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
    let pack_signing_key = std::env::var("PACK_SIGNING_KEY")
        .ok()
        .filter(|key| key.trim().len() >= 32)
        .expect("PACK_SIGNING_KEY must be set to at least 32 non-whitespace bytes");
    let min_time_limit: i64 = std::env::var("MIN_TIME_LIMIT_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let free_daily_questions: i64 = std::env::var("FREE_DAILY_QUESTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let free_daily_library: i64 = std::env::var("FREE_DAILY_LIBRARY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let state = std::sync::Arc::new(api::state::AppState {
        pool,
        min_time_limit_seconds: min_time_limit,
        free_daily_questions,
        free_daily_library,
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
        pack_signing_key: Some(pack_signing_key),
        oidc_credential_key: std::env::var("OIDC_CREDENTIAL_KEY")
            .ok()
            .filter(|key| !key.is_empty()),
        public_api_base_url: std::env::var("PUBLIC_API_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8080/api".into()),
        public_app_url: std::env::var("PUBLIC_APP_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:5173".into()),
        zitadel: zitadel_from_env(),
        lti_tool_key: env_opt("LTI_TOOL_PRIVATE_KEY"),
    });
    api::routes::integrity::spawn_auto_submit_worker(state.clone());
    api::routes::jobs::spawn_jobs_worker(state.clone());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("bind 8080");
    tracing::info!("api listening on 8080");
    axum::serve(listener, api::router(state))
        .await
        .expect("serve");
}
