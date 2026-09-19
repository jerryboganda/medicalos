-- LIB-01: stable library identities with immutable numbered published versions.

CREATE TABLE IF NOT EXISTS library_items (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('article', 'reference'))
);

CREATE TABLE IF NOT EXISTS library_versions (
    id UUID PRIMARY KEY,
    library_item_id UUID NOT NULL REFERENCES library_items(id),
    version INT NOT NULL CHECK (version > 0),
    status TEXT NOT NULL CHECK (status IN ('published')),
    title TEXT NOT NULL CHECK (btrim(title) <> ''),
    body TEXT NOT NULL CHECK (btrim(body) <> ''),
    provenance_class TEXT NOT NULL CHECK (btrim(provenance_class) <> ''),
    source_label TEXT NOT NULL CHECK (btrim(source_label) <> ''),
    source_url TEXT,
    effective_date DATE NOT NULL,
    jurisdiction TEXT NOT NULL CHECK (btrim(jurisdiction) <> ''),
    UNIQUE (library_item_id, version)
);

CREATE TABLE IF NOT EXISTS library_version_concepts (
    library_version_id UUID NOT NULL REFERENCES library_versions(id),
    concept_version_id UUID NOT NULL REFERENCES concept_versions(id),
    PRIMARY KEY (library_version_id, concept_version_id)
);

CREATE TABLE IF NOT EXISTS library_version_questions (
    library_version_id UUID NOT NULL REFERENCES library_versions(id),
    question_version_id UUID NOT NULL REFERENCES question_versions(id),
    PRIMARY KEY (library_version_id, question_version_id)
);
