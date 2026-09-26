# ARCH-02 — Notification inbox contracts and quiet-hours validation

Status: in-progress (Rust request/response DTOs, regression assertions, and validation fix authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, CORE-08
Source: `.scratch/arch02-typescript-contracts/spec.md`; authenticated notification routes; master plan §6.4

## Problem Statement

The client inlines notification preferences and omits `deep_link` and
`created_at` from inbox entries even though the server returns them. Also, an
out-of-range `quiet_hours_end` is accepted when `quiet_hours_start` is omitted.

## Solution

Generate the notification inbox and preference request/response types from
Rust DTOs, and validate either quiet-hours endpoint independently.

## User Stories

1. As a learner, I want complete typed inbox items and preferences so I can
   open the right deep link and see the current notification state.
2. As a learner, I want quiet-hours values constrained to the supported day
   range regardless of which endpoint I update.

## Implementation Decisions

- Preserve default preferences, notification ordering, row limits, and
  notification behavior.
- Validate each supplied quiet-hours value within 0-23 before writing.
- Include nullable `deep_link` and timestamp fields from the existing query in
  the generated inbox item.

## Testing Decisions

- Extend the authenticated inbox flow with exact-key checks and a stored item
  that has a null deep link.
- Assert invalid start-only and end-only quiet-hours updates are rejected.
- Defer type export, formatting, builds, integration tests, and browser E2E
  execution until implementation slices are complete, as directed by the user.

## Out of Scope

- Adding remote push delivery, campaign scheduling, or new notification types.
- Completing ARCH-02; other client-consumed DTO families remain handwritten.
