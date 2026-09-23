DROP TABLE IF EXISTS private_documents;

ALTER TABLE content_rights
    DROP COLUMN IF EXISTS revocation_note,
    DROP COLUMN IF EXISTS revoked_by,
    DROP COLUMN IF EXISTS revoked_at;
