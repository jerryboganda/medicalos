# 36 — Administrator settings contracts

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, ADMIN-06, OPS-06

## Scope

Generate the runtime-settings update request, current settings view, and
updated-key response from the Rust settings routes. Keep current validation,
authorization, defaults, and JSON key names unchanged; consume generated DTOs
from the administrator client.

## Verification

Exact HTTP key assertions, ts-rs drift checks, and the administrator browser
flow are deferred to the final CI pass, as requested. No local builds or tests
run before implementation slices are complete.
