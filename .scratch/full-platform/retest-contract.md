Status: implemented; CI acceptance pending
Requirement: SR-08 objective re-test grading

## Problem Statement

The result endpoint currently accepts a learner-supplied correctness boolean.
A caller can manufacture successful re-tests without answering a question,
and alternate idempotency keys can multiply the same supposed success.

## Solution

Require a submitted practice session and item as the durable answer receipt.
The API derives grading from its stored answer and published question key,
records each attempt at most once, and commits history and scheduling together.

## User Stories

1. As a learner, I want re-test progress to reflect answers I actually submitted.
2. As a learner, I want hinted, assisted, uncertain or skipped answers to remain
   revision evidence rather than being counted as independent successful passes.
3. As a learner, I want retries to return the stored receipt without moving my
   next due date again.
4. As a learner, I want a published family variant to satisfy its original
   card while an unrelated question cannot advance that card.
5. As a learner, I want another account's session to be inaccessible.
6. As a learner, I want unfinished or obsolete evidence to be refused.
7. As an operator, I want existing historical rows preserved without inventing
   a trusted attempt reference for earlier client-reported outcomes.

## Implementation Decisions

- The request identifies the card version, submitted session, item index and
  bounded idempotency key. A client correctness field is never grading authority.
- History gains a nullable attempt reference with a unique constraint. Existing
  rows remain legacy history, with no fabricated evidence linkage.
- Only the caller's submitted session can supply an answer. Question and card
  must be published and belong to the same question family (or exact version).
- Evidence comes from tutor/timed/revision practice, not mutable exam answers.
- Serialize with the existing session-then-user lock order; persist the receipt
  and the card update in one transaction.
- A clean, sure, correct answer is good and advances an interval. A correct
  but uncertain answer is hard and cannot advance passes. Wrong, skipped or
  assisted evidence is again and resets passes. Hard/again remain near due.
- A reused key with different evidence is a conflict. A used attempt under a
  new key is a conflict. A matching retry is idempotent.
- Evidence submitted before the current card update cannot advance that card.
  Use submission time, not answer creation time, because initial enrollment
  happens in the session-submission transaction.

## Testing Decisions

Use the existing authenticated HTTP integration seam and synthetic seeded
questions. First establish a failing regression that rejects a bare correctness
claim. Then exercise real session creation, answering, submission, result
recording, retries, assistance/confidence, variants and cross-account rejection.
Run compilation, migration rollback/reapply and tests in GitHub Actions only.

## Out of Scope

This slice does not complete native retest UI, notification SLA delivery,
clinical calibration or acceptance of real student content. They remain tracked.
