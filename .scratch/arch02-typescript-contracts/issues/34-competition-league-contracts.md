# 34 — Competition league contracts

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, COMP-03, COMMUNITY-03

## Scope

Replace the handwritten client league state and leave response with Rust-owned
DTOs. Preserve the discriminated `joined: false | true` wire union and the
same-week cohort standings returned by both the state and join endpoints.

## Verification

Exact response-key assertions, ts-rs drift checks, and the community/practice
browser flow are deferred to the final CI pass, as requested. No local builds
or tests run before implementation slices are complete.
