-- INST-06: LTI 1.3 tool side — registered platforms, one-use launch states,
-- linked LMS identities, and pending deep-linking requests.
CREATE TABLE IF NOT EXISTS lti_platforms (
    id UUID PRIMARY KEY,
    institution_id UUID NOT NULL REFERENCES institutions(id) ON DELETE CASCADE,
    issuer TEXT NOT NULL,
    client_id TEXT NOT NULL,
    deployment_id TEXT NOT NULL,
    auth_login_url TEXT NOT NULL,
    key_set_url TEXT NOT NULL,
    display_name TEXT NOT NULL DEFAULT 'LMS',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (issuer, client_id, deployment_id)
);

CREATE TABLE IF NOT EXISTS lti_launch_states (
    state TEXT PRIMARY KEY,
    nonce TEXT NOT NULL,
    platform_id UUID NOT NULL REFERENCES lti_platforms(id) ON DELETE CASCADE,
    target_link_uri TEXT,
    message_type TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS lti_identities (
    platform_id UUID NOT NULL REFERENCES lti_platforms(id) ON DELETE CASCADE,
    subject TEXT NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    linked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (platform_id, subject)
);

CREATE TABLE IF NOT EXISTS lti_deep_link_pends (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    platform_id UUID NOT NULL REFERENCES lti_platforms(id) ON DELETE CASCADE,
    settings JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
