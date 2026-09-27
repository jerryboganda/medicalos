# SR-08 — Automatic enrollment of missed questions in the re-test queue

Status: implementation authored; CI acceptance pending
Requirement IDs: SR-08
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §13

## Problem Statement

The re-test API schedules a card only after a learner submits a re-test result. Practice-session submission does not automatically schedule questions the learner answered incorrectly, skipped, guessed, or answered with assistance, so the recovery queue misses the exact items it is meant to resurface.

## Solution

When an open practice session is first submitted, add every eligible question to the learner's existing re-test scheduler in the same database transaction as session completion. Reuse the configured first interval and existing family-variant serving behavior. Keep the current response contracts and routes.

## User Stories

1. As a learner, I want incorrect answers scheduled for re-testing, so that weak areas return for retrieval practice.
2. As a learner, I want skipped questions scheduled too, so that omissions do not disappear from my review plan.
3. As a learner, I want uncertain or assisted answers scheduled, so that guessed and hint-supported responses are not treated as mastered.
4. As a learner, I want clearly correct, sure, unassisted answers left out of the re-test queue, so that the queue reflects questions needing more practice.
5. As a learner, I want submitting a session more than once to leave its schedule unchanged, so that retries do not duplicate or postpone review work.
6. As a learner, I want re-test questions to continue preferring unseen published variants from the same family, so that review checks transfer rather than repeating the identical item.

## Implementation Decisions

- Use the authenticated practice-session submission endpoint as the single enrollment seam; do not add routes or UI.
- Enroll items with no attempt, a null chosen option, an incorrect answer, `unsure` confidence, or server-recorded/client-declared assistance. A viewed hint is already recorded as assistance.
- Set the card's pass count to zero and its due time to the first valid configured re-test interval.
- Enroll only published question versions. Preserve the existing learner/question uniqueness and family-variant selection rules.
- Perform enrollment only for the first open-to-submitted transition and in the same transaction as that transition, so duplicate submissions cannot reschedule cards and a failure cannot leave a half-completed submission.
- Do not change answer scoring, mock answer editing, retest-result grading, or existing wire response shapes.

## Testing Decisions

- Test through the authenticated HTTP seam with the existing Postgres integration harness.
- Verify that incorrect, skipped, unsure, and assisted items become visible through the existing re-test endpoint after their due time; verify a correct sure unassisted item does not.
- Verify the configured interval, published-content filtering, family-variant preference, and duplicate submission behavior.
- Use database writes to prepare synthetic question/settings fixtures and arrange due-time fixtures; assert observable learner queue results over HTTP.
- GitHub Actions remains the build and test acceptance gate.

## Out of Scope

- New learner-facing queue screens, reminders, notification delivery, or changes to FSRS/card-review behavior.
- Changing scoring or assessment policies for mock, timed, or tutor sessions.
- Automatically advancing a re-test card from an ordinary practice answer; existing explicit re-test results remain authoritative for pass progression.

## Further Notes

The current integration seam and adjacent prior art are the SR-08 re-test cases in `apps/api/tests/integration.rs`; the master-plan rule is §13's question re-test queue.
