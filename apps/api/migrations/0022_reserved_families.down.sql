ALTER TABLE practice_sessions
    DROP COLUMN IF EXISTS ai_allowed,
    DROP COLUMN IF EXISTS form_id;
DROP INDEX IF EXISTS idx_reserved_questions_form;
DROP TABLE IF EXISTS reserved_questions;
