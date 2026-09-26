# 41 — Image case administration request contracts

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, IMG-01, IMG-02, IMG-04

## Scope

Generate request contracts for image case concept mappings, annotation
creation, and independent annotation review. Keep current authorization,
validation, audit behavior, and response payloads intact while using the
generated requests at the API client boundary.

## Verification

Generated-file drift, exact HTTP request/response checks, and image case
admin-browser flows are deferred to final ARCH-02 CI, as requested. No local
builds or tests run before implementation slices are complete.
