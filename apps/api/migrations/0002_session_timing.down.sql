-- 0002_session_timing down: drop the timing columns. Guarded so rollback is
-- safe on any starting state (partial schema, fresh DB, parallel setups).
ALTER TABLE IF EXISTS practice_sessions DROP COLUMN IF EXISTS deadline;
ALTER TABLE IF EXISTS practice_sessions DROP COLUMN IF EXISTS time_limit_seconds;
