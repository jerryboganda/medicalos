# EX-07 — Mock test types and time analysis

Status: ready-for-agent
Requirement IDs: EX-07, QB-17, ADMIN-06
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§11.7, 19.5; `docs/requirements/traceability.md`

## Problem Statement

Mocks currently default to an unclassified form with no type categorization, and session results report only accuracy without time analysis. Additionally, administrators have no interface in the editorial console to build or configure mocks according to blueprint quotas and test types, requiring direct API scripts.

## Solution

Complete the pending tails of EX-07:
1. Support the seven mock types defined in master plan §11.7 (`full`, `mini`, `subject`, `system`, `chapter`, `grand_test`, `final_assessment`) on mock creation and listing.
2. Provide time analysis on mock session submission receipts, including total elapsed time, average time per question, and per-chapter time breakdown alongside accuracy.
3. Add a dedicated Mock Builder panel in the administrative editorial console (`/admin`), enabling administrators to configure mock blueprints, timing, pass marks, and integrity policies, and view existing mocks.
4. Display the mock type badge in the learner practice mock selector and display elapsed time and pacing analysis on the completed mock result summary.

## User Stories

1. As an administrator, I want to assign a mock type (`full`, `mini`, `subject`, `system`, `chapter`, `grand_test`, `final_assessment`) when creating a mock, so that the assessment's curricular scope is clear.
2. As an administrator, I want invalid mock types rejected at the API boundary, so that test forms adhere strictly to the master plan's taxonomy.
3. As an administrator, I want to build and configure mocks directly in the editorial console, selecting exams, blueprint chapter counts, time limits, pass marks, and integrity policies without writing raw HTTP requests.
4. As an administrator, I want to see existing configured mocks in the console with their types, parameters, and creation timestamps, so that I can audit available test forms.
5. As a learner, I want to see the mock type chip on practice cards, so that I know whether an assessment is a quick mini-mock, a subject review, or a full exam simulation.
6. As a learner, I want to see my total elapsed time and average time per question on mock completion, so that I can evaluate my pacing against exam time constraints.
7. As a learner, I want per-chapter time breakdowns on mock results, so that I know which domains consumed the most time during the test.
8. As a maintainer, I want database schema migrations with fully tested rollbacks, so that schema changes remain safe and reversible.

## Implementation Decisions

- Add a `mock_type` column to `mocks` via migration `0057_ex07_mock_types_and_time.up.sql` with a check constraint for `'full'`, `'mini'`, `'subject'`, `'system'`, `'chapter'`, `'grand_test'`, `'final_assessment'`. Default existing and unspecified mocks to `'full'`. Provide a matching `.down.sql` rollback.
- Validate `mock_type` in `CreateMockReq` (`apps/api/src/routes/mock.rs`), defaulting to `"full"`.
- Return `mock_type` in `list_mocks` and mock start responses.
- In `apps/api/src/routes/practice.rs` on mock submission, calculate `total_time_seconds` and `avg_time_per_question_seconds` from non-null attempts on the session, and include `time_seconds` in each chapter's entry in `breakdown`.
- In `apps/client/src/lib/api.ts`, update `MockTest` to include `mock_type`, update `MockResult` to include `total_time_seconds`, `avg_time_per_question_seconds`, and `breakdown[].time_seconds`, and expose `createMock` on `Api`.
- In `apps/client/src/routes/admin/+page.svelte`, add a Hallmark-compliant Mock Builder section with exam picker, mock type selector, blueprint chapter quota builder, timing/policy fields, and a list of existing mocks.
- In `apps/client/src/routes/practice/+page.svelte`, display the formatted `mock_type` chip on each mock card.
- In `apps/client/src/routes/session/[id]/+page.svelte`, display the pacing metrics (total time, average time per question, and per-chapter time) inside the mock result card.

## Testing Decisions

- Add authenticated API integration tests in `apps/api/tests/integration.rs` verifying:
  - Admin creation of mocks with explicit types and rejection of invalid types.
  - `list_mocks` returns `mock_type`.
  - Session submission for a mock computes and returns `total_time_seconds`, `avg_time_per_question_seconds`, and chapter `time_seconds`.
- Add Playwright E2E coverage in `tests/e2e/admin-mock-builder.spec.ts` asserting:
  - Admin mock builder UI renders, validates inputs, and submits new mock configurations.
  - Learner practice view shows the mock type badge.
- Test execution is deferred until the final implementation slice per user instruction.

## Out of Scope

- Adaptive item selection or dynamic difficulty adjustment (mocks use frozen blueprint snapshots per EX-03/EX-07).
- Changing scoring algorithms, pass marks, or entitlement tiers.
- Automated proctoring or video feeds.
