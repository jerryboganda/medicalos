CREATE TABLE IF NOT EXISTS community_post_reports (
    id UUID PRIMARY KEY,
    group_id UUID NOT NULL REFERENCES community_groups(id),
    post_id UUID NOT NULL REFERENCES community_posts(id),
    reporter_id UUID NOT NULL REFERENCES users(id),
    reason TEXT NOT NULL CHECK (reason IN ('spam', 'harassment', 'medical_misinformation', 'other')),
    note TEXT CHECK (note IS NULL OR char_length(note) <= 500),
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'dismissed', 'post_removed')),
    resolved_by UUID REFERENCES users(id),
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (post_id, reporter_id)
);

CREATE INDEX IF NOT EXISTS community_post_reports_queue_idx
    ON community_post_reports (group_id, status, created_at);
