-- 0008_coach: AI-06/09/16 — the Coach: source-grounded tutoring turns.
-- Turns are evidence: idempotent by key, scoped to the learner, and they
-- record which adapter/model produced them (§23 routing accountability).
CREATE TABLE IF NOT EXISTS coach_turns (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    question_version_id UUID REFERENCES question_versions(id),
    prompt_type TEXT NOT NULL,        -- free | why_wrong | explain | compare
    message TEXT NOT NULL,
    answer TEXT NOT NULL,
    adapter TEXT NOT NULL,            -- extractive | openai-compatible | ...
    model TEXT NOT NULL,
    grounded_on JSONB NOT NULL,       -- the permitted sources used
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_coach_turns_user ON coach_turns (user_id, created_at);
