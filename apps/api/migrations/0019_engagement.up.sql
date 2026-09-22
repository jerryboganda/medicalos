-- 0019_engagement: ENG-01 daily goal, streak with freezes, question of the day.
-- Additive only; every mechanic is disableable per learner.

CREATE TABLE IF NOT EXISTS engagement_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    daily_goal_questions INT NOT NULL DEFAULT 20,
    daily_goal_enabled BOOLEAN NOT NULL DEFAULT true,
    streak_enabled BOOLEAN NOT NULL DEFAULT true,
    qotd_enabled BOOLEAN NOT NULL DEFAULT true,
    freeze_bank INT NOT NULL DEFAULT 0 CHECK (freeze_bank >= 0 AND freeze_bank <= 2),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS engagement_days (
    user_id UUID NOT NULL REFERENCES users(id),
    day DATE NOT NULL,
    questions_answered INT NOT NULL DEFAULT 0,
    goal_met BOOLEAN NOT NULL DEFAULT false,
    streak_count INT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, day)
);

CREATE TABLE IF NOT EXISTS qotd_answers (
    user_id UUID NOT NULL REFERENCES users(id),
    day DATE NOT NULL,
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    chosen_index INT NOT NULL,
    answered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, day)
);
