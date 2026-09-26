# ARCH-02 — Rust-owned TypeScript contracts

Status: in-progress
Requirement IDs: ARCH-02
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §29; `AGENT_IMPLEMENTATION_HANDOFF.md`

## Problem Statement

The browser app manually mirrors API request and response shapes in
`apps/client/src/lib/api.ts`. Rust handlers serialize the real wire format, so
the TypeScript copy can drift without a compiler error.

## Solution

Generate client data declarations from the Rust DTOs that define the server's
wire format. Keep transport concerns such as authentication, HTTP methods, and
route paths in the existing client module. Migrate DTO families incrementally,
and keep ARCH-02 open until every client-consumed contract is generated.

## User Stories

1. As a learner, I want the Today response consumed by the browser to match the
   serialized Rust response, so changes to the server shape fail in CI instead
   of appearing as client runtime defects.
2. As a client developer, I want request and response types generated from the
   Rust types, so I do not maintain duplicate structural declarations.
3. As a maintainer, I want generated files checked into the client workspace
   and reproducible in GitHub Actions, so client builds do not need a Rust
   runtime and stale output is detected before merge.

## Implementation Decisions

- Use `ts-rs` derives on the existing serialized Rust DTOs; do not create a
  second schema or change endpoint behavior.
- Store generated declarations under the client library's generated folder.
- Keep the hand-written HTTP transport in `api.ts`; it may import and re-export
  generated types but may not restate their fields.
- The initial vertical slices cover `TodayResponse` and the IMG-04
  `ImageConceptLink` DTO. Other client-consumed contracts remain hand-written
  and keep ARCH-02 in progress.
- The next vertical slice covers the learner image-case and admin annotation
  response DTO family; it preserves existing endpoint payloads and rights gates.
- Run the generator in the Rust GitHub Actions job. Once the bootstrap files
  are checked in, CI fails on tracked or untracked generated-file drift. Client
  and browser builds consume the same generated artifact.
- When a later DTO family adds new files, a marked `codex/*` push may produce
  the artifact while the pull-request drift gate remains active. The generated
  files must be checked in and pass a normal push and PR run before acceptance.
- Keep numeric wire values as TypeScript `number` when the JSON API emits
  numeric values and the domain bounds keep them within JavaScript's safe
  integer range.

## Testing Decisions

- Test the existing authenticated HTTP DTO seam, not the generated macro's
  internal implementation.
- Use ts-rs export tests to regenerate checked-in bindings; a scoped Git diff
  check must fail if committed declarations are stale or newly generated
  files are missing.
- Build the client in GitHub Actions to prove the actual UI consumes the
  generated types.
- Extend this same drift gate as remaining endpoint DTO families migrate.

## Out of Scope

- Replacing the HTTP client, authentication, route construction, or fetch
  behavior.
- Claiming ARCH-02 complete from the Today slice alone.
- Generating types for endpoints the client does not consume.

## Further Notes

The generator choice and package facts are recorded in
`research.md`. See issue `issues/01-today-contracts.md` for the active slice.
