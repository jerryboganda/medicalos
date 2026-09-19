-- 0014_program3: ENG-02 XP/achievements, COMP competitions storage,
-- INST-04 curriculum coverage support, ADMIN-06 settings (§19.5).

CREATE TABLE IF NOT EXISTS xp_ledger (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    points INT NOT NULL,
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_xp_user ON xp_ledger (user_id, created_at);

CREATE TABLE IF NOT EXISTS achievements (
    user_id UUID NOT NULL REFERENCES users(id),
    code TEXT NOT NULL,
    earned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, code)
);

CREATE TABLE IF NOT EXISTS competitions (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    exam_id UUID NOT NULL REFERENCES exams(id),
    question_ids JSONB NOT NULL, -- ordered list of question_version ids
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL DEFAULT 'scheduled', -- scheduled | open | closed
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS competition_entries (
    id UUID PRIMARY KEY,
    competition_id UUID NOT NULL REFERENCES competitions(id),
    user_id UUID NOT NULL REFERENCES users(id),
    handle TEXT NOT NULL, -- opt-in display handle; never email (§17.1)
    answers JSONB NOT NULL,
    score REAL NOT NULL DEFAULT 0,
    total_time_ms BIGINT NOT NULL DEFAULT 0,
    submitted_order INT NOT NULL,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (competition_id, user_id)
);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
