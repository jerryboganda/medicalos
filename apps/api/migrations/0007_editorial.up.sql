-- 0007_editorial: ADMIN-06 baseline — bulk-import batches + audit trail,
-- plus node status for hierarchy management (§5.5).
CREATE TABLE IF NOT EXISTS import_batches (
    id UUID PRIMARY KEY,
    created_by UUID NOT NULL REFERENCES users(id),
    exam_id UUID NOT NULL REFERENCES exams(id),
    filename TEXT NOT NULL DEFAULT 'inline',
    status TEXT NOT NULL, -- dry_run | applied | rolled_back
    summary JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit_events (
    id UUID PRIMARY KEY,
    actor UUID REFERENCES users(id),
    action TEXT NOT NULL,
    entity TEXT NOT NULL,
    entity_id UUID,
    old_value JSONB,
    new_value JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE curriculum_nodes ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active';
