# 37 — Rights-checked private import contracts

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, LIB-06

## Scope

Move private-import rights, create, list, detail, and delete payloads to
Rust-owned DTOs. Preserve rights availability checks, ownership scoping,
media-type validation, response fields, and the existing 201 create status.
Use generated contracts in the library client.

## Verification

Exact HTTP payload checks, generated-file drift, and library browser flow are
deferred to final ARCH-02 CI, as requested. No local builds or tests run before
implementation slices are complete.
