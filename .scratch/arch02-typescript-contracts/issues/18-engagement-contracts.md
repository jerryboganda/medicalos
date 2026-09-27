# ARCH-02 — Engagement and question-of-the-day contracts

Status: in-progress (Rust request/response DTOs, generated declarations, and authenticated wire assertions authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, ENG-01
Source: `.scratch/arch02-typescript-contracts/spec.md`; authenticated engagement routes

## Problem Statement

The Today page manually declares the engagement status, settings request, and
QOTD answer payloads. The QOTD response also has disabled, unconfigured,
available, and answered wire shapes, including fields that must be omitted or
withheld.

## Solution

Generate the learner-facing engagement request and response types from Rust
DTOs while preserving each existing endpoint's field presence and behavior.

## User Stories

1. As a learner, I want engagement controls and QOTD content to reflect the
   exact server response, so disabled or unavailable states render correctly.
2. As a maintainer, I want optional, nullable, and withheld fields represented
   in generated types, so client code cannot assume data the server omits.

## Implementation Decisions

- Type the engagement status, QOTD state, settings request/response, and answer
  request/response in `routes/engagement.rs` and export them under
  `apps/client/src/lib/generated/engagement/`.
- Preserve the QOTD answer-key boundary: available payloads expose option text
  only; `correct_index` appears only in the answer response.
- Preserve QOTD's conditional keys, the explicit `exam_id: null` when no exam
  is selected, and omission when engagement is disabled.
- Keep eligibility, settings validation, answer locking, scoring, persistence,
  and UI behavior unchanged.

## Testing Decisions

- Extend the authenticated engagement integration flow with exact-key
  assertions for settings, the nested status, each QOTD state, option text,
  and answer results.
- Defer type export, formatting, builds, integration tests, and browser E2E
  execution until implementation slices are complete, as directed by the user.

## Out of Scope

- Changing ENG-01 rules, reminder delivery, scheduling, or the QOTD selection
  algorithm.
- Claiming ENG-01 or ARCH-02 complete from these generated contracts.
