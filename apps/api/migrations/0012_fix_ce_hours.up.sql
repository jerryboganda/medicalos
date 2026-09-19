-- 0012: hours as REAL (NUMERIC needs the bigdecimal sqlx feature).
ALTER TABLE ce_activities ALTER COLUMN hours TYPE REAL;
