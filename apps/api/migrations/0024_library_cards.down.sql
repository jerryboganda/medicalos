DROP TABLE IF EXISTS image_cases;
DROP TABLE IF EXISTS media_assets;
ALTER TABLE article_versions
    DROP COLUMN IF EXISTS effective_to,
    DROP COLUMN IF EXISTS effective_from;
ALTER TABLE article_citations DROP COLUMN IF EXISTS kind;
ALTER TABLE cards
    DROP COLUMN IF EXISTS cloze,
    DROP COLUMN IF EXISTS trust,
    DROP COLUMN IF EXISTS card_type;
