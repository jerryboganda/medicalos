CREATE TABLE IF NOT EXISTS institution_program_curriculum (
    program_id UUID NOT NULL REFERENCES institution_programs(id) ON DELETE CASCADE,
    chapter_id UUID NOT NULL REFERENCES curriculum_nodes(id),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (program_id, chapter_id)
);
