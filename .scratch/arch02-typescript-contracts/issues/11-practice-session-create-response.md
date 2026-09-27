# ARCH-02 — Practice session creation response

Status: in-progress (Rust response DTOs, generated declarations, and authenticated wire assertions authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, QB-03, QB-05, EX-08
Source: `.scratch/arch02-typescript-contracts/spec.md`; `CONTEXT.md`

## Problem

The create-session endpoint returns a session ID, the initial question items,
and the per-question time budget. The Rust response is assembled as anonymous
JSON, while `api.ts` declares only `{ session_id: string }`. The response type
therefore omits data the server already sends and can drift from the learner
session flow.

## Solution

Define Rust-serialized DTOs for the existing create-session response, its
question items, and text-only options. Generate the TypeScript types from those
DTOs and use them in the existing client method. Preserve the route, payload,
auth behavior, ordering, and answer-key disclosure boundary.

## User stories

1. As a learner, I want the client contract to include the questions returned
   when a session starts, so the initial session state matches the API.
2. As a learner, I want initial options to contain only their display text,
   so rationales and answer keys remain hidden until an answer is recorded.
3. As a client maintainer, I want the response types generated from the Rust
   serializer, so field changes are caught by the existing drift gate.

## Acceptance

- Rust DTOs and generated TypeScript cover `session_id`, `items`,
  `per_question_seconds`, every item field, and the text-only option shape.
- `api.ts` uses and re-exports the generated response types without repeating
  their fields.
- The existing authenticated session-creation integration flow asserts exact
  response and nested key sets, representative values, ordering, and the
  absence of rationale fields before answering.
- The JSON response shape remains compatible; no endpoint behavior or
  selection logic changes.
- Final GitHub Actions type-export/drift, API, client, and browser checks pass.

## Testing

- Extend the existing authenticated HTTP integration flow at
  `POST /v1/practice/sessions`.
- Use the existing Rust type-export and generated-file drift gates.
- Defer builds and test execution until the remaining implementation slices
  are complete, as directed by the user.

## Out of scope

- Changing create-session request validation, question selection, deadlines,
  session detail, answer submission, or learner UI behavior.
