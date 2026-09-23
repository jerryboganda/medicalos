DROP TABLE IF EXISTS curriculum_node_concepts;
ALTER TABLE concepts DROP CONSTRAINT IF EXISTS concepts_current_version_fk;
DROP TABLE IF EXISTS concept_versions;
DROP TABLE IF EXISTS concepts;
