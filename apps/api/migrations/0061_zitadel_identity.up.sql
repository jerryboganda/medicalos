-- Zitadel identity, phase 1 (.scratch/auth-zitadel/spec.md). Additive and
-- idempotent: startup re-applies every migration.
--
-- users.idp_subject links an account to its Zitadel user (`sub`). Accounts
-- created through Zitadel have no local password, so the hash is optional.
ALTER TABLE users ADD COLUMN IF NOT EXISTS idp_subject TEXT UNIQUE;
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- A session carries the platform-role snapshot (Zitadel project roles) and
-- whether the sign-in used a second factor (§6.3/§25: MFA for privileged
-- roles). The one-use OIDC hand-off ticket carries the same pair across.
ALTER TABLE auth_sessions
    ADD COLUMN IF NOT EXISTS roles TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS mfa BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE oidc_login_tickets
    ADD COLUMN IF NOT EXISTS roles TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS mfa BOOLEAN NOT NULL DEFAULT false;

-- Platform sign-in state (PKCE verifier, nonce), one-use, 5-minute expiry —
-- the institution flow keeps its own oidc_login_states table untouched.
CREATE TABLE IF NOT EXISTS platform_login_states (
    state_hash TEXT PRIMARY KEY,
    nonce TEXT NOT NULL,
    pkce_verifier TEXT NOT NULL,
    provider_metadata JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_platform_login_states_expiry ON platform_login_states (expires_at);

-- §18.1 institution roles become a closed vocabulary for new writes (NOT
-- VALID: rows written before this migration are not rechecked).
ALTER TABLE institution_members DROP CONSTRAINT IF EXISTS institution_members_role_known;
ALTER TABLE institution_members ADD CONSTRAINT institution_members_role_known
    CHECK (role IN ('admin', 'program_lead', 'instructor', 'author', 'reviewer', 'examiner', 'learner'))
    NOT VALID;
