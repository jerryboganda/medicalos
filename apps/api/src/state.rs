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
    /// COM-01: full-mock attempts included in the free tier (§26.1).
    pub free_mock_attempts: i64,
    /// COM-01: free-tier daily chapter-analytics drill-downs (PROG-01
    /// difficulty/trend views); beyond this the upgrade trigger fires.
    pub free_analytics_drills: i64,
    /// CORE-03: free-tier daily library retrievals (search queries plus
    /// article opens); beyond this the upgrade trigger fires.
    pub free_daily_library: i64,
    /// §23: when set, the OpenAI-compatible adapter routes complex turns.
    pub openai_api_key: Option<String>,
    pub openai_base_url: String,
    /// OFF-01: private secret deriving the lease-bound public-key pack signer.
    pub pack_signing_key: Option<String>,
    /// Encryption key for per-institution OIDC client secrets. Omit to allow public clients only.
    pub oidc_credential_key: Option<String>,
    /// Public API prefix registered with OIDC providers, e.g. https://host/api.
    pub public_api_base_url: String,
    /// Browser origin where the SPA's OIDC handoff page is served.
    pub public_app_url: String,
    /// Platform identity provider. None = platform sign-in disabled
    /// (password login and institution SSO keep working).
    pub zitadel: Option<ZitadelConfig>,
}

/// The API's confidential OIDC client at Zitadel (infra/zitadel/provision.sh).
pub struct ZitadelConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    /// Project whose roles become platform roles (audience scope).
    pub project_id: Option<String>,
    /// Zitadel IdP ids behind the "Continue with Google/Apple" buttons.
    pub google_idp_id: Option<String>,
    pub apple_idp_id: Option<String>,
}

impl AppState {
    /// Admin gate shared by mock configuration and the editorial console.
    /// A session holding `PlatformOps` with a second factor passes (§18.1,
    /// §25 — checked through `authz`); the shared `ADMIN_TOKEN` stays valid
    /// as the operator break-glass while the role cutover is pending.
    pub fn require_admin(
        &self,
        user: &crate::auth::AuthUser,
        provided: Option<&str>,
    ) -> Result<(), crate::error::ApiError> {
        if user
            .permissions()
            .contains(&crate::authz::Permission::PlatformOps)
        {
            if user.mfa {
                return Ok(());
            }
            if provided.is_none() {
                // The role alone is not enough without the second factor;
                // `require` returns exactly that mfa_required error.
                return user.require(crate::authz::Permission::PlatformOps);
            }
            // A presented token still gets the legacy check below.
        }
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
