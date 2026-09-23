-- LIB-05: explicit source-to-version dependencies and audited change cases.
-- Scenario runs pin a version so an edit or quarantine cannot rewrite history.

CREATE TABLE IF NOT EXISTS scenario_versions (
    id UUID PRIMARY KEY,
    scenario_id UUID NOT NULL REFERENCES scenarios(id),
    version INT NOT NULL,
    status TEXT NOT NULL DEFAULT 'published',
    state_machine JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (scenario_id, version)
);

INSERT INTO scenario_versions (id, scenario_id, version, status, state_machine, created_at)
SELECT gen_random_uuid(), id, version, status, state_machine, created_at
FROM scenarios
ON CONFLICT (scenario_id, version) DO NOTHING;

ALTER TABLE scenario_runs
    ADD COLUMN IF NOT EXISTS scenario_version_id UUID REFERENCES scenario_versions(id);

UPDATE scenario_runs r
SET scenario_version_id = sv.id
FROM scenarios s
JOIN scenario_versions sv ON sv.scenario_id = s.id AND sv.version = s.version
WHERE r.scenario_id = s.id AND r.scenario_version_id IS NULL;

ALTER TABLE scenario_runs
    ALTER COLUMN scenario_version_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_scenario_runs_version
    ON scenario_runs (scenario_version_id, user_id);

CREATE TABLE IF NOT EXISTS article_reads (
    user_id UUID NOT NULL REFERENCES users(id),
    article_version_id UUID NOT NULL REFERENCES article_versions(id),
    first_read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, article_version_id)
);

CREATE TABLE IF NOT EXISTS source_passages (
    id UUID PRIMARY KEY,
    source_ref TEXT NOT NULL,
    locator TEXT NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (source_ref, locator)
);

CREATE TABLE IF NOT EXISTS source_dependencies (
    id UUID PRIMARY KEY,
    source_passage_id UUID NOT NULL REFERENCES source_passages(id),
    resource_kind TEXT NOT NULL CHECK (resource_kind IN ('question', 'article', 'scenario')),
    question_version_id UUID REFERENCES question_versions(id),
    article_version_id UUID REFERENCES article_versions(id),
    scenario_version_id UUID REFERENCES scenario_versions(id),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (
        (resource_kind = 'question' AND question_version_id IS NOT NULL
         AND article_version_id IS NULL AND scenario_version_id IS NULL)
        OR (resource_kind = 'article' AND article_version_id IS NOT NULL
            AND question_version_id IS NULL AND scenario_version_id IS NULL)
        OR (resource_kind = 'scenario' AND scenario_version_id IS NOT NULL
            AND question_version_id IS NULL AND article_version_id IS NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_source_dependencies_question
    ON source_dependencies (source_passage_id, question_version_id)
    WHERE question_version_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_source_dependencies_article
    ON source_dependencies (source_passage_id, article_version_id)
    WHERE article_version_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_source_dependencies_scenario
    ON source_dependencies (source_passage_id, scenario_version_id)
    WHERE scenario_version_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS source_change_events (
    id UUID PRIMARY KEY,
    source_passage_id UUID NOT NULL REFERENCES source_passages(id),
    source_revision TEXT NOT NULL,
    classification TEXT NOT NULL CHECK (
        classification IN ('minor_typo', 'material_change', 'invalid_answer_key', 'source_withdrawn')
    ),
    note TEXT NOT NULL,
    actor_id UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (source_passage_id, source_revision)
);

CREATE TABLE IF NOT EXISTS source_change_tasks (
    id UUID PRIMARY KEY,
    event_id UUID NOT NULL REFERENCES source_change_events(id),
    dependency_id UUID NOT NULL REFERENCES source_dependencies(id),
    resource_kind TEXT NOT NULL CHECK (resource_kind IN ('question', 'article', 'scenario')),
    question_version_id UUID REFERENCES question_versions(id),
    article_version_id UUID REFERENCES article_versions(id),
    scenario_version_id UUID REFERENCES scenario_versions(id),
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved')),
    resolution TEXT CHECK (resolution IN ('reviewed_current', 'corrected', 'retired')),
    resolution_note TEXT NOT NULL DEFAULT '',
    corrected_question_version_id UUID REFERENCES question_versions(id),
    resolved_by UUID REFERENCES users(id),
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (event_id, dependency_id),
    CHECK (
        (resource_kind = 'question' AND question_version_id IS NOT NULL
         AND article_version_id IS NULL AND scenario_version_id IS NULL)
        OR (resource_kind = 'article' AND article_version_id IS NOT NULL
            AND question_version_id IS NULL AND scenario_version_id IS NULL)
        OR (resource_kind = 'scenario' AND scenario_version_id IS NOT NULL
            AND question_version_id IS NULL AND article_version_id IS NULL)
    ),
    CHECK (
        (status = 'open' AND resolution IS NULL
         AND corrected_question_version_id IS NULL AND resolved_by IS NULL
         AND resolved_at IS NULL)
        OR (status = 'resolved' AND resolution IS NOT NULL
            AND resolved_by IS NOT NULL AND resolved_at IS NOT NULL
            AND ((resolution = 'corrected' AND corrected_question_version_id IS NOT NULL)
                 OR (resolution <> 'corrected' AND corrected_question_version_id IS NULL)))
    )
);

CREATE INDEX IF NOT EXISTS idx_source_change_tasks_event
    ON source_change_tasks (event_id, created_at);

CREATE TABLE IF NOT EXISTS source_change_task_learners (
    task_id UUID NOT NULL REFERENCES source_change_tasks(id),
    user_id UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (task_id, user_id)
);

CREATE TABLE IF NOT EXISTS attempt_corrections (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES source_change_tasks(id),
    attempt_id UUID NOT NULL REFERENCES attempts(id),
    old_correct BOOLEAN,
    new_correct BOOLEAN,
    corrected_question_version_id UUID NOT NULL REFERENCES question_versions(id),
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (task_id, attempt_id)
);

ALTER TABLE notification_preferences
    ADD COLUMN IF NOT EXISTS content_updates BOOLEAN NOT NULL DEFAULT true;
