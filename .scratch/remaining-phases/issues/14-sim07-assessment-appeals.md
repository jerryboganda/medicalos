# SIM-07 — Independent review of consequential simulation assessments

Status: ready-for-human
Requirement IDs: SIM-07, SIM-04, TRUST-01
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §14

## Problem Statement

An assessed simulation run has immutable criterion evidence, but learners have no appeal path and administrators have no independent review workflow. Rewriting the original result would erase what the first examiner recorded.

## Solution

Allow the run owner to submit one reasoned appeal after examiner results exist. An administrator other than the learner and every original examiner may record one immutable decision: confirm the assessment or require a new independent assessment. Preserve the original evidence and show the appeal state alongside the debrief.

## User Stories

1. As a learner, I want to appeal a recorded assessment so that a human can review a consequential judgment.
2. As a learner, I want to see whether my appeal is open, confirmed, or requires reassessment.
3. As an administrator, I want a bounded queue containing the appeal reason and run evidence so that I can review the case.
4. As an administrator, I want the original learner and every original examiner excluded from the appeal decision so that review is independent.
5. As an operator, I want appeals and decisions immutable and audited without copying the appeal narrative into the general audit log.
6. As a learner, I want an upheld appeal to flag reassessment as required without silently changing the original score.

## Implementation Decisions

- Add separate append-only appeal and review rows for scenario assessments.
- Scope learner submission to their own finished run with a recorded examiner assessment; allow one appeal per run.
- Exclude the appellant and all original assessors from resolving the appeal.
- Use only `confirmed` and `reassessment_required` decisions. The latter keeps the original score visible and flags it as unsuitable for consequential use until a new independent assessment exists.
- Expose the queue and details through the existing admin console; surface appeal state on the learner debrief.
- Audit IDs and decision codes, not appeal reasons or reviewer rationale.

## Testing Decisions

- API integration cases cover ownership, finished/assessed preconditions, duplicate submission, separate reviewer identity, queue lifecycle, appeal-state disclosure, and database immutability.
- Playwright covers learner submission and independent admin resolution.
- Build and test execution remain deferred to the final GitHub Actions pass.

## Out of Scope

- Automatically changing the original assessment, issuing accreditation, or declaring a case suitable for consequential use without a new independent assessment.
- Clinical-policy approval of the rubric or appeal criteria.

## Implementation Record

- Added learner-owned appeal submission for a finished run with immutable examiner evidence, one appeal per run, and printable 10–2000 character reasons.
- Added a bounded admin queue and evidence detail view. Appeal decisions require a different admin identity from the learner and every original examiner.
- Added append-only appeal/review tables and database mutation guards. Audit entries include IDs and decision codes but omit appeal and rationale text.
- The learner debrief exposes the appeal state and `consequential_use_status`; reassessment-required preserves the original score and explicitly blocks consequential use pending a new independent assessment.
- Added API integration coverage for unfinished/unassessed runs, ownership, duplicate appeals, appellant/examiner exclusion, queue lifecycle, unchanged original scores, audit privacy, and row immutability; added learner and admin Playwright flows.
- No builds or tests have been run. Clinical policy approval and current GitHub Actions remain separate acceptance gates.
