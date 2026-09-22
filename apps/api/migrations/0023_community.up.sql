-- 0023_community: COMMUNITY-01/02/03, COMP-01/03/04, GROW-01.
-- Everything here is opt-in: no community presence exists until a learner
-- creates a profile with a handle (§16 no-coercive-defaults).

CREATE TABLE IF NOT EXISTS community_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    handle TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS community_groups (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS community_group_members (
    group_id UUID NOT NULL REFERENCES community_groups(id),
    user_id UUID NOT NULL REFERENCES users(id),
    role TEXT NOT NULL DEFAULT 'member', -- moderator | member
    PRIMARY KEY (group_id, user_id)
);

CREATE TABLE IF NOT EXISTS community_posts (
    id UUID PRIMARY KEY,
    group_id UUID NOT NULL REFERENCES community_groups(id),
    author UUID NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'visible', -- visible | removed
    removed_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- COMP-03/GROW-01: private challenges with a shareable token (the deep-link
-- payload for deferred mobile linking).
CREATE TABLE IF NOT EXISTS duels (
    id UUID PRIMARY KEY,
    exam_id UUID NOT NULL REFERENCES exams(id),
    challenger UUID NOT NULL REFERENCES users(id),
    opponent UUID NOT NULL REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'pending', -- pending | active | done | declined
    chapter_id UUID REFERENCES curriculum_nodes(id),
    question_count INT NOT NULL DEFAULT 5,
    share_token TEXT NOT NULL UNIQUE,
    winner UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS duel_sessions (
    duel_id UUID NOT NULL REFERENCES duels(id),
    user_id UUID NOT NULL REFERENCES users(id),
    session_id UUID NOT NULL UNIQUE REFERENCES practice_sessions(id),
    score INT,           -- correct count, set on submit
    total_ms BIGINT,     -- summed elapsed, tie-break
    PRIMARY KEY (duel_id, user_id)
);

-- COMP-01/04: competition cadences + integrity-gated prizes.
ALTER TABLE competitions
    ADD COLUMN IF NOT EXISTS cadence TEXT NOT NULL DEFAULT 'one_off',
    ADD COLUMN IF NOT EXISTS prize_reviewed BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE competition_entries
    ADD COLUMN IF NOT EXISTS flagged BOOLEAN NOT NULL DEFAULT false;
