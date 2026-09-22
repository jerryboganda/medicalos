-- 0021_tenant_foundations: CORE-04 (tenant audit scoping), INST-05
-- (assessment author/reviewer/publisher separation), §18.3/§19.3.
-- Additive only; existing published rows keep NULL actor columns.

-- Institution-scoped audit events (§18.3 audit exports).
ALTER TABLE audit_events
    ADD COLUMN IF NOT EXISTS institution_id UUID REFERENCES institutions(id);
CREATE INDEX IF NOT EXISTS idx_audit_events_institution
    ON audit_events (institution_id, created_at);

-- Workflow actor record on question versions (§19.3: the author of an
-- item cannot be its approver).
ALTER TABLE question_versions
    ADD COLUMN IF NOT EXISTS created_by UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS reviewed_by UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS published_by UUID REFERENCES users(id);

-- Durable review decisions (evidence, not just state).
CREATE TABLE IF NOT EXISTS assessment_reviews (
    id UUID PRIMARY KEY,
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    reviewer UUID NOT NULL REFERENCES users(id),
    decision TEXT NOT NULL, -- approved | rejected
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
