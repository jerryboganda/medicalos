CREATE TABLE IF NOT EXISTS concepts (
    id UUID PRIMARY KEY,
    canonical_key TEXT NOT NULL UNIQUE
        CHECK (canonical_key ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    current_version INT NOT NULL DEFAULT 1 CHECK (current_version > 0),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS concept_versions (
    concept_id UUID NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    version INT NOT NULL CHECK (version > 0),
    display_name TEXT NOT NULL CHECK (length(btrim(display_name)) BETWEEN 1 AND 200),
    definition TEXT NOT NULL CHECK (length(btrim(definition)) BETWEEN 1 AND 4000),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (concept_id, version)
);

DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'concepts'::regclass
          AND conname = 'concepts_current_version_fk'
    ) THEN
        ALTER TABLE concepts
            ADD CONSTRAINT concepts_current_version_fk
            FOREIGN KEY (id, current_version)
            REFERENCES concept_versions (concept_id, version)
            DEFERRABLE INITIALLY DEFERRED;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS curriculum_node_concepts (
    curriculum_node_id UUID NOT NULL REFERENCES curriculum_nodes(id) ON DELETE CASCADE,
    concept_id UUID NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (curriculum_node_id, concept_id)
);

CREATE INDEX IF NOT EXISTS idx_curriculum_node_concepts_concept
    ON curriculum_node_concepts (concept_id, curriculum_node_id);
