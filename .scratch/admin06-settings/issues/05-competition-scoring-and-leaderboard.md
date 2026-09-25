# ADMIN-06 / COMP-02 — Competition scoring controls and leaderboard

Status: in-progress
Type: task
Requirement IDs: ADMIN-06, COMP-01, COMP-02, COMP-04
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§17.1, 19.5

## Problem statement

The shared scoring engine has the documented 5/10/15 defaults and tie ladder,
but competition submissions always use those defaults and the public
leaderboard currently orders by score and time only. It omits accuracy,
attempt count, and average response time. The original batch submission also
trusted client-reported answer time, which could inflate the speed bonus.

## Solution

Add audited admin controls for the three difficulty point values. A competition
captures those values when it is created so later policy changes cannot alter
an active event. One attempt serves randomized questions and options one at a
time; the server measures response time and derives correctness and difficulty
from stored question versions. The shared scoring crate persists the metrics
needed by the leaderboard. The authenticated leaderboard orders by score,
accuracy, total time, and submission time, includes the learner's row when it
falls outside the top 50, and hides flagged entries.

The documented defaults remain easy/medium/hard = 5/10/15, wrong-answer
penalty = 25%, correct-only speed bonus capped at 20%, and the fixed accuracy,
time, submission tie ladder. Difficulty points are positive, strictly
increasing, and bounded from 1 to 1000.

## User stories

1. As an administrator, I want to adjust difficulty points so that future
   competitions can match the intended scoring policy.
2. As an administrator, I want settings updates validated and audited so that
   the current policy and changes are reviewable.
3. As a learner, I want a competition to retain the scoring policy it opened
   with so that an administrator's later changes do not alter my event.
4. As a learner, I want answer correctness and difficulty taken from the
   stored question version so that the client cannot choose its own score.
5. As a learner, I want the leaderboard to apply the documented accuracy,
   time, and submission tie-breaks so that equal scores are ranked fairly.
6. As a learner, I want accuracy, questions attempted, and average response
   time shown so that the result explains the score.
7. As a learner, I want my own entry present even when it is outside the top
   50 so that I can find my rank.
8. As a learner, I want flagged entries hidden while they await review so
   that unverified results do not appear ranked.

## Implementation decisions

- Reuse the existing admin settings API, `app_settings`, audit event, and
  Editorial Console form.
- Store the three points as one validated JSON array and snapshot it on each
  competition at creation. Existing competitions retain the existing 5/10/15
  values.
- Keep the fixed 25% wrong-answer penalty and 20% correct-only speed-bonus
  ceiling from the shared scoring configuration.
- Start one attempt per opted-in learner and competition. Resume the same
  active attempt after reload; never reset the question timer on resume.
- Serve one randomized question at a time, with independently randomized
  option order. Return option text only: no answer key, rationale, or future
  question reaches the client before the competition closes.
- Accept only the current question and an in-range displayed option. The
  client sends neither elapsed time nor a total time; the server measures
  elapsed time and translates the randomized option back to the stored version.
- Persist each answer and next-question response with an idempotency key so a
  retried request cannot advance the attempt twice.
- Persist correct count, attempted count, and average response time on each
  entry. Backfill existing entries in one replay-safe migration.
- Rank unflagged entries by score descending, exact accuracy descending,
  total time ascending, then submission time ascending; return the top 50 plus
  the authenticated learner's row if it is outside that set.

## Testing decisions

- Authenticated HTTP integration tests cover settings defaults, bounds, audit,
  per-competition policy snapshots, same-exam/published/unique question checks,
  randomized question delivery without answer leakage, current-question and
  option validation, server timing, idempotent retries, one attempt per learner,
  server-scored results, leaderboard ranking and fields, hidden flagged
  entries, accurate prize eligibility, and the pinned learner row.
- Playwright exercises the existing settings controls, accessible labels,
  save, and validation error behavior.
- Migration coverage proves existing entries receive truthful metrics and
  startup replay preserves later submissions.
- Build, formatting, API/browser suites, and migration/query checks passed
  together in GitHub Actions run 36159484978; none are run locally.

## Out of scope

Live WebSocket delivery, anomaly detection, external integrity review
operations, prizes, and league/group filters.
