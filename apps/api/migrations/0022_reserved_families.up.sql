-- 0022_reserved_families: EX-05 reserved assessment families served only
-- through their fixed form; EX-06/QB-04 evidence plumbing. Additive only.

-- Questions bound to a reserved form never leak into open pools (practice,
-- QOTD, retests) — they are served exclusively by their assessment form.
CREATE TABLE IF NOT EXISTS reserved_questions (
    question_version_id UUID PRIMARY KEY REFERENCES question_versions(id),
    form_id UUID NOT NULL REFERENCES assessment_forms(id),
    added_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_reserved_questions_form
    ON reserved_questions (form_id);

-- A session started from an assessment form snapshots its AI policy (EX-06).
ALTER TABLE practice_sessions
    ADD COLUMN IF NOT EXISTS form_id UUID REFERENCES assessment_forms(id),
    ADD COLUMN IF NOT EXISTS ai_allowed BOOLEAN;
