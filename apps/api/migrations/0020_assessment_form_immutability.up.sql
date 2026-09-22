-- 0020_assessment_form_immutability: EX-03 frozen assessment forms are append-only.
-- Once frozen_at is set, edits require a new form/version rather than mutating
-- the assessment definition learners may already have started.

CREATE OR REPLACE FUNCTION reject_frozen_assessment_form_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.frozen_at IS NOT NULL THEN
        RAISE EXCEPTION 'frozen assessment forms are immutable; create a new form version'
            USING ERRCODE = '55000';
    END IF;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS assessment_forms_frozen_immutable ON assessment_forms;
CREATE TRIGGER assessment_forms_frozen_immutable
BEFORE UPDATE OR DELETE ON assessment_forms
FOR EACH ROW
EXECUTE FUNCTION reject_frozen_assessment_form_mutation();
