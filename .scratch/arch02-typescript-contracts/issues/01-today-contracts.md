# ARCH-02 — Generate the Today response contract

Status: complete
Triage label: complete
Requirement IDs: ARCH-02
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Acceptance

- Rust `TodayResponse` and its serialized dependencies generate the TypeScript
  declarations used by the client.
- `apps/client/src/lib/api.ts` contains no handwritten field definitions for
  these generated response shapes.
- GitHub Actions regenerates the declarations and fails on stale, missing, or
  additional generated files after the initial artifact bootstrap.
- The client build consumes the checked-in declarations.

## Scope boundary

This closes only the Today DTO family. ARCH-02 remains in progress while other
client-consumed API request and response models remain handwritten.

## Implementation Record

- Complete. Rust-generated Today declarations are checked into the client
  workspace, consumed by the client build, and protected by the tracked-file
  drift gate in GitHub Actions run 36221287354.

## Follow-up slice

IMG-04 adds the generated `ImageConceptLink` response DTO in
`.scratch/img04-anatomy-linkage/spec.md`; the broader ARCH-02 requirement stays
open until the remaining client-consumed contracts migrate.
