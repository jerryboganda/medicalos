DROP INDEX IF EXISTS idx_reports_unresolved_created;
ALTER TABLE question_reports
    DROP COLUMN IF EXISTS corrected_version_id,
    DROP COLUMN IF EXISTS resolved_by,
    DROP COLUMN IF EXISTS acknowledged_at;
