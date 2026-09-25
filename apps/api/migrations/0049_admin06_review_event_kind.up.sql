DO $$
BEGIN
    -- apply_up replays migrations at startup; serialize concurrent instances
    -- while checking, backfilling, and creating the index.
    PERFORM pg_advisory_xact_lock(1296387654, 49);

    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'review_events'
          AND column_name = 'was_new'
    ) THEN
        ALTER TABLE review_events
            ADD COLUMN IF NOT EXISTS was_new BOOLEAN NOT NULL DEFAULT false;

        WITH first_review AS (
            SELECT DISTINCT ON (card_id) id
            FROM review_events
            ORDER BY card_id, reviewed_at, created_at, id
        )
        UPDATE review_events AS event
        SET was_new = true
        FROM first_review
        WHERE event.id = first_review.id;
    END IF;

    CREATE INDEX IF NOT EXISTS idx_review_events_user_reviewed_at
        ON review_events (user_id, reviewed_at);
END;
$$;
