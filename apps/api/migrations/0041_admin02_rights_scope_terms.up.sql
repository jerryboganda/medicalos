-- ADMIN-02: preserve the contract scope needed for downstream authorization.
ALTER TABLE content_rights
    ADD COLUMN IF NOT EXISTS contract_ref TEXT,
    ADD COLUMN IF NOT EXISTS contract_version TEXT,
    ADD COLUMN IF NOT EXISTS asset_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS audiences JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS seat_limit INTEGER,
    ADD COLUMN IF NOT EXISTS offline_terms TEXT,
    ADD COLUMN IF NOT EXISTS quotation_limit_words INTEGER,
    ADD COLUMN IF NOT EXISTS ai_terms TEXT,
    ADD COLUMN IF NOT EXISTS derivative_terms TEXT,
    ADD COLUMN IF NOT EXISTS attribution TEXT,
    ADD COLUMN IF NOT EXISTS royalty_terms TEXT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'content_rights_scope_terms_check'
          AND conrelid = 'content_rights'::regclass
    ) THEN
        ALTER TABLE content_rights
            ADD CONSTRAINT content_rights_scope_terms_check
            CHECK (
                jsonb_typeof(asset_refs) = 'array'
                AND jsonb_typeof(audiences) = 'array'
                AND (seat_limit IS NULL OR seat_limit > 0)
                AND (quotation_limit_words IS NULL OR quotation_limit_words >= 0)
            );
    END IF;
END $$;
