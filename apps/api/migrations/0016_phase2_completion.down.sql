-- 0016_phase2_completion down. Guarded against any starting state.
ALTER TABLE IF EXISTS cards DROP COLUMN IF EXISTS source_question_version_id;
DROP TABLE IF EXISTS question_marks;
ALTER TABLE IF EXISTS attempts
    DROP COLUMN IF EXISTS answer_changes,
    DROP COLUMN IF EXISTS elapsed_ms;
