-- AI-08: learner-protected plan work and idempotent automatic revisions.
ALTER TABLE plan_tasks
    ADD COLUMN IF NOT EXISTS protected BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE plan_tasks
    ADD COLUMN IF NOT EXISTS estimated_minutes INTEGER;

-- Use a disclosed baseline of 1.5 minutes per question, rounded up.
UPDATE plan_tasks
SET estimated_minutes = GREATEST(1, (question_count * 3 + 1) / 2)
WHERE estimated_minutes IS NULL;

ALTER TABLE plan_tasks ALTER COLUMN estimated_minutes SET DEFAULT 15;
ALTER TABLE plan_tasks ALTER COLUMN estimated_minutes SET NOT NULL;

DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'plan_tasks'::regclass
          AND conname = 'plan_tasks_estimated_minutes_check'
    ) THEN
        ALTER TABLE plan_tasks
            ADD CONSTRAINT plan_tasks_estimated_minutes_check
            CHECK (estimated_minutes BETWEEN 1 AND 480);
    END IF;
END $$;

ALTER TABLE plan_revisions
    ADD COLUMN IF NOT EXISTS source_event_id UUID REFERENCES practice_sessions(id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_plan_revisions_source_event
    ON plan_revisions (source_event_id)
    WHERE source_event_id IS NOT NULL;
