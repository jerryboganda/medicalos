-- SIM-04: rubric criteria and assessor evidence follow the exact station version.
ALTER TABLE scenario_rubrics
    ADD COLUMN IF NOT EXISTS scenario_version_id UUID REFERENCES scenario_versions(id);

UPDATE scenario_rubrics rubric
SET scenario_version_id = (
    SELECT version.id
    FROM scenario_versions version
    WHERE version.scenario_id = rubric.scenario_id
    ORDER BY version.version DESC
    LIMIT 1
)
WHERE scenario_version_id IS NULL;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM scenario_rubrics WHERE scenario_version_id IS NULL) THEN
        RAISE EXCEPTION 'scenario rubric could not be attached to a version';
    END IF;
END $$;

ALTER TABLE scenario_rubrics
    ALTER COLUMN scenario_version_id SET NOT NULL;

ALTER TABLE scenario_rubrics
    DROP CONSTRAINT IF EXISTS scenario_rubrics_scenario_id_criterion_key_key;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'scenario_rubrics_version_criterion_unique'
          AND conrelid = 'scenario_rubrics'::regclass
    ) THEN
        ALTER TABLE scenario_rubrics
            ADD CONSTRAINT scenario_rubrics_version_criterion_unique
            UNIQUE (scenario_version_id, criterion_key);
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_scenario_rubrics_version
    ON scenario_rubrics (scenario_version_id);

ALTER TABLE scenario_rubric_evidence
    ADD COLUMN IF NOT EXISTS assessment_status TEXT,
    ADD COLUMN IF NOT EXISTS reviewer_id UUID REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS transcript_event_indexes JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS legacy_score REAL;

UPDATE scenario_rubric_evidence
SET assessment_status = CASE WHEN score IS NULL THEN 'not_assessed' ELSE 'assessed' END
WHERE assessment_status IS NULL;

-- Historical scores have no immutable transcript references. Preserve their
-- value for rollback, but never serve them as verified examiner scores.
UPDATE scenario_rubric_evidence
SET legacy_score = score,
    score = NULL,
    assessment_status = 'legacy_unverified'
WHERE assessment_status = 'assessed'
  AND jsonb_array_length(transcript_event_indexes) = 0;

ALTER TABLE scenario_rubric_evidence
    ALTER COLUMN assessment_status SET DEFAULT 'not_assessed',
    ALTER COLUMN assessment_status SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'scenario_rubric_evidence_assessment_check'
          AND conrelid = 'scenario_rubric_evidence'::regclass
    ) THEN
        ALTER TABLE scenario_rubric_evidence
            ADD CONSTRAINT scenario_rubric_evidence_assessment_check
            CHECK (
                CASE WHEN jsonb_typeof(transcript_event_indexes) <> 'array' THEN FALSE
                ELSE assessment_status IN ('assessed', 'not_assessed', 'legacy_unverified')
                    AND (
                        (assessment_status = 'assessed' AND score IS NOT NULL
                         AND jsonb_array_length(transcript_event_indexes) > 0)
                        OR (assessment_status = 'not_assessed' AND score IS NULL
                            AND jsonb_array_length(transcript_event_indexes) = 0)
                        OR (assessment_status = 'legacy_unverified' AND score IS NULL
                            AND legacy_score IS NOT NULL
                            AND jsonb_array_length(transcript_event_indexes) = 0)
                    )
                END
            );
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS idx_scenario_rubric_evidence_run_criterion
    ON scenario_rubric_evidence (run_id, criterion_key);

CREATE OR REPLACE FUNCTION reject_scenario_assessment_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'scenario rubric and examiner evidence are immutable; create a new scenario version'
        USING ERRCODE = '55000';
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS scenario_rubrics_version_immutable ON scenario_rubrics;
CREATE TRIGGER scenario_rubrics_version_immutable
BEFORE UPDATE OR DELETE ON scenario_rubrics
FOR EACH ROW
EXECUTE FUNCTION reject_scenario_assessment_mutation();

DROP TRIGGER IF EXISTS scenario_rubric_evidence_immutable ON scenario_rubric_evidence;
CREATE TRIGGER scenario_rubric_evidence_immutable
BEFORE UPDATE OR DELETE ON scenario_rubric_evidence
FOR EACH ROW
EXECUTE FUNCTION reject_scenario_assessment_mutation();

-- A published run's state machine is versioned evidence. Lifecycle status may
-- still change for LIB-05 quarantine/archive handling, but version contents
-- and identity cannot be rewritten or removed.
CREATE OR REPLACE FUNCTION guard_scenario_version_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'scenario versions cannot be deleted'
            USING ERRCODE = '55000';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.scenario_id IS DISTINCT FROM OLD.scenario_id
       OR NEW.version IS DISTINCT FROM OLD.version
       OR NEW.state_machine IS DISTINCT FROM OLD.state_machine
       OR NEW.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION 'scenario version content is immutable'
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS scenario_versions_content_immutable ON scenario_versions;
CREATE TRIGGER scenario_versions_content_immutable
BEFORE UPDATE OR DELETE ON scenario_versions
FOR EACH ROW
EXECUTE FUNCTION guard_scenario_version_mutation();
