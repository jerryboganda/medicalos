-- 0024_library_cards: SR-03/04/05 card types and trust, LIB-03/04 citation
-- kinds and guideline validity windows, LIB-08 media, IMG-01/02 image cases.
-- Additive only.

ALTER TABLE cards
    ADD COLUMN IF NOT EXISTS card_type TEXT NOT NULL DEFAULT 'basic',
    ADD COLUMN IF NOT EXISTS trust TEXT NOT NULL DEFAULT 'editorial',
    ADD COLUMN IF NOT EXISTS cloze TEXT;

ALTER TABLE article_citations
    ADD COLUMN IF NOT EXISTS kind TEXT NOT NULL DEFAULT 'source';

ALTER TABLE article_versions
    ADD COLUMN IF NOT EXISTS effective_from DATE,
    ADD COLUMN IF NOT EXISTS effective_to DATE;

CREATE TABLE IF NOT EXISTS media_assets (
    id UUID PRIMARY KEY,
    article_id UUID NOT NULL REFERENCES articles(id),
    url TEXT NOT NULL,
    kind TEXT NOT NULL, -- audio | video
    duration_seconds INT,
    captions JSONB NOT NULL DEFAULT '[]',
    chapters JSONB NOT NULL DEFAULT '[]',
    rights_ref TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS image_cases (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    kind TEXT NOT NULL, -- still | stack
    images JSONB NOT NULL, -- ordered [{url, rights_ref}]
    findings TEXT NOT NULL,
    modality TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
