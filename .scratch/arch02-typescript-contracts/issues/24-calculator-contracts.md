# ARCH-02 issue 24: learner study-tool contracts

Status: implementation authored; generated bindings and CI acceptance pending
Requirement IDs: ARCH-02, QB-13
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

The authenticated calculator and session-hint endpoints use handwritten
TypeScript request and response shapes while Rust accepts several optional
formula inputs and emits stable calculation, conversion, and assisted-hint
responses.

## Acceptance

- Calculator, unit-conversion, and session-hint request/response DTOs are
  exported from the Rust wire types.
- The API client consumes and re-exports those generated types.
- Existing behavior stays the same, including nullable/omitted inputs, unit
  names, and the exam-practice-only disclaimer.
- Authenticated integration assertions pin the exact response keys for
  calculation, conversion, and hint responses.
- Generated bindings are reproducible and API/client gates pass in GitHub
  Actions.

## Seams

- `POST /v1/calculators/{kind}`
- `POST /v1/calculators/convert`
- `GET /v1/practice/sessions/{session_id}/items/{item_index}/hint`

## Out of scope

Changing formulas, supported calculator names, validation, or clinical use.
