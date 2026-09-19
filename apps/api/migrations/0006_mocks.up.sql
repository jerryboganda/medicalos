-- 0005_mocks: EX-07 administrator-configured mock tests + frozen-form
-- attempts, plus the session linkage. Blueprint = [{chapter_id, count}].
CREATE TABLE IF NOT EXISTS mocks (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    exam_id UUID NOT NULL REFERENCES exams(id),
    blueprint JSONB NOT NULL,
    time_limit_seconds INT,
    pass_mark_percent INT NOT NULL DEFAULT 50,
    attempts_allowed INT NOT NULL DEFAULT 1,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE practice_sessions ADD COLUMN IF NOT EXISTS mock_id UUID REFERENCES mocks(id);

CREATE TABLE IF NOT EXISTS mock_attempts (
    id UUID PRIMARY KEY,
    mock_id UUID NOT NULL REFERENCES mocks(id),
    user_id UUID NOT NULL REFERENCES users(id),
    session_id UUID NOT NULL REFERENCES practice_sessions(id),
    score_percent INT NOT NULL,
    passed BOOLEAN NOT NULL,
    percentile INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (mock_id, user_id, session_id)
);

CREATE INDEX IF NOT EXISTS idx_mock_attempts_mock ON mock_attempts (mock_id);
