-- ENG-01: persist declared daily availability and support a time-based goal.
ALTER TABLE engagement_settings
    ADD COLUMN IF NOT EXISTS daily_goal_mode TEXT NOT NULL DEFAULT 'questions'
        CHECK (daily_goal_mode IN ('questions', 'minutes')),
    ADD COLUMN IF NOT EXISTS daily_available_minutes INT NOT NULL DEFAULT 60
        CHECK (daily_available_minutes BETWEEN 5 AND 480);

ALTER TABLE qotd_answers
    ADD COLUMN IF NOT EXISTS elapsed_ms BIGINT NOT NULL DEFAULT 0
        CHECK (elapsed_ms BETWEEN 0 AND 3600000);
