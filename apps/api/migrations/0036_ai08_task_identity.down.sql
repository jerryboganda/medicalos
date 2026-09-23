DROP INDEX IF EXISTS idx_practice_sessions_plan_task_key;
DROP INDEX IF EXISTS idx_plan_tasks_task_key;
ALTER TABLE practice_sessions DROP COLUMN IF EXISTS plan_task_key;
ALTER TABLE plan_tasks DROP COLUMN IF EXISTS task_key;
