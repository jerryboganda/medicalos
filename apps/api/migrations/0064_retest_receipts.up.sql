-- Preserve legacy history without fabricating trusted answer receipts.
ALTER TABLE retest_history
    ADD COLUMN IF NOT EXISTS attempt_id UUID REFERENCES attempts(id),
    ADD COLUMN IF NOT EXISTS result_payload JSONB;
CREATE UNIQUE INDEX IF NOT EXISTS retest_history_attempt_receipt
    ON retest_history (attempt_id) WHERE attempt_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS retest_history_receipt_key
    ON retest_history (user_id, idempotency_key) WHERE attempt_id IS NOT NULL;
ALTER TABLE retest_cards
    ADD COLUMN IF NOT EXISTS enrolled_session_id UUID REFERENCES practice_sessions(id);
