-- QB-16 / ADMIN-06: acknowledgement provenance and correction links.
ALTER TABLE question_reports
    ADD COLUMN IF NOT EXISTS acknowledged_at TIMESTAMPTZ;

UPDATE question_reports
SET acknowledged_at = created_at
WHERE acknowledged_at IS NULL;

ALTER TABLE question_reports
    ALTER COLUMN acknowledged_at SET DEFAULT now(),
    ALTER COLUMN acknowledged_at SET NOT NULL,
    ADD COLUMN IF NOT EXISTS resolved_by UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS corrected_version_id UUID REFERENCES question_versions(id);

CREATE INDEX IF NOT EXISTS idx_reports_unresolved_created
    ON question_reports (created_at, question_version_id)
    WHERE status IN ('open', 'quarantined');
