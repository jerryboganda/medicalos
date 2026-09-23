ALTER TABLE content_rights
    DROP CONSTRAINT IF EXISTS content_rights_scope_terms_check;

ALTER TABLE content_rights
    DROP COLUMN IF EXISTS royalty_terms,
    DROP COLUMN IF EXISTS attribution,
    DROP COLUMN IF EXISTS derivative_terms,
    DROP COLUMN IF EXISTS ai_terms,
    DROP COLUMN IF EXISTS quotation_limit_words,
    DROP COLUMN IF EXISTS offline_terms,
    DROP COLUMN IF EXISTS seat_limit,
    DROP COLUMN IF EXISTS audiences,
    DROP COLUMN IF EXISTS asset_refs,
    DROP COLUMN IF EXISTS contract_version,
    DROP COLUMN IF EXISTS contract_ref;
