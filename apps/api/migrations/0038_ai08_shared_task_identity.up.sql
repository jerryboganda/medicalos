-- Existing forks created before task_key existed contain separate task rows.
-- Match snapshots by their plan-local ordinal within the task shape. A fork
-- that used the column default for created_at cannot use timestamps as identity.
WITH ranked AS (
    SELECT t.id, t.task_key, p.user_id, p.plan_date, p.version AS plan_version,
           t.kind, t.title, t.chapter_id, t.question_count, t.source_session_id,
           t.added_by_revision, t.created_at,
           ROW_NUMBER() OVER (
               PARTITION BY p.user_id, p.plan_date, p.version, t.kind, t.title,
                            t.chapter_id, t.question_count, t.source_session_id,
                            t.added_by_revision
               ORDER BY t.created_at, t.id
           ) AS copy_index
    FROM plan_tasks t
    JOIN plans p ON p.id = t.plan_id
),
canonical AS (
    SELECT DISTINCT ON (
               user_id, plan_date, kind, title, chapter_id, question_count,
               source_session_id, added_by_revision, copy_index
           )
           user_id, plan_date, kind, title, chapter_id, question_count,
           source_session_id, added_by_revision, copy_index, task_key
    FROM ranked
    ORDER BY user_id, plan_date, kind, title, chapter_id, question_count,
             source_session_id, added_by_revision, copy_index,
             plan_version, id
)
UPDATE plan_tasks t
SET task_key = canonical.task_key
FROM ranked r
JOIN canonical ON ROW(
        r.user_id, r.plan_date, r.kind, r.title, r.chapter_id,
        r.question_count, r.source_session_id, r.added_by_revision, r.copy_index
    ) IS NOT DISTINCT FROM ROW(
        canonical.user_id, canonical.plan_date, canonical.kind,
        canonical.title, canonical.chapter_id, canonical.question_count,
        canonical.source_session_id, canonical.added_by_revision, canonical.copy_index
    )
WHERE t.id = r.id AND t.task_key IS DISTINCT FROM canonical.task_key;
