DROP TABLE IF EXISTS entitlement_usage;
ALTER TABLE practice_sessions DROP COLUMN IF EXISTS per_question_seconds;
ALTER TABLE users
    DROP COLUMN IF EXISTS max_devices,
    DROP COLUMN IF EXISTS single_active_session;
