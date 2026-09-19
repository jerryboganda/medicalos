pub struct AppState {
    pub pool: sqlx::PgPool,
    /// EX-08 floor for timed sessions; CI/tests lower it for fast E2E.
    pub min_time_limit_seconds: i64,
    /// COM-01 free-tier daily question allowance (§26.1 proposes 10–15).
    pub free_daily_questions: i64,
    /// Test seam only: raw verification/reset secrets are never returned by
    /// the production binary. Real delivery belongs behind the mail adapter.
    pub expose_test_auth_tokens: bool,
}
