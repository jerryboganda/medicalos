# 40 — Pack lease revocation contract

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, PROT-02

## Scope

Give pack lease revocation a Rust-owned response DTO and consume its generated
contract in the offline client without changing the user-scoped delete or
not-found behavior.

## Verification

Generated-file drift, exact HTTP response checks, and the offline browser flow
are deferred to final ARCH-02 CI, as requested. No local builds or tests run
before implementation slices are complete.
