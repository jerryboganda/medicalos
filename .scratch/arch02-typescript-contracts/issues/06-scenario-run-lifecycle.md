# ARCH-02 — Scenario run lifecycle response contracts

Status: ready-for-agent
Requirement IDs: ARCH-02, SIM-01, SIM-03, SIM-05, SIM-06
Source: `.scratch/arch02-typescript-contracts/spec.md`; `.scratch/remaining-phases/issues/13-sim06-counterfactual-timeline.md`
Implementation state: implementation and both static review axes complete; final CI acceptance is deferred until the remaining implementation slices are complete.

## Problem Statement

Scenario run and transition responses are anonymous JSON. The handwritten
client also treats the raw stored transcript as an indexed timeline, although
only the separate `timeline` response has indexes and uncertainty fields.

## Solution

Define Rust-owned response DTOs for the run view, indexed timeline events, and
scenario event receipt. Generate the browser declarations from those DTOs and
represent the raw transcript entries as opaque values until a client needs a
more specific contract.

## Implementation Decisions

- Migrate `GET /v1/scenarios/runs/{run_id}` and
  `POST /v1/scenarios/runs/{run_id}/events` responses.
- Preserve raw transcript JSON at runtime; type its client representation as
  `unknown[]`. The indexed timeline remains the stable, structured view.
- Preserve every current timeline field, including nulls, uncertainty state,
  event ordering, and numeric indexes. Do not include fields the endpoint does
  not serialize.
- Keep all scenario state transitions, persistence, and access checks intact.
- Use the existing authenticated API integration seam and ARCH-02 drift gate.

## Acceptance

- Rust DTO serialization matches the current run and event response JSON.
- The client imports generated run, event-receipt, and timeline-event types;
  it no longer claims raw transcript rows are indexed timeline records.
- API integration coverage asserts the run and event wire shapes, including
  nullable timeline fields.
- Final GitHub Actions generated-contract, API, client, and browser jobs pass.

## Testing Decisions

- Extend the existing authenticated SIM-04/SIM-06 scenario lifecycle flow.
- Use the Rust type-export job, generated-file drift check, client build, and
  browser suite at the final validation gate.
- Defer build and test execution until the remaining implementation slices are
  complete, as directed by the user.

## Out of Scope

- Scenario debrief, counterfactual replay, rubric, appeal, team, and request
  body contracts.
- Any change to the scenario state machine, transcript persistence, or UI.
