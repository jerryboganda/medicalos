# Enforce current display rights in assessments and demo delivery

Status: complete (push CI run 36856293152 on c9d6a3a; PR #32 checks passed)

## Outcome

Assessment, mock, guest, and QTI package paths do not expose or export question
content unless the current grant covers the applicable use.

## Acceptance criteria

- Inventory fixed-assessment, mock, and QTI export paths under both `/v1` and
  `/api/v1`, plus the shared `/guest/trial` paths (which have no `/api/v1` alias).
- Refuse to launch a fixed assessment if any reserved question has lost current
  learner-display rights; filter mock selection to currently displayable items.
- Filter guest question selection to currently displayable items and stop
  returning a previously sampled question once its grant expires or is revoked.
- Treat admin QTI export as redistribution: require a separate explicit
  `distribution` use grant with active dates, learner or unrestricted audience,
  complete asset scope, and no unsupported seat limit. A learner `display`
  grant alone must never authorize exporting a package with answer keys.
- Keep existing session read and answer checks on the shared practice pipeline.
- Existing attempts and aggregate scores remain intact; responses deny or omit
  only content that is no longer displayable.
- Cover active, revoked, expired, wrong-audience, seat-limited, and incomplete
  asset-scope grants with public-route regressions; cover both API aliases and
  QTI's separate distribution permission.
- Verify current authoring/publishing behavior still agrees with runtime
  eligibility.

## Spec

See `../spec.md`.

## Comments

- 2026-10-01 — Implemented and accepted on exact source commit `c9d6a3a` in [GitHub Actions run 36856293152](https://github.com/jerryboganda/medicalos/actions/runs/36856293152). Rust, offline SQLx compile, client, E2E, site, and Zitadel jobs passed; the PR reported 12 passing checks. Assessment, mock, guest, and both QTI API aliases are covered, with QTI requiring active distribution rights for every published question. The fixture now aligns the reused question's source reference with the scoped grant. License authenticity and consuming-LMS certification remain outside this code acceptance.
