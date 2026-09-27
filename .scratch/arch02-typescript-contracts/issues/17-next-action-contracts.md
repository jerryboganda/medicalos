# ARCH-02 — Next-action request and response contracts

Status: in-progress (Rust request/response DTOs, generated declarations, and authenticated wire assertions authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, AI-04, PLAN-01
Source: `.scratch/arch02-typescript-contracts/spec.md`; authenticated next-action route

## Problem Statement

The Today page maintains its own request query and next-action response shapes.
The route returns several related payload variants, and their field presence
can drift from the browser declarations.

## Solution

Generate the query request and the route's response union from Rust DTOs. Keep
the existing route, query parameters, selection rules, and conditional JSON
keys unchanged.

## User Stories

1. As a learner, I want the Today page to read the exact next-action response
   variant so that it presents a valid recommendation or a clear reason none
   is available.
2. As a client maintainer, I want recommendation fields and query options
   generated from the server types so that changes are caught before merge.

## Implementation Decisions

- Model deadline, missing-plan, no-action, and selected-action responses as an
  untagged Rust union matching the current route's JSON shapes.
- Preserve explicit nulls and omitted fields as currently serialized.
- Generate the recommendation, allowance, and query declarations alongside
  the response union.
- Leave recommendation selection logic and UI behavior unchanged.

## Testing Decisions

- Extend the existing authenticated next-action integration flow to assert
  exact top-level keys for missing-plan, no-action, and selected-action
  responses, plus the recommendation's exact fields.
- Use the existing Rust type-export, drift, client build, and browser gates.
  Defer their execution until all implementation slices are authored, as
  directed by the user.

## Out of Scope

- Changing AI-04 scoring, plan state, entitlements, rights filtering, or
  recommendation UX.
- Completing ARCH-02; other client-consumed contracts remain handwritten.
