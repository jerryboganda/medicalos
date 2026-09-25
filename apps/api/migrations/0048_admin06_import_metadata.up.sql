ALTER TABLE question_versions
    ADD COLUMN IF NOT EXISTS tags TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS source_refs TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS media_refs TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS rights_ref TEXT;

UPDATE question_versions
SET source_refs = ARRAY[source_ref]
WHERE cardinality(source_refs) = 0 AND btrim(source_ref) <> '';
