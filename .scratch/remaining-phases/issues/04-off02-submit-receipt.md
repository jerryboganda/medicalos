# OFF-02 — Idempotent session submission receipt

Status: ready-for-human
Requirement IDs: OFF-02, QB-13, EX-08
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§8.3, 11.6, 11.8
Implementation state: API CI acceptance passed in run 36159484978.

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

Integration coverage for concurrent replay and stored result equality passed
in GitHub Actions run
[36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978).
