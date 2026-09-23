-- 0026_session_tails: CORE-07 single-active-session + hard device limit,
-- QB-03 optional per-question budgets on untimed sessions.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS single_active_session BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS max_devices INT NOT NULL DEFAULT 5;

ALTER TABLE practice_sessions
    ADD COLUMN IF NOT EXISTS per_question_seconds INT;

-- COM-01: daily usage counters for free-tier entitlement triggers (the
-- chapter-analytics drill-down allowance).
CREATE TABLE IF NOT EXISTS entitlement_usage (
    user_id UUID NOT NULL REFERENCES users(id),
    key TEXT NOT NULL,
    day DATE NOT NULL,
    count INT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, key, day)
);
