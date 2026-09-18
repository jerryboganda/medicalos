-- 0003_entitlements down.
ALTER TABLE users DROP COLUMN IF EXISTS tier;
