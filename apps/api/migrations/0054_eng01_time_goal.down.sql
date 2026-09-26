ALTER TABLE qotd_answers DROP COLUMN IF EXISTS elapsed_ms;
ALTER TABLE engagement_settings
    DROP COLUMN IF EXISTS daily_available_minutes,
    DROP COLUMN IF EXISTS daily_goal_mode;
