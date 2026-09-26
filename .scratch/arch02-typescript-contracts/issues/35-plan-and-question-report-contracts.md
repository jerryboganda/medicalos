# 35 — Plan undo and question-report contracts

Status: implementation authored; final ARCH-02 CI acceptance pending
Requirement IDs: ARCH-02, AI-07, QB-08, QB-16

## Scope

Generate the existing plan-undo receipt and question-report request, response,
resolution, and administrator-queue DTOs from Rust. Preserve the current
HTTP fields and nullable values. Represent administrator reporter feedback as
typed rows so nullable notes are not narrowed incorrectly in the client.

## Verification

Exact HTTP payload assertions, type-export drift, and relevant API/browser
flows are deferred to the final verification pass, as requested. No local
build or test suite runs before the implementation slices are complete.
