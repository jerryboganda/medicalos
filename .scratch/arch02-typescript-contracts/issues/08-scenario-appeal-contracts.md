# ARCH-02 — Generate scenario appeal request and response contracts

Status: ready-for-agent
Requirement IDs: ARCH-02, SIM-07
Source: `.scratch/arch02-typescript-contracts/spec.md`; `.scratch/remaining-phases/issues/14-sim07-assessment-appeals.md`
Implementation state: implementation and both static review axes complete; final CI acceptance is deferred until the remaining implementation slices are complete.

## Problem Statement

Scenario appeal submission and independent review responses are anonymous JSON.
The browser keeps separate handwritten queue, detail, and request shapes, so
Rust changes can drift from the learner and admin clients.

## Solution

Generate Rust-owned TypeScript contracts for appeal creation, admin queue and
detail responses, and independent review requests and responses. Preserve the
existing serialized payloads and review policy.

## Implementation Decisions

- Use `ts-rs` on the serialized Rust request and response DTOs.
- Preserve existing routes, status codes, validation errors, authorization,
  review independence, and audit behavior.
- Reuse generated `ScenarioAppealDecision`, `ScenarioAppealStatus`,
  `ScenarioTimelineEvent`, and `ScenarioRubricResult` types where applicable.
- Keep queue rows and appeal detail fields at their existing wire nesting and
  nullability; do not expose private reviewer data.
- Keep HTTP methods, paths, authentication, and transport in `api.ts`; remove
  only duplicated contract declarations from that module.

## Acceptance

- Appeal creation, queue, detail, and review DTOs serialize to the exact
  existing response shapes.
- Request contracts cover appeal reason and reviewer decision/rationale.
- The client consumes generated contracts without restating response fields.
- Existing authenticated integration coverage asserts representative exact
  keys and response object sizes.
- Final GitHub Actions API, generated-contract drift, client, and browser jobs
  pass.

## Testing Decisions

- Extend the existing SIM-07 integration flow for exact response shape checks.
- Add every exported DTO to the existing Rust TypeScript export gate.
- Defer test and build execution until all implementation slices are complete,
  as directed by the user.

## Out of Scope

- Changes to assessment, appeal eligibility, reviewer selection, appeal
  decisions, clinical policy, or consequential-use authorization.
- Admin assessment queue and examiner submission contracts.
