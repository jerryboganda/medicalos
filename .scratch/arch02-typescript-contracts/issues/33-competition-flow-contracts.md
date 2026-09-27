# 33 — Competition flow contracts

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, COMP-01, COMP-02, COMP-04

## Scope

Generate Rust-owned client contracts for competition listings, start and
answer requests, in-progress and submitted attempt steps, displayed questions,
and leaderboard entries. Keep the current response fields and preserve the
idempotent answer replay payload when decoding stored responses.

## Verification

HTTP key and nullability assertions, type-export drift, and the competition
browser flow are deferred to the final ARCH-02 CI verification pass, as
requested. No local build or test suite is run before implementation slices
are complete.
