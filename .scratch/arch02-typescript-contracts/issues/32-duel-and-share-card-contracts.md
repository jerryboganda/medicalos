# 32 — Duel and share-card contracts

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, COMMUNITY-02, GROW-01

## Scope

Move the client-consumed duel creation, acceptance, state, decline, and list
payloads plus share-card payloads to Rust-owned `ts-rs` DTOs. Keep the existing
endpoints and serialized fields, including nullable winners and duel-side
scores. Add a client transport method for resolving a duel share token and use
generated DTOs on the community page instead of a local shape and cast.

## Verification

Exact-key and nullability assertions for the affected HTTP responses are
deferred to the final ARCH-02 verification pass, as requested by the user.
Generation drift, API integration, and the community browser flow also await
that final CI run. No local build or test suite is run before the remaining
implementation slices are complete.
