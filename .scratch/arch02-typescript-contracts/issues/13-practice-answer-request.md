# ARCH-02 — Practice answer request

Status: in-progress (Rust request DTO, generated declaration, and authenticated wire assertion authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, QB-03, QB-04, QB-17, OFF-02
Source: `.scratch/arch02-typescript-contracts/spec.md`; `CONTEXT.md`

## Problem

The client maintains the practice answer request inline and omits fields
accepted by Rust, including confidence, declared assistance, elapsed time, and
offline-recorded time. This can drift from the request the server validates.

## Solution

Generate the existing Rust answer deserializer as the client request type and
use it in the current API method. Preserve the route, answer validation,
idempotency, mock answer changes, offline sync, and first-answer-wins behavior.

## User stories

1. As a learner, I want answers to carry the server-supported confidence and
   assistance fields, so my evidence is classified consistently.
2. As a learner, I want elapsed and offline-recorded time represented in the
   request contract, so the client can submit timing evidence without changing
   server authority.
3. As a client maintainer, I want the answer request generated from Rust, so
   optional and nullable fields stay compatible with deserialization.
4. As a maintainer, I want the supported confidence values constrained in the
   generated type, so invalid values are caught before sending.

## Acceptance

- Generated declarations cover every `AnswerReq` field, preserving required
  `item_index` and idempotency key, optional nullable `chosen_index` and
  metadata, the confidence union, JSON-number elapsed time, and timestamp-
  string offline time.
- `api.ts` imports/re-exports the generated type and removes its duplicate
  request-field list.
- The existing authenticated answer flow exercises confidence, assistance,
  elapsed time, and an explicit-null recorded timestamp while retaining replay
  and first-answer-wins assertions.
- Final GitHub Actions type-export/drift, API, client, and browser checks pass.

## Testing

- Extend the existing authenticated `POST /v1/practice/sessions/{sid}/answers`
  seam; assert its response and idempotency behavior remain intact.
- Use the existing Rust type-export and generated-file drift gates.
- Defer builds and tests until implementation slices are complete, as directed
  by the user.

## Out of scope

- Changing answer validation, scoring, timing trust, database storage,
  idempotency, replay payloads, or session detail responses.
