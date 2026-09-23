-- 0027_recovery_qa: OPS-04 recovery-drill evidence, AI-16 coach regression
-- runs. Additive only.

CREATE TABLE IF NOT EXISTS recovery_drills (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL, -- manifest_signature | schema_rollback | pack_rollback
    result TEXT NOT NULL, -- pass | fail
    evidence JSONB NOT NULL,
    ran_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS coach_regression_runs (
    id UUID PRIMARY KEY,
    cases_total INT NOT NULL,
    cases_passed INT NOT NULL,
    results JSONB NOT NULL,
    ran_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
