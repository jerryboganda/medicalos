-- 0004_recall: SR-01/02/03 storage — decks, FSRS card state, review events.
CREATE TABLE IF NOT EXISTS decks (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS cards (
    id UUID PRIMARY KEY,
    deck_id UUID NOT NULL REFERENCES decks(id),
    user_id UUID NOT NULL REFERENCES users(id),
    front TEXT NOT NULL,
    back TEXT NOT NULL,
    state JSONB NOT NULL, -- scheduler::CardState
    suspended BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_cards_user_due ON cards (user_id) WHERE suspended = false;

CREATE TABLE IF NOT EXISTS review_events (
    id UUID PRIMARY KEY,
    card_id UUID NOT NULL REFERENCES cards(id),
    user_id UUID NOT NULL REFERENCES users(id),
    rating TEXT NOT NULL, -- again | hard | good | easy
    reviewed_at TIMESTAMPTZ NOT NULL,
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (card_id, idempotency_key)
);
