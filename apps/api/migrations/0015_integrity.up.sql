-- 0015_integrity: client integrity signals (§11.3/EX-08). Records are
-- evidence; they never auto-punish (policy per test decides, §11.3).
CREATE TABLE IF NOT EXISTS integrity_events (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    session_id UUID REFERENCES practice_sessions(id),
    signal_type TEXT NOT NULL, -- background | screenshot | clock_change | fullscreen_exit | ...
    detail JSONB NOT NULL DEFAULT '{}',
    client_time TIMESTAMPTZ,
    server_time TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_integrity_session ON integrity_events (session_id);
