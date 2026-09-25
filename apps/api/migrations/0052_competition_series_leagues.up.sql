CREATE TABLE IF NOT EXISTS competition_series (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    exam_id UUID NOT NULL REFERENCES exams(id),
    question_pool JSONB NOT NULL CHECK (jsonb_typeof(question_pool) = 'array'),
    question_count INTEGER NOT NULL CHECK (question_count >= 3),
    cadence TEXT NOT NULL CHECK (cadence IN ('daily', 'weekly', 'monthly')),
    monthly_anchor_day SMALLINT NOT NULL CHECK (monthly_anchor_day BETWEEN 1 AND 31),
    monthly_anchor_end BOOLEAN NOT NULL DEFAULT false,
    next_start_at TIMESTAMPTZ NOT NULL,
    duration_seconds BIGINT NOT NULL CHECK (duration_seconds > 0),
    created_by UUID REFERENCES users(id),
    difficulty_points JSONB NOT NULL,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE competitions
    ADD COLUMN IF NOT EXISTS series_id UUID REFERENCES competition_series(id);

CREATE UNIQUE INDEX IF NOT EXISTS competitions_series_start_unique
    ON competitions (series_id, starts_at)
    WHERE series_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS competition_series_due_idx
    ON competition_series (next_start_at)
    WHERE active = true;

CREATE TABLE IF NOT EXISTS competition_league_players (
    exam_id UUID NOT NULL REFERENCES exams(id),
    user_id UUID NOT NULL REFERENCES users(id),
    handle TEXT NOT NULL,
    division INTEGER NOT NULL DEFAULT 1 CHECK (division >= 1),
    active BOOLEAN NOT NULL DEFAULT true,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (exam_id, user_id)
);

CREATE TABLE IF NOT EXISTS competition_league_cohorts (
    id UUID PRIMARY KEY,
    exam_id UUID NOT NULL REFERENCES exams(id),
    week_start DATE NOT NULL,
    division INTEGER NOT NULL CHECK (division >= 1),
    cohort_number INTEGER NOT NULL CHECK (cohort_number >= 1),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (exam_id, week_start, division, cohort_number)
);

CREATE TABLE IF NOT EXISTS competition_league_memberships (
    cohort_id UUID NOT NULL REFERENCES competition_league_cohorts(id),
    exam_id UUID NOT NULL REFERENCES exams(id),
    week_start DATE NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    handle TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at TIMESTAMPTZ,
    PRIMARY KEY (cohort_id, user_id),
    UNIQUE (exam_id, week_start, user_id)
);

CREATE INDEX IF NOT EXISTS competition_league_members_cohort_idx
    ON competition_league_memberships (cohort_id, left_at, user_id);
