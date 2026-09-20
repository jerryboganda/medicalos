-- 0017_offline: Phase 3 offline pack leases (OFF-04, PROT-02, §22).
-- One lease per (learner, device, exam): the pack key is disclosed only to
-- the leasing device, leases expire (14-day default), and revocation is
-- server-side so a leaked key stops working at the next manifest check.

CREATE TABLE IF NOT EXISTS pack_leases (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL,
    exam_id UUID NOT NULL REFERENCES exams(id),
    chapters JSONB NOT NULL,
    pack_key TEXT NOT NULL, -- 32-byte hex, per-device (§22 NEW)
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, device_id, exam_id)
);

CREATE INDEX IF NOT EXISTS idx_pack_leases_user ON pack_leases (user_id, expires_at);
