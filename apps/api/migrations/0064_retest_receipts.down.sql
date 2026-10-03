ALTER TABLE retest_cards DROP COLUMN IF EXISTS enrolled_session_id;
DROP INDEX IF EXISTS retest_history_receipt_key;
DROP INDEX IF EXISTS retest_history_attempt_receipt;
ALTER TABLE retest_history
    DROP COLUMN IF EXISTS result_payload,
    DROP COLUMN IF EXISTS attempt_id;
