-- QB-12: durable learner question marks feed the marked-only builder pool.
CREATE TABLE IF NOT EXISTS question_marks (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    question_version_id UUID NOT NULL REFERENCES question_versions(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, question_version_id)
);

CREATE INDEX IF NOT EXISTS idx_question_marks_question
    ON question_marks (question_version_id);
