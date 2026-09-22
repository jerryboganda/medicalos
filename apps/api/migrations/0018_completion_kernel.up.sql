-- 0018_completion_kernel: shared durable state for the remaining phases.
-- Additive only; behavior stays behind the existing HTTP seam.

-- CORE-04/07: account lifecycle, explicit devices, revocable sessions.
ALTER TABLE users ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;
ALTER TABLE auth_sessions ADD COLUMN IF NOT EXISTS device_id TEXT;
ALTER TABLE auth_sessions ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS user_devices (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    device_key TEXT NOT NULL,
    label TEXT NOT NULL DEFAULT 'device',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ,
    UNIQUE (user_id, device_key)
);

-- EX-01/02/03/05/06: official registry metadata + date-effective exam specs.
ALTER TABLE exams ADD COLUMN IF NOT EXISTS official_source_url TEXT;
ALTER TABLE exams ADD COLUMN IF NOT EXISTS aliases JSONB NOT NULL DEFAULT '[]';

CREATE TABLE IF NOT EXISTS exam_spec_versions (
    id UUID PRIMARY KEY,
    exam_id UUID NOT NULL REFERENCES exams(id),
    version INT NOT NULL,
    effective_from DATE NOT NULL,
    effective_to DATE,
    config JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'published',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (exam_id, version)
);

CREATE TABLE IF NOT EXISTS assessment_forms (
    id UUID PRIMARY KEY,
    exam_spec_id UUID NOT NULL REFERENCES exam_spec_versions(id),
    name TEXT NOT NULL,
    assessment_family TEXT NOT NULL,
    blueprint JSONB NOT NULL,
    reserved BOOLEAN NOT NULL DEFAULT false,
    ai_allowed BOOLEAN NOT NULL DEFAULT false,
    frozen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS learner_accommodations (
    user_id UUID NOT NULL REFERENCES users(id),
    accommodation_key TEXT NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, accommodation_key)
);

-- AI-11/12 + PLAN-03/04 support.
CREATE TABLE IF NOT EXISTS coach_memory (
    user_id UUID NOT NULL REFERENCES users(id),
    memory_key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, memory_key)
);

CREATE TABLE IF NOT EXISTS intervention_outcomes (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    intervention_type TEXT NOT NULL,
    concept_key TEXT,
    triggered_at TIMESTAMPTZ NOT NULL,
    measured_at TIMESTAMPTZ,
    outcome JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- INST-01/03/05/06: programs, scoped external identities, publication roles,
-- and interoperability payload receipts.
CREATE TABLE IF NOT EXISTS institution_programs (
    id UUID PRIMARY KEY,
    institution_id UUID NOT NULL REFERENCES institutions(id),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE cohorts ADD COLUMN IF NOT EXISTS program_id UUID REFERENCES institution_programs(id);

CREATE TABLE IF NOT EXISTS external_identities (
    institution_id UUID NOT NULL REFERENCES institutions(id),
    provider TEXT NOT NULL,
    subject TEXT NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (institution_id, provider, subject)
);

CREATE TABLE IF NOT EXISTS assessment_publications (
    assessment_form_id UUID PRIMARY KEY REFERENCES assessment_forms(id),
    author_id UUID NOT NULL REFERENCES users(id),
    reviewer_id UUID REFERENCES users(id),
    publisher_id UUID REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'draft',
    reviewed_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS interoperability_receipts (
    id UUID PRIMARY KEY,
    institution_id UUID NOT NULL REFERENCES institutions(id),
    standard TEXT NOT NULL,
    direction TEXT NOT NULL,
    external_id TEXT,
    payload JSONB NOT NULL,
    status TEXT NOT NULL,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- COMMUNITY-01/02/03 + COMP-03/04.
CREATE TABLE IF NOT EXISTS community_groups (
    id UUID PRIMARY KEY,
    exam_id UUID REFERENCES exams(id),
    name TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'private',
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS community_group_members (
    group_id UUID NOT NULL REFERENCES community_groups(id),
    user_id UUID NOT NULL REFERENCES users(id),
    role TEXT NOT NULL DEFAULT 'member',
    handle TEXT,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, user_id)
);

CREATE TABLE IF NOT EXISTS community_challenges (
    id UUID PRIMARY KEY,
    group_id UUID REFERENCES community_groups(id),
    competition_id UUID REFERENCES competitions(id),
    challenger_id UUID NOT NULL REFERENCES users(id),
    challenged_id UUID NOT NULL REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE competition_entries ADD COLUMN IF NOT EXISTS integrity_status TEXT NOT NULL DEFAULT 'clear';

-- SIM-02/03/04/06/07 + IMG-01/02/03/04 + CAREER-02.
CREATE TABLE IF NOT EXISTS scenario_rubrics (
    id UUID PRIMARY KEY,
    scenario_id UUID NOT NULL REFERENCES scenarios(id),
    criterion_key TEXT NOT NULL,
    label TEXT NOT NULL,
    max_score REAL NOT NULL DEFAULT 1,
    UNIQUE (scenario_id, criterion_key)
);

CREATE TABLE IF NOT EXISTS scenario_rubric_evidence (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES scenario_runs(id),
    criterion_key TEXT NOT NULL,
    evidence TEXT NOT NULL,
    score REAL,
    transcript_uncertain BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS imaging_assets (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    modality TEXT,
    anatomy_region TEXT,
    rights_ref TEXT NOT NULL,
    deidentified BOOLEAN NOT NULL DEFAULT false,
    pixel_hash TEXT,
    annotations JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS supervised_feedback (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    portfolio_entry_id UUID REFERENCES portfolio_entries(id),
    scenario_run_id UUID REFERENCES scenario_runs(id),
    reviewer_id UUID NOT NULL REFERENCES users(id),
    feedback TEXT NOT NULL,
    signed_off BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- COM-03 / ADMIN-05 minimal commerce administration state.
CREATE TABLE IF NOT EXISTS price_tiers (
    id UUID PRIMARY KEY,
    region TEXT NOT NULL,
    product TEXT NOT NULL,
    currency TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (region, product)
);

CREATE TABLE IF NOT EXISTS coupons (
    code TEXT PRIMARY KEY,
    percent_off INT NOT NULL,
    expires_at TIMESTAMPTZ,
    active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS referral_codes (
    code TEXT PRIMARY KEY,
    owner_user_id UUID REFERENCES users(id),
    uses INT NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- TRUST-05/QB-10: collect outcome evidence now; predictive claims remain
-- disabled until a validation record exists for the intended use/population.
CREATE TABLE IF NOT EXISTS exam_outcomes (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    exam_id UUID NOT NULL REFERENCES exams(id),
    sat_on DATE NOT NULL,
    outcome JSONB NOT NULL,
    consented BOOLEAN NOT NULL DEFAULT false,
    verified BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS validation_records (
    id UUID PRIMARY KEY,
    intended_use TEXT NOT NULL,
    population TEXT NOT NULL,
    method TEXT NOT NULL,
    result JSONB NOT NULL,
    approved_by TEXT NOT NULL,
    approved_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
