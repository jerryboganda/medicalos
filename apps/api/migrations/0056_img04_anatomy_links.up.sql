-- IMG-04: image cases point to stable concepts and pin their immutable version.
CREATE TABLE IF NOT EXISTS image_case_concepts (
    case_id UUID NOT NULL REFERENCES image_cases(id) ON DELETE CASCADE,
    concept_id UUID NOT NULL,
    concept_version INT NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (case_id, concept_id),
    FOREIGN KEY (concept_id, concept_version)
        REFERENCES concept_versions (concept_id, version) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_image_case_concepts_concept
    ON image_case_concepts (concept_id, concept_version, case_id);
