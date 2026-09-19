-- CORE-09: tightly bounded guest question trial before account creation.
CREATE TABLE IF NOT EXISTS guest_trials (
    token_hash TEXT PRIMARY KEY,
    expires_at TIMESTAMPTZ NOT NULL,
    converted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS guest_trial_items (
    trial_token_hash TEXT NOT NULL REFERENCES guest_trials(token_hash) ON DELETE CASCADE,
    item_index SMALLINT NOT NULL CHECK (item_index >= 0 AND item_index < 3),
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    chosen_index SMALLINT,
    correct BOOLEAN,
    idempotency_key TEXT,
    answered_at TIMESTAMPTZ,
    PRIMARY KEY (trial_token_hash, item_index),
    UNIQUE (trial_token_hash, question_version_id),
    UNIQUE (trial_token_hash, idempotency_key)
);
