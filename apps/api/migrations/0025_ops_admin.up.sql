-- 0025_ops_admin: OPS-05 analytics stream, ADMIN-02 rights ledger,
-- ADMIN-04 incidents. Additive only.

-- OPS-05: pseudonymous product-analytics stream. Rule 1 (no direct
-- identifiers) and rule 2 (never mixed with learning evidence) are
-- enforced at the receiving endpoint.
CREATE TABLE IF NOT EXISTS analytics_events (
    id BIGSERIAL PRIMARY KEY,
    anonymous_id TEXT NOT NULL,
    name TEXT NOT NULL,
    properties JSONB NOT NULL DEFAULT '{}',
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_analytics_events_name
    ON analytics_events (name, received_at);

-- ADMIN-02/TRUST-07: the rights ledger (§19.2): licensor, territory,
-- permitted uses, validity. Media and images cite these by ref_code.
CREATE TABLE IF NOT EXISTS content_rights (
    id UUID PRIMARY KEY,
    ref_code TEXT NOT NULL UNIQUE,
    licensor TEXT NOT NULL,
    territory TEXT NOT NULL DEFAULT 'worldwide',
    permitted_uses JSONB NOT NULL DEFAULT '[]',
    valid_from DATE NOT NULL,
    valid_to DATE,
    notes TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ADMIN-04: support incidents with lifecycle.
CREATE TABLE IF NOT EXISTS incidents (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    severity TEXT NOT NULL, -- sev1 | sev2 | sev3
    status TEXT NOT NULL DEFAULT 'open', -- open | mitigated | resolved
    opened_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ
);
