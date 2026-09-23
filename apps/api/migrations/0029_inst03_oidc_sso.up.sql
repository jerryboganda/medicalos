-- INST-03: OIDC authorization-code login with short-lived, one-use state.
CREATE TABLE IF NOT EXISTS institution_oidc_providers (
    institution_id UUID PRIMARY KEY REFERENCES institutions(id) ON DELETE CASCADE,
    issuer TEXT NOT NULL,
    client_id TEXT NOT NULL,
    client_secret_ciphertext BYTEA,
    enabled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS oidc_login_states (
    state_hash TEXT PRIMARY KEY,
    institution_id UUID NOT NULL REFERENCES institutions(id) ON DELETE CASCADE,
    issuer TEXT NOT NULL,
    client_id TEXT NOT NULL,
    nonce TEXT NOT NULL,
    pkce_verifier TEXT NOT NULL,
    provider_metadata JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_oidc_login_states_expiry ON oidc_login_states (expires_at);

CREATE TABLE IF NOT EXISTS oidc_login_tickets (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_oidc_login_tickets_expiry ON oidc_login_tickets (expires_at);
