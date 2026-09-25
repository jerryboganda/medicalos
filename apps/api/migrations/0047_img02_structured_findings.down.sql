ALTER TABLE image_cases
    DROP CONSTRAINT IF EXISTS image_cases_findings_structured_array;

ALTER TABLE image_cases
    DROP COLUMN IF EXISTS findings_structured;
