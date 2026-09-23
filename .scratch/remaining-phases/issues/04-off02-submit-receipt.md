# OFF-02 — Idempotent session submission receipt

Status: ready-for-human
Requirement IDs: OFF-02, QB-13, EX-08
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§8.3, 11.6, 11.8
Implementation state: complete; final API CI acceptance is deferred.

## Problem

If the server submits a session but its response is lost, the learner's queued
retry currently receives `session_closed`. The session is complete on the
server, but the client cannot recover the score and summary.

## Acceptance

- Submission stores the exact result receipt with the session.
- Repeating submission returns the original receipt, including after a lost
  response, without repeating XP, learner-state, streak, community, or plan
  completion side effects.
- Concurrent submission attempts produce the same receipt and apply completion
  side effects once.
- Previously submitted sessions without a stored receipt can still reconstruct
  a receipt from persisted attempts and session data.
- Existing response fields and ownership checks remain unchanged.

## Verification boundary

Integration coverage is authored for concurrent replay and stored result
equality. Do not run local builds or tests; final GitHub Actions is the
acceptance gate.
