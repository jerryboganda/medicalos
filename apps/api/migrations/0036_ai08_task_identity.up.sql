-- Stable identity follows a plan task across immutable plan versions.
ALTER TABLE plan_tasks
    ADD COLUMN IF NOT EXISTS task_key UUID;

UPDATE plan_tasks
SET task_key = gen_random_uuid()
WHERE task_key IS NULL;

ALTER TABLE plan_tasks ALTER COLUMN task_key SET DEFAULT gen_random_uuid();
ALTER TABLE plan_tasks ALTER COLUMN task_key SET NOT NULL;

ALTER TABLE practice_sessions
    ADD COLUMN IF NOT EXISTS plan_task_key UUID;

CREATE INDEX IF NOT EXISTS idx_plan_tasks_task_key
    ON plan_tasks (task_key);

CREATE INDEX IF NOT EXISTS idx_practice_sessions_plan_task_key
    ON practice_sessions (plan_task_key)
    WHERE plan_task_key IS NOT NULL;
