# ARCH-02 — Scenario discovery and start response contracts

Status: in-progress
Triage label: ready-for-agent
Requirement IDs: ARCH-02, SIM-01, SIM-02, SIM-05
Source: `.scratch/arch02-typescript-contracts/spec.md`; `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §14

## Problem Statement

The scenario list and start endpoints return stable response shapes, but their
client types are handwritten in TypeScript and can drift from the Rust API.

## Solution

Define Rust-owned serializable DTOs for the published scenario list, scenario
summary, and scenario start receipt. Generate the client declarations from
those DTOs and make the existing API methods consume them without changing
routes, permissions, or response JSON.

## Implementation Decisions

- Migrate only `GET /v1/scenarios` and `POST /v1/scenarios/runs` response DTOs
  in this slice; keep request types and later run/debrief/team responses for
  separate slices.
- Preserve `slug`, `title`, `version` in each summary and the existing seven
  scenario-start fields, including UUID string encoding and action ordering.
- Keep the authenticated HTTP integration seam and existing ts-rs generator,
  generated-file drift gate, and client build.
- Do not change scenario publication, run creation, transition, or access rules.

## Acceptance

- Rust response DTOs serialize the exact existing list and start response
  shapes.
- The client imports generated declarations and contains no duplicate summary
  fields or inline start response object.
- Existing authenticated scenario integration coverage asserts the list and
  start wire shape.
- Final GitHub Actions generated-contract, API, client, and browser jobs pass.

## Testing Decisions

- Extend the existing authenticated scenario API integration test with
  independent expected field/value assertions.
- Use the existing type-export job and generated-file drift check to prove the
  client declarations come from Rust and are current.
- Defer all builds and test execution to the final project validation pass, as
  directed by the user.

## Out of Scope

- Scenario run, timeline, transcript, debrief, team, appeal, and assessment
  response contracts.
- Any endpoint behavior, permission, persistence, or UI changes.
