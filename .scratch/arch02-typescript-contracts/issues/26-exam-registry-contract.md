# ARCH-02 issue 26: exam registry contract

Status: implementation authored; generated bindings and CI acceptance pending
Requirement IDs: ARCH-02, EX-01
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

The browser narrows the exam registry response to three fields while the Rust
endpoint returns official-source metadata and aliases as part of each record.

## Acceptance

- Rust owns the exam registry item and list response types.
- The generated types include the complete current wire response, preserving
  nullable source URLs and the JSON-valued aliases field without narrowing its
  stored data contract.
- The API client uses the generated response type.
- Authenticated HTTP assertions pin the exact envelope and item keys.
- Final type-export, API, and client checks pass in GitHub Actions.

## Seams

- `GET /v1/exams`

## Out of scope

Changing exam registry data, alias validation, or exam-spec administration.
