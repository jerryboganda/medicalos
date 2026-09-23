DROP TRIGGER IF EXISTS image_case_annotation_reviews_immutable ON image_case_annotation_reviews;
DROP TRIGGER IF EXISTS image_case_annotations_immutable ON image_case_annotations;
DROP FUNCTION IF EXISTS reject_image_annotation_mutation();
DROP TABLE IF EXISTS image_case_annotation_reviews;
DROP TABLE IF EXISTS image_case_annotations;
