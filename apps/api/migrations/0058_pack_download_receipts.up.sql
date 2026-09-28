-- OFF-01: verified download receipts — a server-recorded, signed attestation
-- that a device with an active lease received rights-verified pack content.
CREATE TABLE IF NOT EXISTS pack_download_receipts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    exam_id UUID NOT NULL REFERENCES exams(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    checksums JSONB NOT NULL,
    signature TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_pack_download_receipts_user
    ON pack_download_receipts(user_id);
