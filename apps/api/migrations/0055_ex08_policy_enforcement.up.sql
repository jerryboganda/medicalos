-- EX-08: snapshot assessment sync and away-time policy on mock sessions.
ALTER TABLE mocks
    ADD COLUMN IF NOT EXISTS late_sync_grace_seconds INT NOT NULL DEFAULT 600
        CHECK (late_sync_grace_seconds BETWEEN 0 AND 600),
    ADD COLUMN IF NOT EXISTS integrity_policy TEXT NOT NULL DEFAULT 'log_only'
        CHECK (integrity_policy IN ('log_only', 'warn', 'auto_submit')),
    ADD COLUMN IF NOT EXISTS away_timeout_seconds INT
        CHECK (away_timeout_seconds BETWEEN 15 AND 3600);

ALTER TABLE practice_sessions
    ADD COLUMN IF NOT EXISTS late_sync_grace_seconds INT NOT NULL DEFAULT 0
        CHECK (late_sync_grace_seconds BETWEEN 0 AND 600),
    ADD COLUMN IF NOT EXISTS integrity_policy TEXT NOT NULL DEFAULT 'log_only'
        CHECK (integrity_policy IN ('log_only', 'warn', 'auto_submit')),
    ADD COLUMN IF NOT EXISTS away_timeout_seconds INT
        CHECK (away_timeout_seconds BETWEEN 15 AND 3600),
    ADD COLUMN IF NOT EXISTS away_since TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS auto_submitted_by_policy BOOLEAN NOT NULL DEFAULT FALSE;

-- Existing tutor/timed sessions had the same fixed ten-minute grace before
-- this policy became configurable; preserve it across an in-flight migration.
UPDATE practice_sessions
SET late_sync_grace_seconds = 600
WHERE preset IN ('tutor', 'timed') AND late_sync_grace_seconds = 0;

DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'mocks_integrity_timeout_policy_check'
    ) THEN
        ALTER TABLE mocks ADD CONSTRAINT mocks_integrity_timeout_policy_check
        CHECK (
            (integrity_policy = 'log_only' AND away_timeout_seconds IS NULL)
            OR (integrity_policy IN ('warn', 'auto_submit')
                AND away_timeout_seconds BETWEEN 15 AND 3600)
        );
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'practice_sessions_integrity_timeout_policy_check'
    ) THEN
        ALTER TABLE practice_sessions ADD CONSTRAINT practice_sessions_integrity_timeout_policy_check
        CHECK (
            (integrity_policy = 'log_only' AND away_timeout_seconds IS NULL)
            OR (integrity_policy IN ('warn', 'auto_submit')
                AND away_timeout_seconds BETWEEN 15 AND 3600)
        );
    END IF;
END $$;

ALTER TABLE mock_attempts
    ADD COLUMN IF NOT EXISTS ranked BOOLEAN NOT NULL DEFAULT TRUE;

CREATE INDEX IF NOT EXISTS idx_practice_sessions_due_auto_submit
    ON practice_sessions (away_since)
    WHERE preset = 'mock' AND status = 'open'
      AND integrity_policy = 'auto_submit' AND away_since IS NOT NULL;
