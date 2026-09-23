ALTER TABLE practice_sessions
    ADD COLUMN IF NOT EXISTS result_payload JSONB;
