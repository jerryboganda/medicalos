-- QB-08/QB-16: keep public correction history beside its report resolution.
ALTER TABLE question_reports
    ADD COLUMN IF NOT EXISTS correction_note TEXT;
