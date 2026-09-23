DROP TRIGGER IF EXISTS document_extraction_reviews_immutable ON document_extraction_reviews;
DROP TRIGGER IF EXISTS document_extraction_reports_immutable ON document_extraction_reports;
DROP FUNCTION IF EXISTS reject_extraction_evidence_mutation();
DROP TABLE IF EXISTS document_extraction_reviews;
DROP TABLE IF EXISTS document_extraction_reports;
