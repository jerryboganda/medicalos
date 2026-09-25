DO $$
BEGIN
    -- Startup replays this migration; serialize its one-time backfill.
    PERFORM pg_advisory_xact_lock(1296387654, 50);

    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'competitions'
          AND column_name = 'difficulty_points'
    ) THEN
        ALTER TABLE competitions
            ADD COLUMN difficulty_points JSONB NOT NULL DEFAULT '[5, 10, 15]'::jsonb;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'competition_entries'
          AND column_name = 'correct_count'
    ) THEN
        ALTER TABLE competition_entries
            ADD COLUMN correct_count BIGINT NOT NULL DEFAULT 0,
            ADD COLUMN attempted_count BIGINT NOT NULL DEFAULT 0,
            ADD COLUMN average_response_time_ms DOUBLE PRECISION NOT NULL DEFAULT 0;

        WITH entry_stats AS (
            SELECT
                ce.id,
                COUNT(answer_item.payload) AS attempted_count,
                COUNT(*) FILTER (
                    WHERE question_version.correct_index::BIGINT =
                        (answer_item.payload ->> 'chosen_index')::BIGINT
                ) AS correct_count,
                COALESCE(
                    AVG((answer_item.payload ->> 'elapsed_ms')::BIGINT)::DOUBLE PRECISION,
                    0.0::DOUBLE PRECISION
                ) AS average_response_time_ms
            FROM competition_entries AS ce
            LEFT JOIN LATERAL jsonb_array_elements(
                CASE
                    WHEN jsonb_typeof(ce.answers) = 'array' THEN ce.answers
                    ELSE '[]'::jsonb
                END
            ) AS answer_item(payload) ON true
            LEFT JOIN question_versions AS question_version
                ON question_version.id =
                    (answer_item.payload ->> 'question_version_id')::UUID
            GROUP BY ce.id
        )
        UPDATE competition_entries AS ce
        SET correct_count = entry_stats.correct_count,
            attempted_count = entry_stats.attempted_count,
            average_response_time_ms = entry_stats.average_response_time_ms
        FROM entry_stats
        WHERE ce.id = entry_stats.id;
    END IF;

    CREATE TABLE IF NOT EXISTS competition_attempts (
        id UUID PRIMARY KEY,
        competition_id UUID NOT NULL REFERENCES competitions(id),
        user_id UUID NOT NULL REFERENCES users(id),
        handle TEXT NOT NULL,
        question_ids JSONB NOT NULL,
        current_index INTEGER NOT NULL DEFAULT 0 CHECK (current_index >= 0),
        option_order JSONB NOT NULL,
        answers JSONB NOT NULL DEFAULT '[]'::jsonb,
        question_started_at TIMESTAMPTZ NOT NULL,
        status TEXT NOT NULL DEFAULT 'in_progress'
            CHECK (status IN ('in_progress', 'submitted')),
        created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
        submitted_at TIMESTAMPTZ,
        UNIQUE (competition_id, user_id)
    );
END;
$$;
