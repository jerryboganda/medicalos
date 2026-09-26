# EX-07 — Mock test types, time analysis, and editorial mock builder

Status: authored (implementation and tests authored, CI acceptance pending)
Requirement IDs: EX-07, QB-17, ADMIN-06
Source: `.scratch/ex07-mock-types/spec.md`; `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§11.7, 19.5; `docs/requirements/traceability.md`

## Acceptance

- `CreateMockReq` supports optional `mock_type` with allowed values: `full`, `mini`, `subject`, `system`, `chapter`, `grand_test`, `final_assessment`. Defaults to `full`. Invalid types are rejected with 422 `invalid_mock_type`.
- Database migration `0057_ex07_mock_types_and_time.up.sql` adds `mock_type TEXT NOT NULL DEFAULT 'full'` with matching check constraint to `mocks`, with reversible `down.sql`.
- `GET /v1/mocks` includes `mock_type` in the response for every mock.
- On mock practice session submission, `result.mock` includes:
  - `total_time_seconds`: non-negative integer indicating total seconds elapsed across recorded attempts.
  - `avg_time_per_question_seconds`: average seconds spent per answered question.
  - `breakdown[].time_seconds`: total seconds spent on questions within each curriculum chapter.
- The administrative console (`/admin`) provides a Mock Builder section allowing editors to build mock blueprints, select test types, configure pass marks, time limits, and integrity policies, and review existing mocks.
- The practice session result view renders elapsed time and per-question/per-chapter pacing analysis for mock sessions.
- Learner practice mock list displays the formatted test type chip.
- Authenticated integration tests and browser E2E tests are authored to verify API and UI behaviors.

## Seams

- Authenticated API endpoints: `POST /v1/mocks`, `GET /v1/mocks`, `POST /v1/sessions/:id/submit`.
- Admin console `/admin` and learner pages `/practice`, `/session/:id`.
- Migration rollback/reapply idempotency.
