-- 0013_program2: NOTE-02 note concepts, SR-08 re-test scheduling storage.
CREATE TABLE IF NOT EXISTS note_concepts (
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    concept TEXT NOT NULL,
    PRIMARY KEY (note_id, concept)
);

CREATE INDEX IF NOT EXISTS idx_note_concepts_concept ON note_concepts (concept);

-- SR-08: one re-test card per (learner, answered question version).
CREATE TABLE IF NOT EXISTS retest_cards (
    user_id UUID NOT NULL REFERENCES users(id),
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    passes INT NOT NULL DEFAULT 0,
    due TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, question_version_id)
);

CREATE TABLE IF NOT EXISTS retest_history (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    correct BOOLEAN NOT NULL,
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, question_version_id, idempotency_key)
);
