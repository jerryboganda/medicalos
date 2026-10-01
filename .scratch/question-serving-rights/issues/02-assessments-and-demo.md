# Enforce current display rights in assessments and demo delivery

Status: ready-for-agent

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
