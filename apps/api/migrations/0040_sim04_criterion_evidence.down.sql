DROP INDEX IF EXISTS idx_scenario_rubric_evidence_run_criterion;
DROP INDEX IF EXISTS idx_scenario_rubrics_version;

DROP TRIGGER IF EXISTS scenario_rubric_evidence_immutable ON scenario_rubric_evidence;
DROP TRIGGER IF EXISTS scenario_rubrics_version_immutable ON scenario_rubrics;
DROP FUNCTION IF EXISTS reject_scenario_assessment_mutation();
DROP TRIGGER IF EXISTS scenario_versions_content_immutable ON scenario_versions;
DROP FUNCTION IF EXISTS guard_scenario_version_mutation();

ALTER TABLE scenario_rubric_evidence
    DROP CONSTRAINT IF EXISTS scenario_rubric_evidence_assessment_check;

UPDATE scenario_rubric_evidence
SET score = legacy_score
WHERE assessment_status = 'legacy_unverified' AND legacy_score IS NOT NULL;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM scenario_rubrics
        GROUP BY scenario_id, criterion_key
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION 'cannot restore scenario-wide rubric keys while versioned duplicates exist';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'scenario_rubrics_scenario_id_criterion_key_key'
          AND conrelid = 'scenario_rubrics'::regclass
    ) THEN
        ALTER TABLE scenario_rubrics
            ADD CONSTRAINT scenario_rubrics_scenario_id_criterion_key_key
            UNIQUE (scenario_id, criterion_key);
    END IF;
END $$;

ALTER TABLE scenario_rubrics
    DROP CONSTRAINT IF EXISTS scenario_rubrics_version_criterion_unique;

ALTER TABLE scenario_rubric_evidence
    DROP COLUMN IF EXISTS transcript_event_indexes,
    DROP COLUMN IF EXISTS reviewer_id,
    DROP COLUMN IF EXISTS assessment_status,
    DROP COLUMN IF EXISTS legacy_score;

ALTER TABLE scenario_rubrics
    DROP COLUMN IF EXISTS scenario_version_id;
