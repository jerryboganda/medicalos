# ARCH-02 — EX-07 mock and timed-submit contracts

Status: in-progress (Rust DTOs and authenticated wire assertions authored; final type-export, API, client, and browser CI acceptance pending)
Requirement IDs: ARCH-02, EX-07, QB-17
Source: .scratch/arch02-typescript-contracts/spec.md; .scratch/ex07-mock-types/issues/01-mock-types-and-time-analysis.md

## Problem

The browser still hand-maintains the mock configuration/list/start payloads and
the practice submit receipt, including mock pacing and per-item timing. These
contracts can drift from the JSON assembled by the Rust routes.

## Solution

Make Rust DTOs the source for the client types for mock create/list/start and
practice submit. Keep route behavior, validation, stored receipt replay, and
the existing HTTP transport unchanged.

## Acceptance

- Generated Rust-owned contracts cover mock test type and integrity policy,
  blueprint entries, create request/response, list item/response, start
  response, and every field in the submit/time/mock/breakdown response.
- The existing authenticated integration flow asserts exact key sets and
  representative values for create, list, start, submit timing, and mock
  breakdown responses.
- `api.ts` consumes generated types and contains no duplicate field
  declarations for these payloads; its receipt type accounts for historical
  stored receipts without weakening the current Rust-generated DTO.
- Stored `result_payload` receipts from prior submissions continue to be
  returned as their original JSON without normalization.
- Existing invalid `mock_type` inputs continue to return 422
  `invalid_mock_type`; request nullability and defaults remain compatible.
- Final GitHub Actions type-export/drift, API, client, and browser jobs pass.

## Verification

- The API integration test is authored but intentionally not run yet, per the
  instruction to finish implementation slices before builds, tests, and CI.
- Static diff checks and code review are performed after implementation.

## Out of scope

- Changing mock validation, database constraints, scoring, timing calculations,
  admin UI, or learner UI behavior.
