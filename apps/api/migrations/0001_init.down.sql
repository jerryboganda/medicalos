-- 0001_init.down: full rollback of 0001 (§31.1: every migration has a tested
-- rollback path; CI proves up -> down -> up).

DROP TABLE IF EXISTS plan_revisions;
DROP TABLE IF EXISTS plan_tasks;
DROP TABLE IF EXISTS plans;
DROP TABLE IF EXISTS learner_concept_state;
DROP TABLE IF EXISTS attempts;
DROP TABLE IF EXISTS session_items;
DROP TABLE IF EXISTS practice_sessions;
DROP TABLE IF EXISTS question_versions;
DROP TABLE IF EXISTS questions;
DROP TABLE IF EXISTS curriculum_nodes;
DROP TABLE IF EXISTS exams;
DROP TABLE IF EXISTS auth_sessions;
DROP TABLE IF EXISTS users;
