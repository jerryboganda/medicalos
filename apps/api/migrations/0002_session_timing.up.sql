-- 0002_session_timing: EX-08 server-issued deadline for timed sessions.
ALTER TABLE practice_sessions ADD COLUMN IF NOT EXISTS time_limit_seconds INT;
ALTER TABLE practice_sessions ADD COLUMN IF NOT EXISTS deadline TIMESTAMPTZ;
