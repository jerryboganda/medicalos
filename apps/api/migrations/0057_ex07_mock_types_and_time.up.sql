-- 0057_ex07_mock_types_and_time: EX-07 mock test types and session time analysis.
ALTER TABLE mocks
    ADD COLUMN IF NOT EXISTS mock_type TEXT NOT NULL DEFAULT 'full'
        CHECK (mock_type IN ('full', 'mini', 'subject', 'system', 'chapter', 'grand_test', 'final_assessment'));
