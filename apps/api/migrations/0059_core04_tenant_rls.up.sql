-- CORE-04 / §20.3: row-level security as defense in depth on the
-- institution-scoped tables. A least-privilege role (support tools,
-- analytics, background workers) is confined to the tenants listed in the
-- app.institution_ids setting and sees nothing while that setting is
-- absent. The application's owner role keeps its default bypass so the
-- existing handler seams and cross-tenant admin surfaces are unaffected;
-- wiring the app pool onto a least-privilege role is the follow-on slice.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'medos_tenant_viewer') THEN
        CREATE ROLE medos_tenant_viewer NOLOGIN;
    END IF;
END
$$;

GRANT medos_tenant_viewer TO CURRENT_USER;
GRANT USAGE ON SCHEMA public TO medos_tenant_viewer;
GRANT SELECT ON institutions, institution_members, institution_programs,
    external_identities, interoperability_receipts, cohorts, cohort_members
    TO medos_tenant_viewer;

ALTER TABLE institutions ENABLE ROW LEVEL SECURITY;
ALTER TABLE institution_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE institution_programs ENABLE ROW LEVEL SECURITY;
ALTER TABLE external_identities ENABLE ROW LEVEL SECURITY;
ALTER TABLE interoperability_receipts ENABLE ROW LEVEL SECURITY;
ALTER TABLE cohorts ENABLE ROW LEVEL SECURITY;
ALTER TABLE cohort_members ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON institutions
    USING (id = ANY (string_to_array(current_setting('app.institution_ids', true), ',')::uuid[]));
CREATE POLICY tenant_isolation ON institution_members
    USING (institution_id = ANY (string_to_array(current_setting('app.institution_ids', true), ',')::uuid[]));
CREATE POLICY tenant_isolation ON institution_programs
    USING (institution_id = ANY (string_to_array(current_setting('app.institution_ids', true), ',')::uuid[]));
CREATE POLICY tenant_isolation ON external_identities
    USING (institution_id = ANY (string_to_array(current_setting('app.institution_ids', true), ',')::uuid[]));
CREATE POLICY tenant_isolation ON interoperability_receipts
    USING (institution_id = ANY (string_to_array(current_setting('app.institution_ids', true), ',')::uuid[]));
CREATE POLICY tenant_isolation ON cohorts
    USING (institution_id = ANY (string_to_array(current_setting('app.institution_ids', true), ',')::uuid[]));
CREATE POLICY tenant_isolation ON cohort_members
    USING (EXISTS (
        SELECT 1 FROM cohorts c
        WHERE c.id = cohort_id
          AND c.institution_id = ANY (string_to_array(current_setting('app.institution_ids', true), ',')::uuid[])
    ));
