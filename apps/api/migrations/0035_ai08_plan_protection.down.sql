DROP INDEX IF EXISTS idx_plan_revisions_source_event;
ALTER TABLE plan_revisions DROP COLUMN IF EXISTS source_event_id;
ALTER TABLE plan_tasks
    DROP CONSTRAINT IF EXISTS plan_tasks_estimated_minutes_check,
    DROP COLUMN IF EXISTS estimated_minutes,
    DROP COLUMN IF EXISTS protected;
