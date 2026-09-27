# ARCH-02 — Practice session detail response

Status: in-progress (Rust response DTOs and authenticated wire assertions authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, QB-03, QB-05, QB-08, QB-13, EX-08
Source: `.scratch/arch02-typescript-contracts/spec.md`; `CONTEXT.md`

## Problem

The practice page's handwritten `PracticeSession` and `SessionItem` types omit
server fields and duplicate the session-detail wire format. The endpoint also
conditionally discloses answer rationales, answer keys, and tutoring cards, so
its response contract must preserve those exact boundaries.

## Solution

Represent the existing session-detail payload, item, answer-option, report
receipt, and report-status shapes as Rust response DTOs; generate their
TypeScript declarations; and have the client consume the generated session
response. Preserve the wire keys while enforcing the established feedback
release policy for each session mode.

## User stories

1. As a learner, I want the session view to show server-issued session timing,
   status, source, and mock metadata so it can recover the correct workspace.
2. As an exam candidate, I want correctness and rationales withheld until my
   answer is recorded, so the session response cannot reveal unreleased keys.
3. As a learner who reported a question, I want my acknowledgement and
   resolution receipt represented accurately in the session response.
4. As a client maintainer, I want session detail types generated from Rust so
   optional and nullable fields cannot silently drift.

## Acceptance

- Typed Rust DTOs serialize the same session and item fields, nulls, and
  conditional tutoring-card field as the existing handler.
- Unanswered items expose only option text and null answer-key fields. Tutor
  feedback is released after each answer; mock/timed feedback stays hidden from
  session detail until the session is submitted.
- Generated TypeScript declares session metadata, per-item fields, optional
  rationales, report status, the nullable report receipt, and optional tutoring
  cards.
- `api.ts` consumes and re-exports generated types and removes handwritten
  `PracticeSession` and `SessionItem` declarations.
- Authenticated integration assertions verify exact response keys and seeded
  values before and after an answer, including the answer-key disclosure rule.
- Final GitHub Actions type-export/drift, API, client, and browser checks pass.

## Testing

- Extend the authenticated session-detail flow in
  `full_loop_cold_start_answer_submit_revision_undo`.
- Assert exact session and item keys before answering, and compare unreleased
  answer fields/options against null/text-only expectations.
- Fetch detail again after answering and compare answer feedback and option
  rationales to the seeded question fixture.
- In the mock lifecycle, fetch detail after an answer and assert that correctness,
  keys, explanations, and rationales stay hidden until submit; then verify their
  release after submit.
- Use the existing API integration seam; defer builds and test execution until
  the implementation slices are authored, as directed by the user.

## Out of scope

- Changing session ownership, timing, report resolution, tutoring generation,
  session recovery, or client behavior.
- Generating session create, answer, or submission contracts already covered by
  issues 11–14.
