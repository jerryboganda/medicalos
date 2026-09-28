DROP POLICY IF EXISTS tenant_isolation ON institutions;
DROP POLICY IF EXISTS tenant_isolation ON institution_members;
DROP POLICY IF EXISTS tenant_isolation ON institution_programs;
DROP POLICY IF EXISTS tenant_isolation ON external_identities;
DROP POLICY IF EXISTS tenant_isolation ON interoperability_receipts;
DROP POLICY IF EXISTS tenant_isolation ON cohorts;
DROP POLICY IF EXISTS tenant_isolation ON cohort_members;

ALTER TABLE institutions DISABLE ROW LEVEL SECURITY;
ALTER TABLE institution_members DISABLE ROW LEVEL SECURITY;
ALTER TABLE institution_programs DISABLE ROW LEVEL SECURITY;
ALTER TABLE external_identities DISABLE ROW LEVEL SECURITY;
ALTER TABLE interoperability_receipts DISABLE ROW LEVEL SECURITY;
ALTER TABLE cohorts DISABLE ROW LEVEL SECURITY;
ALTER TABLE cohort_members DISABLE ROW LEVEL SECURITY;
REVOKE SELECT ON institutions, institution_members, institution_programs,
    external_identities, interoperability_receipts, cohorts, cohort_members
    FROM medos_tenant_viewer;
