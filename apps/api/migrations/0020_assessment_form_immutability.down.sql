-- 0020_assessment_form_immutability down.

DO $$
BEGIN
    IF to_regclass('public.assessment_forms') IS NOT NULL THEN
        DROP TRIGGER IF EXISTS assessment_forms_frozen_immutable ON assessment_forms;
    END IF;
END;
$$;

DROP FUNCTION IF EXISTS reject_frozen_assessment_form_mutation();
