# 39 — Administrator workflow, import, and audit contracts

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, ADMIN-04, ADMIN-06

## Scope

Move question-import requests and results, import rollback, assessment
workflow outcomes, and administrator audit events to Rust-owned DTOs. Preserve
the existing validation, duty-separation rules, audit records, and exact JSON
payloads while using the generated contracts in the client.

## Verification

Generated-file drift, exact import/workflow response assertions, and the
administrator console browser flow are deferred to final ARCH-02 CI, as
requested. No local builds or tests run before implementation slices are
complete.
