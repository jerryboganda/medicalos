pub struct AppState {
    pub pool: sqlx::PgPool,
    /// EX-08 floor for timed sessions; CI/tests lower it for fast E2E.
    pub min_time_limit_seconds: i64,
    /// COM-01 free-tier daily question allowance (§26.1 proposes 10–15).
    pub free_daily_questions: i64,
    /// QB-15: community statistics stay hidden below this attempt sample.
    pub community_min_sample: i64,
    /// §18/§19.5 scaffold: mock configuration is admin-gated until the
    /// editorial console lands. None = endpoint disabled.
    pub admin_token: Option<String>,
}
