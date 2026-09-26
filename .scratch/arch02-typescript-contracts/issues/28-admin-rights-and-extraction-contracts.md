# ARCH-02 issue 28: admin rights and extraction-report contracts

Status: implementation authored; generated bindings and CI acceptance pending
Requirement IDs: ARCH-02, ADMIN-02, LIB-07
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

The admin content-rights and document-extraction routes build dynamic JSON,
while the browser maintains handwritten request and response shapes. This
allows nullability, optional metadata, and status values to drift.

## Acceptance

- Rust owns content-rights create/list/revoke request and response contracts.
- Rust owns extraction-report create/review requests and list/detail response
  contracts, including review state and derived report status.
- The client uses generated contracts for every covered route and keeps the
  existing wire fields, null values, sorting, and status vocabulary.
- Authenticated HTTP assertions pin exact keys, nullability, and the two
  extraction-report response variants.
- Type export, API integration, and client checks pass in GitHub Actions.

## Seams

- `POST|GET /v1/admin/content-rights`
- `PATCH /v1/admin/content-rights/{rights_id}/revoke`
- `GET /v1/admin/library/extraction-reports`
- `GET|POST /v1/admin/library/extraction-reports/{report_id}`
- `POST /v1/admin/library/extraction-reports/{report_id}/review`

## Verification

The type conversions and client migration are authored. Add/finish exact wire
assertions during the final verification pass after the implementation slices
are complete; do not mark this issue accepted until the generated export and
GitHub Actions gates pass.
