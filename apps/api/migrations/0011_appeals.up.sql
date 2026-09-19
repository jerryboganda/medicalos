-- 0011_appeals: learner challenges (SIM-07 adjacent; human review resolves).
CREATE TABLE IF NOT EXISTS appeals (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open', -- open | upheld | rejected
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
