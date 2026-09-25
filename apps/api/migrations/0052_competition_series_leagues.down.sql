DROP INDEX IF EXISTS competition_league_members_cohort_idx;
DROP TABLE IF EXISTS competition_league_memberships;
DROP TABLE IF EXISTS competition_league_cohorts;
DROP TABLE IF EXISTS competition_league_players;
DROP INDEX IF EXISTS competition_series_due_idx;
DROP INDEX IF EXISTS competitions_series_start_unique;
ALTER TABLE competitions DROP COLUMN IF EXISTS series_id;
DROP TABLE IF EXISTS competition_series;
