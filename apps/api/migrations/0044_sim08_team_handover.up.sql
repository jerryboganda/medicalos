-- SIM-08: private teams and structured handovers stay attached to one case run.
CREATE TABLE IF NOT EXISTS scenario_team_members (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES scenario_runs(id),
    user_id UUID NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('team_lead', 'history_taker', 'scribe', 'observer')),
    invited_by UUID NOT NULL REFERENCES users(id),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (run_id, user_id),
    UNIQUE (run_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_scenario_team_one_lead
    ON scenario_team_members (run_id)
    WHERE role = 'team_lead';
CREATE INDEX IF NOT EXISTS idx_scenario_team_user
    ON scenario_team_members (user_id, run_id);

INSERT INTO scenario_team_members (id, run_id, user_id, role, invited_by, joined_at)
SELECT gen_random_uuid(), run.id, run.user_id, 'team_lead', run.user_id, run.started_at
FROM scenario_runs run
ON CONFLICT (run_id, user_id) DO NOTHING;

CREATE TABLE IF NOT EXISTS scenario_team_invites (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES scenario_runs(id),
    created_by UUID NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('history_taker', 'scribe', 'observer')),
    token_sha256 CHAR(64) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_by UUID REFERENCES users(id),
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((accepted_by IS NULL) = (accepted_at IS NULL))
);

CREATE INDEX IF NOT EXISTS idx_scenario_team_invites_run_expiry
    ON scenario_team_invites (run_id, expires_at)
    WHERE accepted_at IS NULL;

CREATE OR REPLACE FUNCTION guard_scenario_team_invite_update()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'scenario team invitations cannot be deleted'
            USING ERRCODE = '55000';
    END IF;

    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.run_id IS DISTINCT FROM OLD.run_id
       OR NEW.created_by IS DISTINCT FROM OLD.created_by
       OR NEW.role IS DISTINCT FROM OLD.role
       OR NEW.token_sha256 IS DISTINCT FROM OLD.token_sha256
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR OLD.accepted_at IS NOT NULL
       OR NEW.accepted_at IS NULL
       OR NEW.accepted_by IS NULL THEN
        RAISE EXCEPTION 'scenario team invitations are immutable except for one acceptance'
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS scenario_team_invites_accept_once ON scenario_team_invites;
CREATE TRIGGER scenario_team_invites_accept_once
BEFORE UPDATE OR DELETE ON scenario_team_invites
FOR EACH ROW
EXECUTE FUNCTION guard_scenario_team_invite_update();

CREATE OR REPLACE FUNCTION reject_scenario_team_record_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'scenario team membership, handovers, and acknowledgements are immutable'
        USING ERRCODE = '55000';
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS scenario_team_members_immutable ON scenario_team_members;
CREATE TRIGGER scenario_team_members_immutable
BEFORE UPDATE OR DELETE ON scenario_team_members
FOR EACH ROW
EXECUTE FUNCTION reject_scenario_team_record_mutation();

CREATE TABLE IF NOT EXISTS scenario_handovers (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES scenario_runs(id),
    from_member_id UUID NOT NULL,
    to_member_id UUID NOT NULL,
    situation TEXT NOT NULL CHECK (char_length(situation) BETWEEN 1 AND 2000),
    background TEXT NOT NULL CHECK (char_length(background) BETWEEN 1 AND 2000),
    assessment TEXT NOT NULL CHECK (char_length(assessment) BETWEEN 1 AND 2000),
    recommendation TEXT NOT NULL CHECK (char_length(recommendation) BETWEEN 1 AND 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (from_member_id <> to_member_id),
    FOREIGN KEY (run_id, from_member_id) REFERENCES scenario_team_members(run_id, id),
    FOREIGN KEY (run_id, to_member_id) REFERENCES scenario_team_members(run_id, id)
);

CREATE INDEX IF NOT EXISTS idx_scenario_handovers_run_created
    ON scenario_handovers (run_id, created_at, id);

CREATE TABLE IF NOT EXISTS scenario_handover_acknowledgements (
    handover_id UUID PRIMARY KEY REFERENCES scenario_handovers(id),
    acknowledged_by UUID NOT NULL REFERENCES users(id),
    acknowledged_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

DROP TRIGGER IF EXISTS scenario_handovers_immutable ON scenario_handovers;
CREATE TRIGGER scenario_handovers_immutable
BEFORE UPDATE OR DELETE ON scenario_handovers
FOR EACH ROW
EXECUTE FUNCTION reject_scenario_team_record_mutation();

DROP TRIGGER IF EXISTS scenario_handover_acknowledgements_immutable ON scenario_handover_acknowledgements;
CREATE TRIGGER scenario_handover_acknowledgements_immutable
BEFORE UPDATE OR DELETE ON scenario_handover_acknowledgements
FOR EACH ROW
EXECUTE FUNCTION reject_scenario_team_record_mutation();
