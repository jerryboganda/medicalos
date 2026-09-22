DROP TABLE IF EXISTS assessment_reviews;
ALTER TABLE question_versions
    DROP COLUMN IF EXISTS published_by,
    DROP COLUMN IF EXISTS reviewed_by,
    DROP COLUMN IF EXISTS created_by;
DROP INDEX IF EXISTS idx_audit_events_institution;
ALTER TABLE audit_events DROP COLUMN IF EXISTS institution_id;
