# ARCH-02 issue 23: marks and review contracts

Status: implementation authored; generated bindings and CI acceptance pending
Requirement IDs: ARCH-02
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

Marks and spaced-review endpoints still serialize dynamic JSON while the
browser keeps handwritten request and response shapes. The review queue also
returns card metadata that the browser contract currently omits.

## Acceptance

- Deck and card creation, marked-question listing, review queue, review event,
  and review-debt DTOs are derived from the Rust wire structs.
- The typed responses preserve all existing JSON fields and null behavior,
  including cloze metadata and event timestamps.
- The client uses the generated request and response types for these calls.
- Authenticated HTTP regressions assert exact keys for queue items, marks,
  review outcomes, and honest review-debt null projections.
- The generated TypeScript output is reproducible and the API/client gates pass
  in GitHub Actions.

## Seams

- `POST /v1/decks` and `POST /v1/decks/{deck_id}/cards`
- `GET /v1/me/marks`, `POST|DELETE /v1/questions/{version_id}/mark`
- `GET /v1/reviews/queue`, `POST /v1/reviews/events`,
  `GET /v1/me/review-debt`

## Out of scope

Changing scheduling, card creation, mark privacy, endpoint behavior, or queue
selection rules.
