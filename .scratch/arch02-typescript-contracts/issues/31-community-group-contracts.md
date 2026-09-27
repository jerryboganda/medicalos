# ARCH-02 issue 31: opt-in community group and moderation contracts

Status: implementation authored; generated bindings and CI acceptance pending
Requirement IDs: ARCH-02, COMMUNITY-01, COMMUNITY-03
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

Community profile, group, post, and moderation endpoints return dynamic JSON,
while the browser repeats those response shapes inline. The profile endpoint
also has two wire shapes: opted-out responses omit the handle, while opted-in
responses include it.

## Acceptance

- Rust owns the profile, group, post, report, and moderation request/response
  DTOs consumed by the browser.
- The opted-out response continues omitting `handle`; report notes and queue
  fields preserve their existing null behavior.
- Member/moderator authorization, anonymized author handles, ordering,
  tombstone behavior, and report resolution remain unchanged.
- Authenticated HTTP assertions cover both profile states, member-visible
  posts, report/queue fields, and moderator resolution.
- Type export, API integration, and client checks pass in GitHub Actions.

## Seams

- `/v1/community/profile`, `/v1/community/me`, and profile-by-handle
- `/v1/community/groups` and its member/post/report routes
- `/v1/community/me/reports`

## Verification

The route DTOs and generated client bindings are authored. Add/finish exact
wire assertions during final verification after implementation slices are
complete; keep acceptance pending until generated export and GitHub Actions
gates pass.
