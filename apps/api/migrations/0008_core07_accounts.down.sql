DROP TABLE IF EXISTS password_reset_challenges;
DROP TABLE IF EXISTS email_verification_challenges;

DROP INDEX IF EXISTS idx_auth_sessions_active_device;
DROP INDEX IF EXISTS idx_auth_sessions_refresh_token_hash;
DROP INDEX IF EXISTS idx_auth_sessions_id;

ALTER TABLE auth_sessions
    DROP COLUMN IF EXISTS revoked_at,
    DROP COLUMN IF EXISTS last_seen_at,
    DROP COLUMN IF EXISTS refresh_expires_at,
    DROP COLUMN IF EXISTS refresh_token_hash,
    DROP COLUMN IF EXISTS device_name,
    DROP COLUMN IF EXISTS device_id,
    DROP COLUMN IF EXISTS id;

ALTER TABLE users
    DROP COLUMN IF EXISTS deletion_requested_at,
    DROP COLUMN IF EXISTS email_verified_at;

