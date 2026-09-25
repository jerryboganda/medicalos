# ADMIN-06 — Live spaced-review caps

Status: in-progress
Type: task
Requirement IDs: ADMIN-06, SR-02, PLAN-03
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§13, 19.5

## Problem

The review queue uses fixed daily limits. Operators cannot tune due-review and
new-card caps through the existing audited settings surface.

## Scope

Expose `max_reviews_per_day` and `max_new_cards_per_day` through the current
admin settings API and Editorial Console. Keep the documented defaults of 30
due reviews and 10 new cards. Updates take effect on the next queue request;
existing review history and scheduling state are unchanged.

## Acceptance

- Admin GET returns effective values, using 30 and 10 when no override exists.
- Admin PATCH validates each value in its supported range, rejects an invalid
  batch atomically, and records old and new effective values in the existing
  audit event.
- The review queue reads current values and today's consumed allowance at
  request time, returns only the remaining daily capacity, and reports due
  cards left in `backlog_remaining`.
- Daily allowances reset on the UTC calendar day, matching persisted review
  timestamps and avoiding dependence on the database session timezone.
- Review-event writes enforce the same daily caps atomically. Replaying an
  accepted idempotency key remains successful and consumes no additional
  capacity.
- Review history records whether each event introduced a new card or reviewed
  an existing one, so the two daily caps remain distinct across restarts.
- The existing console loads, edits, saves, and displays errors for both
  controls with accessible labels and the current design tokens.
- API integration coverage verifies defaults, bounds, audit values, and live
  queue effects, including UTC-day boundaries under a non-UTC database
  timezone plus concurrent migration replay. Playwright verifies settings
  roundtrip and error states.
- Settings remain in `app_settings`; one additive migration records the review
  event category and indexes the daily usage lookup.
- The replayed startup migration serializes its one-time backfill under
  concurrent API startup.

## Verification boundary

Use authenticated API and existing admin-page browser seams. Keep formatting,
build, tests, and migration rollback verification in the final GitHub Actions
batch; do not run local builds or test suites.

## Out of scope

Per-learner overrides, changing card due dates, and changing the scheduler's
ordering policy.
