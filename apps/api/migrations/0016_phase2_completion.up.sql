-- 0016_phase2_completion: QB-06 marked pool, QB-17 time + answer-change
-- analysis, SR-09 key-point card provenance.

-- QB-17: client-reported per-item time (server clamps; honesty: null when
-- absent, never synthesized) and how many times a mock answer changed
-- before submission (answer-change analysis, §11.9).
ALTER TABLE attempts
    ADD COLUMN IF NOT EXISTS elapsed_ms BIGINT,
    ADD COLUMN IF NOT EXISTS answer_changes INT NOT NULL DEFAULT 0;

-- QB-06: learner-marked questions are a first-class session pool.
CREATE TABLE IF NOT EXISTS question_marks (
    user_id UUID NOT NULL REFERENCES users(id),
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, question_version_id)
);

CREATE INDEX IF NOT EXISTS idx_question_marks_user ON question_marks (user_id);

-- SR-09: provenance for auto-created key-point cards (which question
-- produced them) so review does not duplicate cards across attempts.
ALTER TABLE cards
    ADD COLUMN IF NOT EXISTS source_question_version_id UUID REFERENCES question_versions(id);
