# ARCH-02 — Administrator scenario assessment contracts

Status: in-progress (Rust DTOs, generated declarations, and authenticated wire assertions authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, SIM-04
Source: .scratch/arch02-typescript-contracts/spec.md; .scratch/arch02-typescript-contracts/issues/08-scenario-appeal-contracts.md; MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md §14

## Problem

The administrator assessment queue, assessment detail, submission request, and receipt are still handwritten or anonymous JSON contracts. They can drift between the Rust API and the TypeScript client.

## Solution

Generate Rust-owned TypeScript contracts for the existing administrator assessment endpoints and use them in the client without changing authorization, validation, persistence, or response JSON.

## Acceptance

- Rust DTOs and generated TypeScript cover the pending assessment item and wrapper, assessment detail, submission status enum, criterion and wrapper request, and assessment receipt.
- The generated contracts preserve the current nullable score, defaultable transcript evidence fields, raw transcript as unknown[], and exact response field names.
- Omitted defaultable evidence fields remain accepted; explicitly null evidence fields remain invalid.
- api.ts uses generated contracts and has no duplicate field declarations for these request/response payloads.
- The authenticated SIM-04 integration flow asserts exact response key counts and representative wire values, including no learner identifier in examiner detail.
- Final GitHub Actions API, type-export/drift, client, and browser jobs pass.

## Testing

- Extend the existing authenticated HTTP integration flow at the admin assessment endpoints.
- Use the existing Rust type-export and generated-file drift gates.
- Defer test and build execution until the remaining implementation slices are complete, as directed by the user.

## Out of scope

- Assessment eligibility, score validation, examiner independence, evidence rules, persistence, or route behavior.
- Clinical policy or changes to the learner debrief and appeal workflows.
