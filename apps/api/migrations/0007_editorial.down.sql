-- 0007_editorial down. Guarded against any starting state.
ALTER TABLE IF EXISTS curriculum_nodes DROP COLUMN IF EXISTS status;
DROP TABLE IF EXISTS audit_events;
DROP TABLE IF EXISTS import_batches;
