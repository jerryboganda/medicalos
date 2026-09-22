ALTER TABLE competition_entries DROP COLUMN IF EXISTS flagged;
ALTER TABLE competitions
    DROP COLUMN IF EXISTS prize_reviewed,
    DROP COLUMN IF EXISTS cadence;
DROP TABLE IF EXISTS duel_sessions CASCADE;
DROP TABLE IF EXISTS duels CASCADE;
DROP TABLE IF EXISTS community_posts CASCADE;
DROP TABLE IF EXISTS community_group_members CASCADE;
DROP TABLE IF EXISTS community_groups CASCADE;
DROP TABLE IF EXISTS community_profiles CASCADE;
