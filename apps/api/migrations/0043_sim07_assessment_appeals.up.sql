-- SIM-07: appeal records and independent decisions remain separate from the
-- original, immutable examiner evidence.
CREATE TABLE scenario_assessment_appeals (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL UNIQUE REFERENCES scenario_runs(id),
    appellant_id UUID NOT NULL REFERENCES users(id),
    reason TEXT NOT NULL CHECK (char_length(reason) BETWEEN 10 AND 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_scenario_assessment_appeals_created
    ON scenario_assessment_appeals (created_at DESC);

CREATE TABLE scenario_assessment_appeal_reviews (
    id UUID PRIMARY KEY,
    appeal_id UUID NOT NULL UNIQUE REFERENCES scenario_assessment_appeals(id),
    reviewer_id UUID NOT NULL REFERENCES users(id),
    decision TEXT NOT NULL CHECK (decision IN ('confirmed', 'reassessment_required')),
    rationale TEXT NOT NULL CHECK (char_length(rationale) BETWEEN 10 AND 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION reject_scenario_assessment_appeal_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'scenario assessment appeals and decisions are immutable'
        USING ERRCODE = '55000';
    RETURN NULL;
END;
$$;

CREATE TRIGGER scenario_assessment_appeals_immutable
BEFORE UPDATE OR DELETE ON scenario_assessment_appeals
FOR EACH ROW
EXECUTE FUNCTION reject_scenario_assessment_appeal_mutation();

CREATE TRIGGER scenario_assessment_appeal_reviews_immutable
BEFORE UPDATE OR DELETE ON scenario_assessment_appeal_reviews
FOR EACH ROW
EXECUTE FUNCTION reject_scenario_assessment_appeal_mutation();
