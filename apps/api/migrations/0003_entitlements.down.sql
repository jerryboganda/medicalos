-- 0003_entitlements down. Guarded like 0002 (see issue-tracker notes).
ALTER TABLE IF EXISTS users DROP COLUMN IF EXISTS tier;
