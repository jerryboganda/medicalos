# Recheck question display rights throughout competition attempts

Status: ready-for-agent

## Problem Statement

Competitions store published question-version IDs and return question text as
learners start or resume an attempt. A grant can change after a competition
was authored, after an attempt began, or between an answer and an
idempotent replay. Publication status alone does not authorize those later
displays.

## Solution

Require current display eligibility whenever competition questions are
selected, presented, answered, or replayed. Refuse to start an attempt when its
configured set is no longer fully displayable; for an in-progress attempt,
withhold the unavailable question and refuse stale answers. Preserve submitted
scores and other content-free competition evidence.

## User Stories

1. As a learner, I want competition setup and recurring question pools to
   contain only currently displayable questions, so that a stored pool cannot
   advertise unavailable content.
2. As a learner, I want an attempt to recheck rights before returning a
   question, so that resuming a stale attempt cannot leak text.
3. As a learner, I want answer submission to recheck current rights, so that
   cached question IDs cannot bypass revocation.
4. As a learner, I want idempotent replays to avoid returning a previously
   cached next-question payload after rights change, so that retries remain
   safe.
5. As a learner, I want my submitted score and leaderboard evidence retained
   without question content, so that rights changes do not rewrite results.

## Implementation Decisions

- Reuse `question_display_rights_active` when creating a competition,
  materializing a recurring occurrence, starting/resuming an attempt, reading
  the current question, and accepting an answer.
- Before creating a new attempt, require every question in the configured
  occurrence to remain published and displayable. Do not silently substitute
  a different question or create a partial snapshot.
- A stored in-progress attempt may remain recorded after rights change, but
  it must not return the unavailable question or accept an answer for it.
- Before replaying a stored response, inspect any question payload it would
  return and recheck that exact version. If it is no longer eligible, return a
  content-free unavailable response instead of the cached question.
- Preserve completed scores, entries, and content-free leaderboard results.
- Keep the current competition UI and response layout when questions remain
  eligible. Unavailable attempts use the existing error/status pattern.

## Testing Decisions

- Exercise competition creation, recurring materialization, entry start/resume,
  question answer, and idempotent replay through their learner/admin API
  routes.
- Start from an active display grant, then test revoked, expired,
  wrong-audience, unsupported-seat-limit, and incomplete-asset-scope changes.
- Assert no new attempt is created from an ineligible question set; an
  in-progress attempt cannot return or accept the affected question; cached
  response replay has no question payload; stored scores and entries remain.
- Restore eligibility and verify still-pending eligible attempts can return
  their unchanged question selection.

## Out of Scope

- QOTD (issue 05), other saved review and learner records (issue 06), offline
  device behavior (issue 04), external license authenticity, institutional
  competition acceptance, and deployment.
- Substituting or reordering the competition's configured question set after
  rights change.

## Further Notes

Competition attempts persist randomized question order, option order, and
answer receipts. The rights check must guard both freshly constructed
responses and previously stored response payloads.
