-- 0003_entitlements: CORE-03/COM-01 minimal tier + free allowance anchor.
ALTER TABLE users ADD COLUMN IF NOT EXISTS tier TEXT NOT NULL DEFAULT 'free';
