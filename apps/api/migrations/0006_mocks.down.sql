-- 0005_mocks down. Guarded against any starting state.
DROP TABLE IF EXISTS mock_attempts;
ALTER TABLE IF EXISTS practice_sessions DROP COLUMN IF EXISTS mock_id;
DROP TABLE IF EXISTS mocks;
