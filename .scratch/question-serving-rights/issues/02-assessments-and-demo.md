# Enforce current display rights in assessments and demo delivery

Status: needs-triage

## Outcome

Assessment and demo routes do not expose published question content after its
current learner-display grant becomes unavailable.

## Acceptance criteria

- Inventory both `/v1` and `/api/v1` route families for exam, mock, QTI/runtime,
  and guest/demo question delivery.
- Apply the shared current display-rights predicate before selecting, starting,
  reading, or returning question content and feedback.
- Existing attempts and aggregate scores remain intact; responses deny or omit
  only content that is no longer displayable.
- Cover active, revoked, expired, wrong-audience, seat-limited, and incomplete
  asset-scope grants with public-route regressions.
- Verify current authoring/publishing behavior still agrees with runtime
  eligibility.

## Spec

See `../spec.md`.

## Comments
