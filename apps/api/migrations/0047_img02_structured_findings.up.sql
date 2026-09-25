ALTER TABLE image_cases
    ADD COLUMN IF NOT EXISTS findings_structured JSONB NOT NULL DEFAULT '[]'::jsonb;

UPDATE image_cases
SET findings_structured = jsonb_build_array(
    jsonb_build_object('section', 'Findings', 'text', findings)
)
WHERE findings_structured = '[]'::jsonb;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'image_cases_findings_structured_array'
          AND conrelid = 'image_cases'::regclass
    ) THEN
        ALTER TABLE image_cases
            ADD CONSTRAINT image_cases_findings_structured_array
            CHECK (jsonb_typeof(findings_structured) = 'array');
    END IF;
END $$;
