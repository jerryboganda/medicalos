# ARCH-02 issue 30: institutional workspace and analytics contracts

Status: implementation authored; generated bindings and CI acceptance pending
Requirement IDs: ARCH-02, INST-01, INST-02, INST-07
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

Institution membership, cohort, program, curriculum-coverage, audit, and
analytics routes return dynamic JSON. The client duplicates those nested
contracts, including the analytics response's suppressed and unsuppressed
shapes.

## Acceptance

- Rust owns every institutional request and response consumed by `api.ts`.
- Suppressed analytics keeps its reason/minimum fields and omits chapters;
  unsuppressed analytics includes chapters and omits the suppression details.
- Curriculum coverage preserves nullable aggregate fields when the cohort is
  below the disclosure threshold.
- Tenant authorization, ordering, audit values, and aggregate calculations
  stay unchanged.
- Authenticated HTTP checks pin both analytics variants, nullable coverage,
  and representative membership/program response keys.
- Type export, API integration, and client checks pass in GitHub Actions.

## Seams

- `/v1/me/institutions`
- `/v1/institutions` and `/v1/institutions/{institution_id}/members`
- `/v1/institutions/{institution_id}/cohorts`
- `/v1/institutions/{institution_id}/programs` and curriculum/coverage routes
- `/v1/institutions/{institution_id}/analytics` and `/audit`
- `/v1/cohorts/{cohort_id}/assignments`

## Verification

The route DTOs and generated client bindings are authored. Add/finish exact
wire assertions during final verification after implementation slices are
complete; keep the requirement status at in-progress until the generated
export and GitHub Actions gates pass.
