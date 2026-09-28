-- §6.3: per-account login throttling. Counts consecutive failed passwords
-- and locks the account with a capped exponential backoff (auth.rs).
CREATE TABLE IF NOT EXISTS login_throttle (
    email TEXT PRIMARY KEY,
    consecutive_failures INT NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
