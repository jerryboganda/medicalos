ALTER TABLE mock_attempts DROP COLUMN IF EXISTS ranked;
DROP INDEX IF EXISTS idx_practice_sessions_due_auto_submit;
ALTER TABLE practice_sessions
    DROP CONSTRAINT IF EXISTS practice_sessions_integrity_timeout_policy_check,
    DROP COLUMN IF EXISTS auto_submitted_by_policy,
    DROP COLUMN IF EXISTS away_since,
    DROP COLUMN IF EXISTS away_timeout_seconds,
    DROP COLUMN IF EXISTS integrity_policy,
    DROP COLUMN IF EXISTS late_sync_grace_seconds;
ALTER TABLE mocks
    DROP CONSTRAINT IF EXISTS mocks_integrity_timeout_policy_check,
    DROP COLUMN IF EXISTS away_timeout_seconds,
    DROP COLUMN IF EXISTS integrity_policy,
    DROP COLUMN IF EXISTS late_sync_grace_seconds;
