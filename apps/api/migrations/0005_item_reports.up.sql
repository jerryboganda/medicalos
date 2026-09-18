-- 0005_item_reports: QB-08 learner reporting + quarantined-item exclusion.
-- Idempotent (IF NOT EXISTS) so psql, tests, and startup can all apply it.
CREATE TABLE IF NOT EXISTS question_reports (
    id UUID PRIMARY KEY,
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    reporter_id UUID NOT NULL REFERENCES users(id),
    category TEXT NOT NULL, -- wrong_answer | bad_explanation | typo | duplicate | outdated | broken_image | other
    note TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'open', -- open | quarantined | resolved_fixed | resolved_rejected
    resolution_note TEXT,
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (question_version_id, reporter_id)
);

CREATE INDEX IF NOT EXISTS idx_reports_version_status
    ON question_reports (question_version_id, status);
