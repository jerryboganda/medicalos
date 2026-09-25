ALTER TABLE question_versions
    DROP COLUMN IF EXISTS media_refs,
    DROP COLUMN IF EXISTS source_refs,
    DROP COLUMN IF EXISTS tags,
    DROP COLUMN IF EXISTS rights_ref;
