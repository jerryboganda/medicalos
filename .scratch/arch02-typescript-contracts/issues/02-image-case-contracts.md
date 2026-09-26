# ARCH-02 — Generate image-case response contracts

Status: complete
Triage label: ready-for-agent
Requirement IDs: ARCH-02, IMG-01, IMG-02, IMG-04
Source: `.scratch/arch02-typescript-contracts/spec.md`; `.scratch/img04-anatomy-linkage/spec.md`

## Problem Statement

The browser keeps handwritten types for learner image cases and the admin
annotation queue. The API assembles those response payloads as anonymous JSON,
so their field shapes can drift from the client without a compile-time error.

## Solution

Define the existing image-case response shapes as Rust DTOs, generate their
TypeScript declarations with the ARCH-02 pipeline, and have the client import
those declarations. Preserve the current JSON payloads and all rights and
review gates.

## User Stories

1. As a learner, I want image-case summaries and details to match the server's
   serialized response so that image navigation does not fail after an API
   change.
2. As a learner, I want generated contracts to preserve the fact that findings
   and approved annotations appear only after the existing authorization and
   disclosure rules succeed.
3. As an editor, I want the annotation queue, create receipt, and review result
   to use Rust-owned response shapes so that the admin workflow stays typed.
4. As a client developer, I want the image response DTO family generated and
   checked in so that I do not maintain duplicate field declarations.

## Implementation Decisions

- Define serializable DTOs for image-case summaries, details, image references,
  structured findings, approved annotations, pending admin annotations, and
  every response envelope used by the image-case client methods.
- Reuse the existing version-pinned concept-link DTO as the summary's nested
  concept type.
- Preserve current field names, nullability, array ordering, case-kind values,
  and the existing flat detail payload. This includes case-list, concept-map,
  annotation-list, annotation-create, and annotation-review responses.
- Keep list/detail responses behind the active image-display-rights check;
  expose only independently approved learner annotations and only pending
  records in the admin queue.
- Keep authentication, endpoint paths, and transport in the current client API
  module; replace only the handwritten image-case data declarations.
- Export generated declarations to the existing image contract directory and
  use the current tracked-file drift gate. Use a one-time feature-branch
  bootstrap marker only for the push run that produces new declarations; the
  pull-request drift gate stays enforced, and a normal run must pass after the
  generated files are checked in.
- Leave image-case request-body declarations for a separate ARCH-02 slice.

## Acceptance

- The client no longer declares handwritten image-case response interfaces or
  inline response object shapes for its image-case methods.
- Rust response DTOs serialize the same field names, nullability, ordering,
  status values, and flat learner detail shape as before.
- Existing image rights and independent annotation-review gates remain intact.
- API integration and real browser image flows pass, and GitHub Actions exports
  the bindings, detects drift, and builds the client.

## Testing Decisions

- Test serialized behavior through the existing authenticated API integration
  seam for learner list/detail and the admin annotation queue.
- Preserve and extend the real browser image-case flows to assert the visible
  summary, detail, concept, and annotation fields.
- Use the Rust export test, tracked-file drift gate, client build, and full
  Playwright suite in GitHub Actions; do not run heavyweight builds locally.
- Treat the expected field names and values in the current endpoint behavior
  as the independent wire-format contract.

## Out of Scope

- DICOM de-identification, pixel integrity, capture protection, and clinical
  validation.
- Changes to image rights, annotation approval, or concept versioning rules.
- Changes to endpoint paths or learner/admin product behavior.

## Further Notes

IMG-04's `ImageConceptLink` is already generated. This issue migrates the
remaining image-case response DTOs while ARCH-02 stays open for other client
contract families.

## Acceptance evidence

- GitHub Actions push [run 36223707295](https://github.com/jerryboganda/medicalos/actions/runs/36223707295) and PR [run 36223709097](https://github.com/jerryboganda/medicalos/actions/runs/36223709097) passed on `7dffa02d5d8d0eec5b29bf73c9d44f3e96956c84`.
- The suite passed 112 API integration tests, 19 other Rust tests, 23 Rust-owned TypeScript export tests, and 57 Playwright E2E tests. Formatting, Clippy, dependency/license checks, SQLx query-cache generation, WASM, generated-file drift, client, and site gates also passed.
- The follow-up push and PR runs both enforced the checked-in generated-file drift gate. No endpoint, rights, or review behavior changed.
- This closes the image-case response DTO slice only; ARCH-02 remains in progress for other client-consumed API contracts.
