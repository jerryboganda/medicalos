-- LIB-06: private text imports remain owner-scoped and gated by live rights.
ALTER TABLE content_rights
    ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS revoked_by UUID REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS revocation_note TEXT;

CREATE TABLE IF NOT EXISTS private_documents (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rights_id UUID NOT NULL REFERENCES content_rights(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 200),
    media_type TEXT NOT NULL CHECK (media_type IN ('text/plain', 'text/markdown')),
    content TEXT NOT NULL CHECK (btrim(content) <> '' AND octet_length(content) <= 1048576),
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[a-f0-9]{64}$'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_private_documents_owner_created
    ON private_documents (user_id, created_at DESC);
