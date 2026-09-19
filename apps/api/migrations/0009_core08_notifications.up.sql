-- CORE-08: learner notification preferences, mobile push registration,
-- and the server-authoritative in-app inbox foundation.

CREATE TABLE IF NOT EXISTS notification_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    quiet_start TIME,
    quiet_end TIME,
    plan_review_reminders BOOLEAN NOT NULL DEFAULT true,
    mock_assignment BOOLEAN NOT NULL DEFAULT true,
    competition BOOLEAN NOT NULL DEFAULT true,
    duel_invitation BOOLEAN NOT NULL DEFAULT true,
    report_resolved BOOLEAN NOT NULL DEFAULT true,
    subscription_events BOOLEAN NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT notification_quiet_hours_pair CHECK (
        (quiet_start IS NULL AND quiet_end IS NULL)
        OR (quiet_start IS NOT NULL AND quiet_end IS NOT NULL)
    )
);

CREATE TABLE IF NOT EXISTS push_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('ios', 'android')),
    token TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, device_id, platform)
);

CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category TEXT NOT NULL CHECK (
        category IN (
            'plan_review_reminders',
            'mock_assignment',
            'competition',
            'duel_invitation',
            'report_resolved',
            'subscription_events'
        )
    ),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    deep_link TEXT NOT NULL CHECK (deep_link LIKE '/%' AND deep_link NOT LIKE '//%'),
    campaign_key TEXT,
    promotional BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_notifications_user_created
    ON notifications (user_id, created_at DESC);
