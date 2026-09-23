-- LIB-07: content-free extraction coverage reports and independent reviews.
CREATE TABLE IF NOT EXISTS document_extraction_reports (
    id UUID PRIMARY KEY,
    source_label TEXT NOT NULL CHECK (btrim(source_label) <> '' AND char_length(source_label) <= 240),
    source_sha256 TEXT NOT NULL CHECK (source_sha256 ~ '^[a-f0-9]{64}$'),
    media_type TEXT NOT NULL CHECK (media_type IN (
        'application/pdf',
        'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
        'application/vnd.openxmlformats-officedocument.presentationml.presentation',
        'application/epub+zip',
        'text/html',
        'image/jpeg',
        'image/png',
        'image/tiff',
        'image/webp',
        'text/plain'
    )),
    parser_version TEXT NOT NULL CHECK (btrim(parser_version) <> '' AND char_length(parser_version) <= 100),
    rights_id UUID NOT NULL REFERENCES content_rights(id) ON DELETE RESTRICT,
    malware_scan_status TEXT NOT NULL CHECK (malware_scan_status IN ('clean', 'blocked', 'not_scanned')),
    expected_regions JSONB NOT NULL CHECK (CASE WHEN jsonb_typeof(expected_regions) = 'array'
        THEN jsonb_array_length(expected_regions) BETWEEN 1 AND 5000 ELSE FALSE END),
    extracted_regions JSONB NOT NULL CHECK (CASE WHEN jsonb_typeof(extracted_regions) = 'array'
        THEN jsonb_array_length(extracted_regions) <= 5000 ELSE FALSE END),
    uncertain_regions JSONB NOT NULL CHECK (CASE WHEN jsonb_typeof(uncertain_regions) = 'array'
        THEN jsonb_array_length(uncertain_regions) <= 5000 ELSE FALSE END),
    critical_regions JSONB NOT NULL CHECK (CASE WHEN jsonb_typeof(critical_regions) = 'array'
        THEN jsonb_array_length(critical_regions) <= 5000 ELSE FALSE END),
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_extraction_reports_created
    ON document_extraction_reports (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_extraction_reports_checksum
    ON document_extraction_reports (source_sha256, parser_version);

CREATE TABLE IF NOT EXISTS document_extraction_reviews (
    id UUID PRIMARY KEY,
    report_id UUID NOT NULL UNIQUE REFERENCES document_extraction_reports(id) ON DELETE RESTRICT,
    reviewer_id UUID REFERENCES users(id) ON DELETE SET NULL,
    decision TEXT NOT NULL CHECK (decision IN ('approved', 'rejected')),
    verified_regions JSONB NOT NULL CHECK (CASE WHEN jsonb_typeof(verified_regions) = 'array'
        THEN jsonb_array_length(verified_regions) <= 5000 ELSE FALSE END),
    note TEXT NOT NULL CHECK (char_length(btrim(note)) BETWEEN 10 AND 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION reject_extraction_evidence_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'extraction reports and review decisions are immutable; create a new record'
        USING ERRCODE = '55000';
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS document_extraction_reports_immutable ON document_extraction_reports;
CREATE TRIGGER document_extraction_reports_immutable
BEFORE UPDATE OR DELETE ON document_extraction_reports
FOR EACH ROW
EXECUTE FUNCTION reject_extraction_evidence_mutation();

DROP TRIGGER IF EXISTS document_extraction_reviews_immutable ON document_extraction_reviews;
CREATE TRIGGER document_extraction_reviews_immutable
BEFORE UPDATE OR DELETE ON document_extraction_reviews
FOR EACH ROW
EXECUTE FUNCTION reject_extraction_evidence_mutation();
