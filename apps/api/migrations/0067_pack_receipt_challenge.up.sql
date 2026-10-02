-- Preserve the complete signed payload so recorded receipts can be audited.
ALTER TABLE pack_download_receipts
    ADD COLUMN IF NOT EXISTS request_nonce TEXT,
    ADD COLUMN IF NOT EXISTS issued_at TEXT;
