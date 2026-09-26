# ARCH-02 — Scenario collaboration response contracts

Status: ready-for-agent
Requirement IDs: ARCH-02, SIM-08
Source: `.scratch/arch02-typescript-contracts/spec.md`; `.scratch/remaining-phases/issues/15-sim08-team-handover.md`
Implementation state: in progress; final CI acceptance is deferred until the remaining implementation slices are complete.

## Problem Statement

Scenario collaboration endpoints return invite, membership, handover, and
acknowledgement receipts as anonymous JSON while the browser keeps parallel
TypeScript response shapes.

## Solution

Define Rust-owned response DTOs for the existing scenario team collaboration
endpoints and generate their client contracts. Preserve the current request,
authorization, audit, and persistence behavior.

## Implementation Decisions

- Migrate responses for team invite creation and acceptance, handover listing
  and creation, and handover acknowledgement.
- Reuse the generated scenario-team role enum; preserve exact field names,
  response status codes, UUID/date encodings, and the literal acknowledgement
  value.
- Keep request bodies and assessment/debrief/replay contracts for separate
  slices.
- Keep the authenticated SIM-08 API integration seam and the existing ts-rs
  generation and drift pipeline.

## Acceptance

- Rust DTOs serialize the exact existing response fields, values, and optional
  acknowledgement timestamp.
- Client collaboration methods use generated response declarations and the
  generated `ScenarioHandover` data type.
- The existing authenticated SIM-08 integration flow asserts these wire
  shapes.
- Final GitHub Actions generated-contract, API, client, and browser jobs pass.

## Testing Decisions

- Extend the existing SIM-08 HTTP integration flow for invite, join, handover,
  and acknowledgement receipts.
- Use type export, generated-file drift, client, and browser jobs at the final
  validation gate.
- Defer builds and tests until the remaining implementation slices are
  complete, as directed by the user.

## Out of Scope

- Request-body generation, changes to membership or handover behavior, and
  clinical validation of scenario content.
