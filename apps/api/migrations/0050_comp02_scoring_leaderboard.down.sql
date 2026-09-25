DROP TABLE IF EXISTS competition_attempts;

ALTER TABLE competitions
    DROP COLUMN IF EXISTS difficulty_points;

ALTER TABLE competition_entries
    DROP COLUMN IF EXISTS correct_count,
    DROP COLUMN IF EXISTS attempted_count,
    DROP COLUMN IF EXISTS average_response_time_ms;
