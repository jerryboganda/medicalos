-- 0002_session_timing down: drop the timing columns.
ALTER TABLE practice_sessions DROP COLUMN IF EXISTS deadline;
ALTER TABLE practice_sessions DROP COLUMN IF EXISTS time_limit_seconds;
