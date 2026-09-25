DROP INDEX IF EXISTS idx_review_events_user_reviewed_at;

ALTER TABLE review_events
    DROP COLUMN IF EXISTS was_new;
