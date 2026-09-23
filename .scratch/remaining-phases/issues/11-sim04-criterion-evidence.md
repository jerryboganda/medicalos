# SIM-04 — Criterion-level simulation evidence

Status: ready-for-human
Requirement IDs: SIM-04, SIM-02, SIM-05, SIM-07
Source: MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md §§14.1–14.3
Implementation state: version-pinned rubric, deterministic station completion, pending-assessment queue, browser examiner workflow, and learner debrief evidence are implemented; provider voice, clinical validation, and human appeal operations remain separate gates.

## Problem Statement

Scenario rubric tables exist, but there is no API to define criteria or record examiner evidence. Learner debriefs contain only the transcript and state; they cannot show criterion-level evidence or distinguish an unobserved skill from a failed skill.

## Solution

Pin each authored rubric criterion to a scenario version. Mark configured terminal states so a run can finish. An admin examiner records exactly one append-only result per criterion after completion. Assessed results require a score within the criterion maximum and one or more indexes into the immutable run transcript. Not-assessed results require a reason, have no score, and cite no transcript event. The learner debrief returns every criterion and uses not_assessed when no examiner result exists.

## User Stories

1. As an editor, I want to attach rubric criteria to a scenario version so that later revisions do not change prior runs.
2. As an examiner, I want to record evidence and a score per criterion so that feedback is traceable to the learner's observed actions.
3. As an examiner, I want to mark a skill not assessed with a reason so that absent evidence never becomes an invented failure or pass.
4. As a learner, I want my debrief to show each criterion, its evidence, and its assessment state.
5. As an operator, I want assessment writes limited to admin examiners and audited without copying private transcript content into the audit stream.

## Implementation Decisions

- Reuse scenario_rubrics and scenario_rubric_evidence; add a scenario-version key, event-index list, reviewer, and explicit assessment state.
- Preserve existing transcript event order and use zero-based event indexes as the evidence link.
- Keep examiner writes insert-only and unique per run/criterion. The run must be finished; scenario machines with rubrics declare terminal states.
- Keep AI out of authoring and scoring. Only an authenticated admin examiner can write results.
- Return absent assessment rows as explicit not_assessed with a null score and no evidence.
- Provide an admin queue for finished unassessed stations and a browser examiner form that cites transcript events or records an explicit not-assessed reason.
- Regression seams: authenticated HTTP API and the existing admin browser route. Integration cases cover rubric creation/version pinning, terminal completion, evidence bounds, score bounds, explicit not-assessed, queue lifecycle, immutable one-time assessment, self-assessment rejection, audit privacy, and owner isolation. Playwright covers the examiner's transcript citation flow. Test/build execution remains deferred to the final GitHub Actions pass.

## Out of Scope

- Patient voice, speech recognition, clinical validation, human appeal operations, and institutional consequential-use approval.
- Automatic or model-generated scores, transcript rewriting, and physical-skill claims unsupported by observed evidence.

## Implementation Record

- Added version-pinned rubric criteria, successor version creation, configured terminal-state completion, a pending-run queue, admin examiner GET/POST routes, insert-only criterion results, owner-visible debrief evidence, and a browser examiner flow with transcript event selection. API and browser regression cases are authored; no tests or builds have been run.
