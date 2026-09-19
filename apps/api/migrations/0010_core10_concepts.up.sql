-- CORE-10: stable learning concepts are separate from curriculum placement.
-- Published concept versions anchor content to an immutable numbered meaning.

CREATE TABLE IF NOT EXISTS concepts (
    id UUID PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS concept_versions (
    id UUID PRIMARY KEY,
    concept_id UUID NOT NULL REFERENCES concepts(id),
    version INT NOT NULL CHECK (version > 0),
    status TEXT NOT NULL CHECK (status IN ('published')),
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    definition TEXT NOT NULL CHECK (btrim(definition) <> ''),
    UNIQUE (concept_id, version)
);

CREATE TABLE IF NOT EXISTS curriculum_node_concepts (
    node_id UUID NOT NULL REFERENCES curriculum_nodes(id),
    concept_version_id UUID NOT NULL REFERENCES concept_versions(id),
    PRIMARY KEY (node_id, concept_version_id)
);

CREATE TABLE IF NOT EXISTS question_version_concepts (
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    concept_version_id UUID NOT NULL REFERENCES concept_versions(id),
    relation TEXT NOT NULL CHECK (relation IN ('primary', 'secondary')),
    PRIMARY KEY (question_version_id, concept_version_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_question_version_one_primary_concept
    ON question_version_concepts (question_version_id)
    WHERE relation = 'primary';
