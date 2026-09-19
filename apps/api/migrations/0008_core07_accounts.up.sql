-- CORE-07: verified personal accounts, device-aware rotating sessions,
-- password recovery, and in-app deletion initiation.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS email_verified_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deletion_requested_at TIMESTAMPTZ;

ALTER TABLE auth_sessions
    ADD COLUMN IF NOT EXISTS id UUID,
    ADD COLUMN IF NOT EXISTS device_id TEXT,
    ADD COLUMN IF NOT EXISTS device_name TEXT,
    ADD COLUMN IF NOT EXISTS refresh_token_hash TEXT,
    ADD COLUMN IF NOT EXISTS refresh_expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ;

-- Existing sessions predate stable session ids. Backfill without requiring a
-- PostgreSQL extension; newly-created CORE-07 sessions always provide UUIDs.
UPDATE auth_sessions
SET id = md5(token_hash || created_at::text)::uuid
WHERE id IS NULL;

UPDATE auth_sessions
SET last_seen_at = created_at
WHERE last_seen_at IS NULL;

ALTER TABLE auth_sessions
    ALTER COLUMN id SET NOT NULL,
    ALTER COLUMN last_seen_at SET DEFAULT now(),
    ALTER COLUMN last_seen_at SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_sessions_id
    ON auth_sessions (id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_sessions_refresh_token_hash
    ON auth_sessions (refresh_token_hash)
    WHERE refresh_token_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_auth_sessions_active_device
    ON auth_sessions (user_id, device_id)
    WHERE revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS email_verification_challenges (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS password_reset_challenges (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

