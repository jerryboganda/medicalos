# ARCH-02 issue 27: Coach conversation contracts

Status: implementation authored; generated bindings and CI acceptance pending
Requirement IDs: ARCH-02, AI-06, AI-16
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

The Coach client duplicates request and response shapes for grounded turns,
conversation history, and the list of questions the learner has answered.
Idempotent replay adds a timestamp that fresh turns omit.

## Acceptance

- Rust owns the turn request, turn response, history item/response, and
  answerable-question item/response contracts.
- Fresh and replayed turn JSON keeps the current conditional timestamp and all
  grounding metadata; free-form grounding JSON remains opaque.
- The API client uses the generated contracts for all three calls.
- Authenticated HTTP assertions pin exact keys for fresh/replayed turns,
  history entries, and answerable questions.
- Final type-export, API, and client checks pass in GitHub Actions.

## Seams

- `POST /v1/coach/turns`
- `GET /v1/coach/history`
- `GET /v1/coach/answerable-questions`

## Out of scope

Changing answer-first policy, allowance behavior, grounding, prompt modes, or
idempotency.
