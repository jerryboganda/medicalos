# SIM-06 — Counterfactual replay and debrief timeline

Status: in-progress
Requirement IDs: SIM-06, SIM-01, TRUST-01
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §14

## Problem Statement

Completed simulation runs preserve an ordered transcript, but learners and reviewers cannot explore a different sequence of station actions without risking changes to the original run.

## Solution

Expose an indexed timeline derived from the immutable run transcript and a read-only counterfactual replay endpoint. Replay uses the exact scenario version pinned to the run, validates every proposed action through that version's deterministic transition machine, and returns the alternate timeline without persisting or changing the original transcript, state, or rubric evidence.

## User Stories

1. As a learner, I want a numbered timeline so that feedback can refer to the exact observed action.
2. As a learner, I want to try an alternate action sequence so that I can compare a different path through the same station.
3. As an examiner, I want replay to use the run's frozen scenario version so that later authoring changes cannot alter historical debriefs.
4. As an operator, I want invalid event sequences refused with a useful error and no partial result.
5. As a learner, I want replay to leave my completed run and examiner evidence untouched.

## Implementation Decisions

- Reuse the authenticated debrief endpoint as the highest-level read seam and add a read-only replay endpoint for completed, owner-scoped runs.
- Reuse the deterministic transition resolver from the scenario engine.
- Limit a replay to 100 bounded event names; stop at a terminal state and refuse further actions.
- Return the actual indexed timeline, alternate indexed timeline, final state, terminal flag, and pinned scenario version.
- Do not store replay attempts or use an AI-generated patient response in this deterministic slice.

## Testing Decisions

- Add HTTP integration cases for successful alternate replay, invalid transitions, terminal-state handling, version pinning, and proof that the durable run transcript is unchanged.
- Keep builds and tests deferred to the final GitHub Actions pass.

## Out of Scope

- A patient-voice model, clinical validation of alternative paths, or assessment changes based on replay.
- Persisting counterfactual branches as clinical evidence.

## Implementation Record

- Added a published-station list, learner-owned run view, indexed debrief timeline, and a read-only alternate-path form. Replay is pinned to the run version and leaves the original evidence unchanged. API and browser regression cases are authored; clinical path validation and current CI remain pending. No tests or builds have been run.
