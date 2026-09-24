-- SIM-03: corrections for uncertain text-mode transcript segments. The
-- original transcript stays immutable; corrections are separate records
-- shown alongside the uncertain segment.
CREATE TABLE IF NOT EXISTS scenario_transcript_corrections (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES scenario_runs(id),
    event_index INT NOT NULL CHECK (event_index >= 0),
    original_event TEXT NOT NULL,
    corrected_text TEXT NOT NULL CHECK (char_length(corrected_text) BETWEEN 1 AND 500),
    corrected_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (run_id, event_index)
);
