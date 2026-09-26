# ARCH-02 — Practice answer response

Status: in-progress (Rust response DTOs and authenticated wire assertions authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, QB-03, QB-04, QB-05, QB-17, OFF-02
Source: `.scratch/arch02-typescript-contracts/spec.md`; `CONTEXT.md`

## Problem

The client has a handwritten `AnswerResult` that describes tutor feedback but
not the recorded-only response returned by timed and mock answers. The Rust
handler also assembles anonymous JSON, so the current types can miss wire-shape
changes.

## Solution

Represent the existing feedback and recorded-only payloads as Rust response
DTOs, generate their TypeScript union, and return those DTOs through the
existing authenticated answer endpoint. Keep the JSON untagged so no new
discriminator is added to the wire.

## User stories

1. As a learner in tutor or revision mode, I want answer feedback to include
   the correct choice, rationales, learning point, exam tip, and available
   tutoring cards, so I can review my response.
2. As a learner in timed or mock mode, I want answers acknowledged without
   early feedback, so assessment integrity is preserved.
3. As a learner retrying a request after a lost response, I want the replay to
   retain the same response shape and mark it as already recorded.
4. As a mock candidate changing an answer before submission, I want the receipt
   to report an actual answer change while omitting that field otherwise.
5. As a client maintainer, I want both response shapes generated from Rust, so
   the UI cannot assume tutor feedback exists for every preset.

## Acceptance

- Rust response DTOs serialize the existing feedback and recorded-only
  payloads without adding wire fields or changing answer logic.
- The generated TypeScript response is a union of the feedback and recorded
  receipt shapes; tutoring cards and answer-change fields are optional exactly
  when omitted by the server.
- `api.ts` consumes and re-exports the generated types and removes the
  handwritten `AnswerResult` declaration.
- Authenticated API assertions verify exact keys and seeded feedback values,
  option rationales, replay behavior, and mock recorded/changed receipt shapes.
- Final GitHub Actions type-export/drift, API, client, and browser checks pass.

## Testing

- Extend the existing authenticated tutor answer/replay flow and mock answer
  change flow at `POST /v1/practice/sessions/{sid}/answers`.
- Use seeded question data as the expected feedback source and assert exact
  object keys so no feedback or receipt fields leak into another preset.
- Defer builds and tests until implementation slices are complete, as directed
  by the user.

## Out of scope

- Changing answer validation, scoring, idempotency, answer-change policy,
  tutoring generation, offline sync, or session detail responses.
