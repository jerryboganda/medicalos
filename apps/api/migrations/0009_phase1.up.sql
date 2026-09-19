-- 0009_phase1: Phase-1 completion storage — notes (NOTE-01/02/03), library
-- (LIB-01/03), in-app notifications (CORE-08), guest trials (CORE-09), and
-- versioned goals with protected commitments (CORE-02).

-- NOTE-01: source-linked private notes. Private by default; export (NOTE-03)
-- is a plain JSON dump of the learner's own notes.
CREATE TABLE IF NOT EXISTS notes (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    title TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL DEFAULT '',
    source_question_version_id UUID REFERENCES question_versions(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_notes_user ON notes (user_id, updated_at DESC);

-- NOTE-02: backlinks between a learner's own notes.
CREATE TABLE IF NOT EXISTS note_links (
    id UUID PRIMARY KEY,
    from_note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    to_note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    UNIQUE (from_note_id, to_note_id)
);

CREATE TABLE IF NOT EXISTS note_collections (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    UNIQUE (user_id, name)
);

CREATE TABLE IF NOT EXISTS note_collection_items (
    collection_id UUID NOT NULL REFERENCES note_collections(id) ON DELETE CASCADE,
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    PRIMARY KEY (collection_id, note_id)
);

-- LIB-01: versioned library articles. LIB-03: citation anchors.
CREATE TABLE IF NOT EXISTS articles (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS article_versions (
    id UUID PRIMARY KEY,
    article_id UUID NOT NULL REFERENCES articles(id),
    version INT NOT NULL,
    status TEXT NOT NULL, -- published
    body TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    jurisdiction TEXT,
    UNIQUE (article_id, version)
);

CREATE TABLE IF NOT EXISTS article_citations (
    id UUID PRIMARY KEY,
    version_id UUID NOT NULL REFERENCES article_versions(id),
    anchor TEXT NOT NULL,
    target TEXT NOT NULL
);

-- CORE-08: in-app inbox is the primary channel; push tokens register later
-- (remote push is owned-plugin work, §20.1) — the inbox ships now.
CREATE TABLE IF NOT EXISTS notification_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    plan_reminders BOOLEAN NOT NULL DEFAULT true,
    mock_results BOOLEAN NOT NULL DEFAULT true,
    reports BOOLEAN NOT NULL DEFAULT true,
    quiet_hours_start INT NOT NULL DEFAULT 22,
    quiet_hours_end INT NOT NULL DEFAULT 7
);

CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    category TEXT NOT NULL, -- plan | mock_result | report | competition
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    deep_link TEXT,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications (user_id, created_at DESC);

-- CORE-09: guest trial — opaque client key, bounded sample, no account.
CREATE TABLE IF NOT EXISTS guest_trials (
    id UUID PRIMARY KEY,
    guest_key TEXT NOT NULL UNIQUE,
    questions_served INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- CORE-02: versioned goals + protected commitments (never agent-mutable).
CREATE TABLE IF NOT EXISTS goals (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    exam_id UUID REFERENCES exams(id),
    target_note TEXT NOT NULL,
    exam_date DATE,
    version INT NOT NULL DEFAULT 1,
    retired BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS protected_commitments (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    label TEXT NOT NULL,
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
