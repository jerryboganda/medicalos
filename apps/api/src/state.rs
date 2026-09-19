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
    /// §26.1/AI-13: free daily AI allowance (Coach turns) — cost limit.
    pub free_daily_coach_turns: i64,
    /// §23: when set, the OpenAI-compatible adapter routes complex turns.
    pub openai_api_key: Option<String>,
    pub openai_base_url: String,
}

impl AppState {
    /// Admin gate shared by mock configuration and the editorial console.
    pub fn require_admin(&self, provided: Option<&str>) -> Result<(), crate::error::ApiError> {
        match (&self.admin_token, provided) {
            (Some(expected), Some(got)) if expected == got => Ok(()),
            (Some(_), _) => Err(crate::error::ApiError::forbidden(
                "admin_required",
                "this operation requires the admin token",
            )),
            (None, _) => Err(crate::error::ApiError::forbidden(
                "admin_disabled",
                "admin operations are disabled (no ADMIN_TOKEN configured)",
            )),
        }
    }
}
