# ARCH-02 — Scenario debrief and replay response contracts

Status: ready-for-agent
Requirement IDs: ARCH-02, SIM-06
Source: `.scratch/arch02-typescript-contracts/spec.md`; `.scratch/remaining-phases/issues/13-sim06-counterfactual-timeline.md`
Implementation state: implementation and both static review axes complete; final CI acceptance is deferred until the remaining implementation slices are complete.

## Problem Statement

The learner debrief and counterfactual replay responses are anonymous JSON.
The client currently models the raw transcript as indexed timeline events,
omits transcript corrections, and reuses one timeline type for two distinct
wire shapes.

## Solution

Add Rust-owned response DTOs for the learner debrief, rubric results, appeal
summary, transcript corrections, and counterfactual replay. Generate the
client declarations from the serialized Rust types without changing endpoint
behavior.

## Implementation Decisions

- Preserve the exact payloads from `GET /v1/scenarios/runs/{run_id}/debrief`
  and `POST /v1/scenarios/runs/{run_id}/counterfactual`.
- Keep the raw stored transcript opaque as `unknown[]`; use structured types
  only for the separately generated timeline arrays.
- Give counterfactual events their actual wire shape (`index`, `sequence`,
  `from`, `on`, `to`, and `terminal`) instead of reusing the learner timeline
  event shape.
- Preserve nullable assessment evidence, timestamps, and appeal fields.
- Keep rubric, appeal, and consequential-use statuses as closed literal unions
  in the generated TypeScript contract.
- Do not change access control, state transitions, rubric calculations,
  transcript correction behavior, or appeal semantics.

## Acceptance

- Rust DTO serialization matches both existing endpoint payloads, including
  transcript corrections and nullable rubric/appeal fields.
- Rust enum exports preserve the existing finite status unions in TypeScript.
- The client imports the generated debrief and replay response types and no
  longer duplicates their response fields in `api.ts`.
- Existing authenticated integration coverage asserts the key counts and
  distinguishing fields for both wire shapes.
- Final GitHub Actions generated-contract, API, client, and browser jobs pass.

## Testing Decisions

- Extend the existing SIM-06 debrief and counterfactual integration flow.
- Use the Rust type-export job, generated-file drift check, client build, and
  browser suite at the final validation gate.
- Defer build and test execution until all remaining implementation slices are
  complete, as directed by the user.

## Out of Scope

- Admin assessment queue, assessment submission, appeal queue/detail/review,
  and request-body contracts.
- Any change to clinical validation or consequential-use policy.
