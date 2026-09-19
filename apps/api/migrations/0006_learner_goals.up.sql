-- 0006_learner_goals: CORE-02 learner-owned, append-only planning constraints.
CREATE TABLE IF NOT EXISTS learner_goal_versions (
    user_id UUID NOT NULL REFERENCES users(id),
    version INT NOT NULL CHECK (version > 0),
    daily_minutes INT CHECK (daily_minutes BETWEEN 1 AND 1440),
    exam_date DATE,
    protected_commitments JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, version)
);
