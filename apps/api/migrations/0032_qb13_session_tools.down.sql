DROP TRIGGER IF EXISTS attempts_preserve_hint_assistance ON attempts;
DROP FUNCTION IF EXISTS preserve_hint_assistance();

ALTER TABLE session_items
    DROP COLUMN IF EXISTS hint_used;
ALTER TABLE question_versions
    DROP COLUMN IF EXISTS hint;
ALTER TABLE attempts
    DROP COLUMN IF EXISTS offline_recorded_at;
