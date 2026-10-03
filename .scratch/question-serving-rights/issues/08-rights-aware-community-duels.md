# Recheck question rights and accept community duels atomically

Status: in-progress

## Problem Statement

Community duel acceptance currently chooses published questions without
checking current learner-display rights. It can create both learner sessions
from a stale pool, accept a duel with fewer questions than requested, and
leave duplicate or partial sessions if concurrent accepts or a database error
race.

## Solution

Create duel sessions only when the full configured question count remains
published and currently displayable. Lock the pending duel and persist both
sessions plus the active-state transition in one transaction so one acceptance
creates one complete pair.

## User Stories

1. As a learner accepting a duel, I want every question in both sessions to
   have current display rights, so that revoked or out-of-scope content never
   enters a new study session.
2. As a learner, I want an acceptance to fail cleanly when fewer questions
   remain eligible than the duel requests, so that I do not receive a partial
   contest.
3. As either participant, I want both session records and the active duel
   state to be committed together, so that a failed write cannot leave one
   participant with a session the other does not have.
4. As the challenged learner, I want concurrent accept requests to create at
   most one pair of sessions, so that retries cannot duplicate or fork a duel.
5. As a learner, I want an active grant restored before acceptance to make the
   pending duel usable again, without changing the existing community UI.

## Implementation Decisions

- Apply the shared question display-rights predicate inside duel question
  selection while preserving exam, chapter, quarantine, and reserved-form
  boundaries.
- Require each selected pool to match the duel's requested question count
  before inserting any session.
- Lock the duel row and keep question selection, both session writes, duel
  session links, and the pending-to-active transition in one transaction.
- Preserve the current API response shape and use the existing conflict and
  empty-pool error pattern; no UI or schema change is needed.

## Testing Decisions

- Test through public duel creation and acceptance routes using synthetic
  rights grants and questions.
- Revoke one of three grants for a three-question duel and assert acceptance
  refuses without creating either learner session; restore it and assert both
  sessions contain the requested count.
- Force the second participant link to collide with an existing duel slot and
  assert a failed acceptance leaves no new session, item, or participant link.
- Hold the duel row lock while two acceptance requests wait, then release it
  and assert exactly one acceptance succeeds and exactly two sessions exist.
- Reuse the route-level test and lock-wait patterns already used for question
  publication and competing editorial decisions.

## Out of Scope

- Duel scoring, participant visibility, share links, prizes, and UI behavior.
- Choosing replacement questions after a duel session has already started.
- External clinical validation or genuine content-license verification.

## Further Notes

This closes a remaining learner-facing session-creation path in the question
rights inventory. The normal practice-session read and answer paths still
recheck current rights after a session has been created.
