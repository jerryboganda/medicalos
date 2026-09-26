# ARCH-02 — Plan protection and capacity replan contracts

Status: in-progress (Rust request/response DTOs and authenticated wire assertions authored; final type-export and GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, PLAN-01, PLAN-02, AI-08
Source: `.scratch/arch02-typescript-contracts/spec.md`; authenticated plan routes

## Problem Statement

The client handwrites task-protection and capacity-replan request and response
shapes. Replanning returns distinct no-op and applied payloads whose fields can
drift independently.

## Solution

Generate protection and replan contracts from the Rust DTOs, including the
existing two-shape replan response union.

## User Stories

1. As a learner, I want plan actions to consume the actual server result so
   that a no-op is not mistaken for a new plan version.
2. As a maintainer, I want request fields and both response variants generated
   from Rust so wire changes fail before merge.

## Implementation Decisions

- Use a serde-untagged Rust union matching the existing `replanned` boolean
  payloads, preserving field names, nullability, and route behavior.
- Keep expected-version checks, capacity selection, protection rules, and
  revision receipts unchanged.
- Replace the inline client request and response types with generated types.

## Testing Decisions

- Extend authenticated plan integration flows with exact-key assertions for
  task protection, within-capacity no-op, and applied replan payloads.
- Defer type export, formatting, builds, integration tests, and browser E2E
  execution until implementation slices are complete, as directed by the user.

## Out of Scope

- Changing replanning policy or UI behavior.
- Completing ARCH-02; other client-consumed DTO families remain handwritten.
