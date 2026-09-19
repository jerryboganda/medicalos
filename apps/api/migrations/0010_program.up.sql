-- 0010_program: Phase 2/4/5/6/7 storage foundations.
-- AI-18: pre-generated one-tap tutoring, generated from reviewed content at
-- publish time — cached, offline-includable, never live-model (§9.5).
CREATE TABLE IF NOT EXISTS pregen_tutoring (
    id UUID PRIMARY KEY,
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    prompt_type TEXT NOT NULL, -- explain | why_wrong | compare | mnemonic | test_me
    content TEXT NOT NULL,
    generated_by TEXT NOT NULL DEFAULT 'extractive',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (question_version_id, prompt_type)
);

-- OPS-06: remote config / feature flags / staged rollout.
CREATE TABLE IF NOT EXISTS feature_flags (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    rollout_percent INT NOT NULL DEFAULT 100,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- INST-01/02: institutions, memberships with roles (§18.1), cohorts,
-- assignments. Faculty workspace queries build on these (INST-02).
CREATE TABLE IF NOT EXISTS institutions (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS institution_members (
    institution_id UUID NOT NULL REFERENCES institutions(id),
    user_id UUID NOT NULL REFERENCES users(id),
    role TEXT NOT NULL, -- admin | instructor | learner
    PRIMARY KEY (institution_id, user_id)
);

CREATE TABLE IF NOT EXISTS cohorts (
    id UUID PRIMARY KEY,
    institution_id UUID NOT NULL REFERENCES institutions(id),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS cohort_members (
    cohort_id UUID NOT NULL REFERENCES cohorts(id),
    user_id UUID NOT NULL REFERENCES users(id),
    PRIMARY KEY (cohort_id, user_id)
);

CREATE TABLE IF NOT EXISTS assignments (
    id UUID PRIMARY KEY,
    cohort_id UUID NOT NULL REFERENCES cohorts(id),
    title TEXT NOT NULL,
    due_at TIMESTAMPTZ,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- CAREER-01: longitudinal portfolio entries (no patient identifiers, §17).
CREATE TABLE IF NOT EXISTS portfolio_entries (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    kind TEXT NOT NULL, -- rotation | case_reflection | procedure_observation | certificate
    title TEXT NOT NULL,
    detail TEXT NOT NULL DEFAULT '',
    occurred_on DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- CAREER-03: continuing-education activity records + credit ledger. Credits
-- are NOT called accredited until a real accreditation exists (§16).
CREATE TABLE IF NOT EXISTS ce_activities (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    activity TEXT NOT NULL,
    hours REAL NOT NULL DEFAULT 0,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- SIM-01/02/05: versioned case scripts with deterministic authored state
-- machines. The LLM may voice the patient (later slices); vitals, labs and
-- transitions come from this table only (§14.3).
CREATE TABLE IF NOT EXISTS scenarios (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    version INT NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'published',
    state_machine JSONB NOT NULL, -- {states: {...}, initial, transitions: [{from, on, to, effects}]}
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS scenario_runs (
    id UUID PRIMARY KEY,
    scenario_id UUID NOT NULL REFERENCES scenarios(id),
    user_id UUID NOT NULL REFERENCES users(id),
    current_state TEXT NOT NULL,
    transcript JSONB NOT NULL DEFAULT '[]',
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ
);

-- Phase 6 scaffold: translation strings keyed by locale (UI strings first;
-- content translation is a Phase 6 pack concern).
CREATE TABLE IF NOT EXISTS translations (
    locale TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (locale, key)
);
