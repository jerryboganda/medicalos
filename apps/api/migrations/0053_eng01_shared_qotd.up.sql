-- ENG-01: select one stable, published QOTD question per exam and database day.
ALTER TABLE engagement_settings
    ADD COLUMN IF NOT EXISTS qotd_exam_id UUID REFERENCES exams(id) ON DELETE SET NULL;

CREATE TABLE IF NOT EXISTS qotd_daily_questions (
    exam_id UUID NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    day DATE NOT NULL,
    question_version_id UUID NOT NULL REFERENCES question_versions(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (exam_id, day)
);

-- Preserve the current day's legacy answers: the first answer per exam becomes
-- that exam's shared pick for the remainder of the database calendar day.
UPDATE engagement_settings es
SET qotd_exam_id = cn.exam_id
FROM qotd_answers a
JOIN question_versions qv ON qv.id = a.question_version_id
JOIN curriculum_nodes cn ON cn.id = qv.chapter_id
WHERE es.user_id = a.user_id
  AND a.day = CURRENT_DATE
  AND es.qotd_exam_id IS NULL;

INSERT INTO qotd_daily_questions (exam_id, day, question_version_id)
SELECT DISTINCT ON (cn.exam_id, a.day)
       cn.exam_id, a.day, a.question_version_id
FROM qotd_answers a
JOIN question_versions qv ON qv.id = a.question_version_id
JOIN curriculum_nodes cn ON cn.id = qv.chapter_id
WHERE a.day = CURRENT_DATE
ORDER BY cn.exam_id, a.day, a.answered_at, a.user_id
ON CONFLICT (exam_id, day) DO NOTHING;
