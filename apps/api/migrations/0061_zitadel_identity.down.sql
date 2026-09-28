-- Reverses 0061_zitadel_identity. Accounts created through Zitadel get an
-- unusable '!' hash so password_hash can be NOT NULL again (they cannot sign
-- in with a password — the same as before this migration existed).
ALTER TABLE institution_members DROP CONSTRAINT IF EXISTS institution_members_role_known;
DROP TABLE IF EXISTS platform_login_states;
ALTER TABLE oidc_login_tickets DROP COLUMN IF EXISTS mfa, DROP COLUMN IF EXISTS roles;
ALTER TABLE auth_sessions DROP COLUMN IF EXISTS mfa, DROP COLUMN IF EXISTS roles;
UPDATE users SET password_hash = '!' WHERE password_hash IS NULL;
ALTER TABLE users ALTER COLUMN password_hash SET NOT NULL;
ALTER TABLE users DROP COLUMN IF EXISTS idp_subject;
