# ARCH-02 — Scenario team roster response contract

Status: ready-for-agent
Requirement IDs: ARCH-02, SIM-08
Source: `.scratch/arch02-typescript-contracts/spec.md`; `.scratch/remaining-phases/issues/15-sim08-team-handover.md`
Implementation state: in progress; final CI acceptance is deferred until the remaining implementation slices are complete.

## Problem Statement

The scenario team roster is returned as anonymous JSON and mirrored by
handwritten TypeScript interfaces, so its role and member fields can drift.

## Solution

Define serializable Rust response DTOs for the existing team role, roster, and
member data. Generate the client contracts from those types without changing
membership, authorization, or privacy behavior.

## Implementation Decisions

- Migrate only `GET /v1/scenarios/runs/{run_id}/team` and the role/member/roster
  types it returns.
- Preserve the four database-constrained role values, UUID string encoding,
  and timestamp string encoding.
- Keep the current response privacy boundary: expose member IDs and roles but
  never expose user IDs.
- Keep request bodies and invite/handover receipts for later contract slices.
- Use the existing authenticated HTTP integration test and ARCH-02 generator
  and drift pipeline.

## Acceptance

- Rust DTOs serialize the exact existing roster fields and member shape.
- The client imports generated roster/member/role declarations instead of
  restating those fields.
- Existing scenario-team API integration coverage asserts the roster shape
  and the absence of user IDs.
- Final GitHub Actions generated-contract, API, client, and browser jobs pass.

## Testing Decisions

- Extend the existing authenticated SIM-08 HTTP integration flow.
- Use the existing type-export job, generated-file drift check, and full client
  and browser jobs at the final validation gate.
- Defer builds and test execution until the remaining implementation slices
  are complete, as directed by the user.

## Out of Scope

- Invitation, handover, acknowledgement, run, debrief, and assessment DTOs.
- Any role, membership, access-control, or persistence behavior changes.
