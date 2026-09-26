# 38 — Administrator hierarchy and concept contracts

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, ADMIN-06, NOTE-02

## Scope

Move the admin hierarchy list/create and concept list/create/version/mapping
request and response shapes to Rust-owned DTOs. Keep existing authorization,
validation, audit behavior, status codes, and wire payloads. Consume the
generated declarations in the API client and admin console.

## Verification

Generated-file drift, exact HTTP payloads, and the admin-console browser flow
are deferred to final ARCH-02 CI, as requested. No local builds or tests run
before implementation slices are complete.
