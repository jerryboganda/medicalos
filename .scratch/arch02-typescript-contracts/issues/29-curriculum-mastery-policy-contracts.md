# ARCH-02 issue 29: curriculum, mastery, and selection-policy contracts

Status: implementation authored; generated bindings and CI acceptance pending
Requirement IDs: ARCH-02, QB-12, PROG-01, AI-17
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

The learner curriculum, mastery heatmap, and selection-policy responses are
hand-built JSON while the client repeats their nested structures inline.
Optional mastery overlays also have three distinct wire states: absent,
present with null accuracy, and present with a numeric accuracy.

## Acceptance

- Rust owns the curriculum, heatmap query/response, and selection-policy DTOs.
- The generated heatmap chapter type preserves optional overlay omission and
  explicit null accuracy.
- The existing ordering, values, evidence rules, query validation, and
  entitlement behavior remain unchanged.
- Authenticated HTTP assertions pin exact base/overlay keys and values.
- Rust type export, API integration, and client checks pass in GitHub Actions.

## Seams

- `GET /v1/me/curriculum`
- `GET /v1/me/heatmap`
- `GET /v1/me/selection-policy`

## Verification

The route DTOs and client bindings are authored. Add/finish exact wire
assertions during the final verification pass after implementation slices are
complete; do not mark this issue accepted until generated export and GitHub
Actions gates pass.
