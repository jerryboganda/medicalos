# ARCH-02 — Portfolio and continuing-education contracts

Status: in-progress (Rust request/response DTOs and authenticated wire assertions authored; final type-export and GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, CAREER-01, CAREER-03
Source: `.scratch/arch02-typescript-contracts/spec.md`; learner portfolio and CE routes

## Problem Statement

The portfolio API accepts an untyped record from the client and returns
`unknown[]` for saved entries. Continuing-education records also use inline
request and response shapes.

## Solution

Generate portfolio and CE request/list/response declarations from Rust DTOs,
including dates, nullable fields, and the allowed portfolio kinds.

## User Stories

1. As a learner, I want saved portfolio entries to have their actual fields so
   I can use them consistently across the product.
2. As a learner, I want CE activity to remain clearly labeled as recorded, not
   accredited, in its typed response.

## Implementation Decisions

- Keep current validation, user scoping, stored fields, sort order, and honest
  non-accreditation note unchanged.
- Replace unknown and inline client declarations with generated Rust DTOs.
- Represent `occurred_on` as an optional nullable date string.

## Testing Decisions

- Extend the existing authenticated portfolio/CE integration flow with exact
  request and response field assertions, including the non-accreditation note.
- Defer type export, formatting, builds, integration tests, and browser E2E
  execution until implementation slices are complete, as directed by the user.

## Out of Scope

- Adding accreditation claims or provider certification workflows.
- Completing ARCH-02; other client-consumed DTO families remain handwritten.
