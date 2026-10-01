-- Runtime student delivery must honor the same current display grant that
-- publication checked. Keep expiry, revocation, audience, seat, and asset-scope
-- rules in one predicate for every question-serving route.
CREATE OR REPLACE FUNCTION question_display_rights_active(
    requested_ref TEXT,
    question_source_ref TEXT,
    question_source_refs TEXT[],
    question_media_refs TEXT[]
) RETURNS BOOLEAN
LANGUAGE SQL
STABLE
AS $function$
    SELECT EXISTS (
        SELECT 1
        FROM content_rights rights
        WHERE rights.ref_code = UPPER(BTRIM(requested_ref))
          AND rights.revoked_at IS NULL
          AND rights.valid_from <= CURRENT_DATE
          AND (rights.valid_to IS NULL OR rights.valid_to >= CURRENT_DATE)
          AND rights.permitted_uses @> '["display"]'::jsonb
          AND (
              rights.audiences = '[]'::jsonb
              OR EXISTS (
                  SELECT 1
                  FROM jsonb_array_elements_text(rights.audiences) AS audience(value)
                  WHERE LOWER(BTRIM(audience.value)) IN ('learners', 'all')
              )
          )
          -- Seat allocation is not implemented, so capped grants fail closed.
          AND rights.seat_limit IS NULL
          AND rights.asset_refs @> jsonb_build_array(BTRIM(question_source_ref))
          AND NOT EXISTS (
              SELECT 1
              FROM unnest(
                  COALESCE(question_source_refs, ARRAY[]::TEXT[])
                  || COALESCE(question_media_refs, ARRAY[]::TEXT[])
              ) AS asset_ref(value)
              WHERE BTRIM(asset_ref.value) = ''
                 OR NOT rights.asset_refs @> jsonb_build_array(BTRIM(asset_ref.value))
          )
    );
$function$;
