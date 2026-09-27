# ARCH-02 — Practice session creation request

Status: in-progress (Rust request DTOs, generated declarations, and authenticated wire assertions authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, QB-03, QB-06, QB-07, AI-08, EX-08
Source: `.scratch/arch02-typescript-contracts/spec.md`; `CONTEXT.md`

## Problem

The client duplicates the `POST /v1/practice/sessions` request shape inline,
omits the per-question time budget, and cannot express the server's accepted
explicit-null values consistently. The Rust request DTO is the actual wire
contract, but changes to it do not currently reach the client compiler.

## Solution

Generate the request and nested blueprint-slice types from the existing Rust
deserializers, then use that type in the existing authenticated API method.
Preserve all validation, defaults, and request behavior.

## User stories

1. As a learner, I want single-chapter and multi-chapter practice requests to
   use the server-defined shape, so the selected pool matches what I chose.
2. As a learner, I want untimed per-question budgets and timed-session limits
   represented in the request contract, so the client can send supported timer
   settings.
3. As a client maintainer, I want omitted and explicit-null optional values to
   match Rust deserialization behavior, so optional controls remain compatible.
4. As a maintainer, I want blueprint entries generated with the request, so
   chapter identifiers and counts cannot drift between Rust and TypeScript.

## Acceptance

- Rust-owned generated declarations cover every `CreateSessionReq` field and
  the nested blueprint slice.
- Optional request properties are both omittable and nullable; bounded `i64`
  time values remain TypeScript `number` to match JSON and the existing client.
- The source filter retains its existing four-value TypeScript union.
- `api.ts` consumes and re-exports the generated request and blueprint types,
  with no duplicate inline field list; `per_question_seconds` is available.
- The authenticated create-session flow continues to accept omitted optional
  fields and explicitly accepts `null` for optional fields without changing
  defaults or validation.
- Final GitHub Actions type-export/drift, API, client, and browser checks pass.

## Testing

- Extend the existing authenticated HTTP integration seam for
  `POST /v1/practice/sessions`; assert successful creation and the established
  response behavior when optional values are explicit null.
- Use the existing Rust type-export and generated-file drift gates.
- Defer build and test execution until implementation slices are complete, as
  directed by the user.

## Out of scope

- Changing request validation, defaults, question selection, session response,
  answer behavior, or transport/authentication.
