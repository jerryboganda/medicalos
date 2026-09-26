# ARCH-02 — Scenario request contracts

Status: implementation authored; Rust-owned regeneration and GitHub Actions acceptance pending
Requirement IDs: ARCH-02, SIM-01, SIM-06, SIM-08
Source: `.scratch/arch02-typescript-contracts/spec.md`; `.scratch/remaining-phases/issues/15-sim08-team-handover.md`

## Problem

Several scenario mutations still send inline or handwritten JSON request
shapes from the client. Their Rust `Deserialize` DTOs are not exported, so
changes to the server request format can drift without a TypeScript error.

## Scope

Export the existing start, team invite, invite join, handover, transition, and
counterfactual replay request DTOs from Rust. Make the API facade use those
generated types. Preserve endpoint behavior, validation, authorization, and
the current wire fields.

## Acceptance

- Rust owns generated request declarations for all six client-consumed
  request DTOs.
- Client calls use the generated declarations for their JSON bodies and
  handover parameter.
- No route or runtime behavior changes.
- Final GitHub Actions type export, generated-file drift, API, client, and
  browser jobs pass.

## Testing seam

Use the existing Rust `type-export` tests and strict generated-file drift gate;
the client build checks each request body against its generated type. Run the
project's API and browser suites at the final validation pass, as directed by
the user.
