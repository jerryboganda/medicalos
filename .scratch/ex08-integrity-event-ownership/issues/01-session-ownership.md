# EX-08 — Scope integrity events to their session owner

Status: ready-for-agent
Requirement IDs: EX-08, CORE-04
Triage label: ready-for-agent
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §11.3; `.scratch/ex08-integrity-event-ownership/spec.md`

## Acceptance

- An authenticated learner can record an integrity event for a session they own.
- A learner cannot record an event against another learner's session.
- Missing and foreign session IDs return the same non-revealing not-found
  response.
- Account-level events without a session ID retain their current behavior.
- The HTTP integration test passes in GitHub Actions.

## Comments

- 2026-09-26: Found during the production-readiness audit. The handler accepts
  a caller-supplied session ID and inserts it without an ownership check.
- 2026-09-26: The regression test failed on commit `6257ffc` because a foreign
  session returned `recorded: true`. Added an owner-scoped lookup before insert;
  GitHub Actions verification is pending.
