# Enforce current display rights in study queues and derived content

Status: needs-triage

## Outcome

Recommendations, review queues, saved question-linked resources, and derived
learning material do not continue to display a question's content after its
current learner-display grant becomes unavailable.

## Acceptance criteria

- Inventory daily plans, QOTD, engagement, retests, marks, notes, insights,
  tutoring cards, and other learner-facing question-linked paths.
- Reuse the shared current display-rights predicate for question discovery and
  any response that returns licensed question text or a derivative of it.
- Define and test how saved learner notes/cards behave after revocation while
  preserving non-content learning evidence and account isolation.
- Cover active, revoked, expired, wrong-audience, seat-limited, and incomplete
  asset-scope grants with public-route regressions.
- Verify no queue can create a new session from ineligible content.

## Spec

See `../spec.md`.

## Comments
