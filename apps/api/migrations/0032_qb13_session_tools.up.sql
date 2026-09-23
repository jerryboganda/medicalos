-- QB-13: authored tutor hints and durable evidence that a hint was viewed.
ALTER TABLE question_versions
    ADD COLUMN IF NOT EXISTS hint TEXT;

ALTER TABLE session_items
    ADD COLUMN IF NOT EXISTS hint_used BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE attempts
    ADD COLUMN IF NOT EXISTS offline_recorded_at TIMESTAMPTZ;

CREATE OR REPLACE FUNCTION preserve_hint_assistance() RETURNS trigger AS $$
DECLARE
    viewed_hint BOOLEAN;
BEGIN
    SELECT hint_used INTO viewed_hint
    FROM session_items
    WHERE session_id = NEW.session_id AND item_index = NEW.item_index
    FOR UPDATE;

    NEW.assisted := NEW.assisted OR COALESCE(viewed_hint, FALSE);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS attempts_preserve_hint_assistance ON attempts;
CREATE TRIGGER attempts_preserve_hint_assistance
    BEFORE INSERT ON attempts
    FOR EACH ROW EXECUTE FUNCTION preserve_hint_assistance();
