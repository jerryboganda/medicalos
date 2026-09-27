# SR-08 — Enroll missed practice questions on submission

Status: implementation authored; CI acceptance pending
Triage label: ready-for-human
Requirement IDs: SR-08
Source: `.scratch/sr08-retest-enrollment/spec.md`; `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §13

## Problem

Practice-session submission counts wrong and skipped questions but never adds them to `retest_cards`. The queue is populated only when a learner explicitly posts a re-test result.

## Acceptance

- First submission schedules missing/skipped, wrong, unsure, and assisted items with `passes = 0` and the configured first valid interval.
- Correct, sure, unassisted answers do not enter the queue.
- Unpublished question versions are not newly scheduled.
- Enrollment is atomic with the first session submission; replaying or concurrently retrying a completed submission does not move the due time or create another card.
- The existing re-test endpoint continues to prefer an unseen published family variant and retains its current response contract.
- Authenticated API regressions cover each behavior; GitHub Actions passes the API test and migration/query gates.

## Seams

- `POST /v1/practice/sessions/{sid}/submit` for finalization and automatic enrollment.
- `GET /v1/me/retests` for observable scheduled items.

## Out of scope

UI work, push reminders, new migrations unless the existing card schema cannot represent this behavior, and changes to explicit re-test grading.
