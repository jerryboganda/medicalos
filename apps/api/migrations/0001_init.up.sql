-- 0001_init.up: Phase 1 slice 1 schema (see .scratch/phase-1-slice-1/spec.md).
-- Idempotent (IF NOT EXISTS) so psql, tests, and startup can all apply it.

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS auth_sessions (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

-- EX-01 (minimal): exam registry; aliases and versioned blueprints arrive later.
CREATE TABLE IF NOT EXISTS exams (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL
);

-- CORE-10 (minimal): Exam -> Subject -> System -> Chapter navigation taxonomy.
CREATE TABLE IF NOT EXISTS curriculum_nodes (
    id UUID PRIMARY KEY,
    exam_id UUID NOT NULL REFERENCES exams(id),
    kind TEXT NOT NULL, -- subject | system | chapter
    name TEXT NOT NULL,
    parent_id UUID REFERENCES curriculum_nodes(id),
    display_order INT NOT NULL DEFAULT 0
);

-- QB-01/QB-02: stable question identity + content family.
CREATE TABLE IF NOT EXISTS questions (
    id UUID PRIMARY KEY,
    family_id UUID NOT NULL
);

-- QB-01/QB-05/QB-11/QB-14: immutable published versions.
CREATE TABLE IF NOT EXISTS question_versions (
    id UUID PRIMARY KEY,
    question_id UUID NOT NULL REFERENCES questions(id),
    version INT NOT NULL,
    status TEXT NOT NULL, -- published
    chapter_id UUID NOT NULL REFERENCES curriculum_nodes(id),
    difficulty TEXT NOT NULL, -- easy | medium | hard
    vignette TEXT NOT NULL,
    lead_in TEXT NOT NULL,
    options JSONB NOT NULL, -- [{text, rationale}], length 2..=10 enforced in Rust (QB-11)
    correct_index SMALLINT NOT NULL,
    key_learning_point TEXT NOT NULL,
    exam_tip TEXT,
    high_yield BOOLEAN NOT NULL DEFAULT false,
    source_ref TEXT NOT NULL,
    UNIQUE (question_id, version)
);

CREATE INDEX IF NOT EXISTS idx_question_versions_chapter
    ON question_versions (chapter_id) WHERE status = 'published';

-- QB-03: practice sessions (presets: tutor | revision).
CREATE TABLE IF NOT EXISTS practice_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    preset TEXT NOT NULL,
    chapter_id UUID REFERENCES curriculum_nodes(id),
    source_session_id UUID REFERENCES practice_sessions(id),
    status TEXT NOT NULL DEFAULT 'open', -- open | submitted
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    submitted_at TIMESTAMPTZ
);

-- Planned item set of a session (navigator and results need the full set,
-- including not-yet-answered items).
CREATE TABLE IF NOT EXISTS session_items (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES practice_sessions(id),
    item_index SMALLINT NOT NULL,
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    UNIQUE (session_id, item_index)
);

-- EX-04/QB-04: durable attempt evidence; idempotent by key, first answer wins.
CREATE TABLE IF NOT EXISTS attempts (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES practice_sessions(id),
    item_index SMALLINT NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    chosen_index SMALLINT, -- NULL = skipped
    correct BOOLEAN,
    confidence TEXT, -- sure | unsure | NULL
    assisted BOOLEAN NOT NULL DEFAULT false,
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, item_index),
    UNIQUE (session_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_attempts_user ON attempts (user_id);

-- AI-01/AI-17: learner-concept state per user+chapter, server-authoritative.
CREATE TABLE IF NOT EXISTS learner_concept_state (
    user_id UUID NOT NULL REFERENCES users(id),
    chapter_id UUID NOT NULL REFERENCES curriculum_nodes(id),
    ability REAL NOT NULL DEFAULT 1500.0,
    evidence_count INT NOT NULL DEFAULT 0,
    independent_count INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, chapter_id)
);

-- PLAN-01: one plan per user per day, versioned.
CREATE TABLE IF NOT EXISTS plans (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    plan_date DATE NOT NULL DEFAULT CURRENT_DATE,
    version INT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    UNIQUE (user_id, plan_date, version)
);

CREATE TABLE IF NOT EXISTS plan_tasks (
    id UUID PRIMARY KEY,
    plan_id UUID NOT NULL REFERENCES plans(id),
    kind TEXT NOT NULL, -- practice | revision
    title TEXT NOT NULL,
    chapter_id UUID REFERENCES curriculum_nodes(id),
    question_count INT NOT NULL DEFAULT 10,
    source_session_id UUID REFERENCES practice_sessions(id),
    status TEXT NOT NULL DEFAULT 'pending', -- pending | done
    added_by_revision UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- PLAN-01/AI-07: plan revisions with an embedded action receipt.
CREATE TABLE IF NOT EXISTS plan_revisions (
    id UUID PRIMARY KEY,
    plan_id UUID NOT NULL REFERENCES plans(id),
    from_version INT NOT NULL,
    to_version INT NOT NULL,
    reason_code TEXT NOT NULL,
    explanation TEXT NOT NULL,
    automatic BOOLEAN NOT NULL,
    undone BOOLEAN NOT NULL DEFAULT false,
    receipt JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
