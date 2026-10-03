ALTER TABLE pack_download_receipts
    DROP COLUMN IF EXISTS issued_at,
    DROP COLUMN IF EXISTS request_nonce;
