-- ENG-01: independent learner switches for daily goal, streak, and QOTD.
-- Streak/progress remain derived from existing goal/session evidence.
CREATE TABLE IF NOT EXISTS engagement_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    daily_goal_enabled BOOLEAN NOT NULL DEFAULT true,
    streak_enabled BOOLEAN NOT NULL DEFAULT true,
    qotd_enabled BOOLEAN NOT NULL DEFAULT true,
    qotd_time TIME,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
