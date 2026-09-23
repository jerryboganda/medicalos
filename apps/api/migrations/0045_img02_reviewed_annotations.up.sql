-- IMG-02: image locations are immutable teaching notes; review decisions are separate and final.
CREATE TABLE IF NOT EXISTS image_case_annotations (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES image_cases(id),
    image_index INTEGER NOT NULL CHECK (image_index >= 0),
    x_percent DOUBLE PRECISION NOT NULL CHECK (x_percent BETWEEN 0 AND 100),
    y_percent DOUBLE PRECISION NOT NULL CHECK (y_percent BETWEEN 0 AND 100),
    body TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 1000),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_image_case_annotations_case
    ON image_case_annotations (case_id, image_index, created_at);

CREATE TABLE IF NOT EXISTS image_case_annotation_reviews (
    annotation_id UUID PRIMARY KEY REFERENCES image_case_annotations(id),
    reviewer_id UUID NOT NULL REFERENCES users(id),
    decision TEXT NOT NULL CHECK (decision IN ('approved', 'rejected')),
    note TEXT CHECK (note IS NULL OR char_length(note) <= 500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION reject_image_annotation_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'image annotations and review decisions are immutable'
        USING ERRCODE = '55000';
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS image_case_annotations_immutable ON image_case_annotations;
CREATE TRIGGER image_case_annotations_immutable
BEFORE UPDATE OR DELETE ON image_case_annotations
FOR EACH ROW
EXECUTE FUNCTION reject_image_annotation_mutation();

DROP TRIGGER IF EXISTS image_case_annotation_reviews_immutable ON image_case_annotation_reviews;
CREATE TRIGGER image_case_annotation_reviews_immutable
BEFORE UPDATE OR DELETE ON image_case_annotation_reviews
FOR EACH ROW
EXECUTE FUNCTION reject_image_annotation_mutation();
