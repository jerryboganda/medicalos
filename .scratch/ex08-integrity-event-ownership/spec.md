# EX-08 integrity-event ownership

## Problem Statement

The authenticated integrity-event endpoint stores the submitted `session_id`
without checking that the session belongs to the authenticated learner. A
learner can therefore attach integrity evidence to another learner's session.

## Solution

When an event names a session, accept it only if that session belongs to the
authenticated learner. Return the same not-found response for a missing session
and a session owned by someone else so the endpoint does not reveal whether a
foreign session ID exists. Preserve account-level events that omit `session_id`.

## User Stories

1. As a learner, I can record an integrity signal for my own session so the
   session has accurate evidence.
2. As a learner, I cannot attach evidence to another learner's session by
   submitting its identifier.
3. As a learner, I receive the same response for an unknown session and a
   session I do not own, so I cannot use this endpoint to enumerate sessions.
4. As a learner, I can continue to record an account-level signal that does
   not reference a session.

## Implementation Decisions

- Keep the existing authenticated Axum endpoint and integrity-event table.
- Check session ownership before inserting an event; use the API's standard
  non-revealing not-found error for both missing and foreign sessions.
- Keep the current signal vocabulary and optional session behavior unchanged.
- Do not add a dependency, schema migration, or UI surface for this fix.

## Testing Decisions

- Test at the existing HTTP integration seam with the real router and ephemeral
  PostgreSQL fixture.
- Prove that the owner can record an event and that another learner receives
  the non-revealing not-found response for the same session ID.
- Prove that an account-level event without a session remains accepted.
- Run all builds and tests in GitHub Actions under the project compute policy.

## Out of Scope

- Per-test integrity policies, away-time grace windows, client signal capture,
  and platform-specific attestation. These remain separate EX-08 work.
- Human review, automated punishment, or disclosure of integrity evidence.

## Further Notes

This closes the session-ownership gap only; it does not complete EX-08.
