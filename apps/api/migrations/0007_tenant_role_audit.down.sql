-- 0007_tenant_role_audit down.
DROP TRIGGER IF EXISTS audit_events_append_only ON audit_events;
DROP FUNCTION IF EXISTS reject_audit_event_mutation();
DROP TABLE IF EXISTS audit_events;
DROP TABLE IF EXISTS platform_roles;
DROP TABLE IF EXISTS tenant_membership_roles;
DROP TABLE IF EXISTS tenant_memberships;
DROP TABLE IF EXISTS tenants;
