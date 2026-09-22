ALTER TABLE competition_entries DROP COLUMN IF EXISTS flagged;
ALTER TABLE competitions
    DROP COLUMN IF EXISTS prize_reviewed,
    DROP COLUMN IF EXISTS cadence;
DROP TABLE IF EXISTS duel_sessions;
DROP TABLE IF EXISTS duels;
DROP TABLE IF EXISTS community_posts;
DROP TABLE IF EXISTS community_group_members;
DROP TABLE IF EXISTS community_groups;
DROP TABLE IF EXISTS community_profiles;
