# Enforce current display rights in study queues and derived content

Status: in-progress (QOTD rights first; retests, marks, notes, cards, and insights remain open)

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

## Learner-record behavior

- Re-evaluate current display eligibility at every read, answer, and session-creation boundary that can return question text or derived question content.
- When source rights are unavailable, omit the linked question payload and source-derived material from learner-facing responses and exports. Retain the underlying notes, marks, attempts, schedules, and outcomes without rewriting or deleting them; keep account ownership and non-content learning evidence intact. Eligible content may reappear if the grant becomes active again.

## Delivery order

1. QOTD selection, display, and answer eligibility: issue 05.
2. Retest queues and marks.
3. Notes, exports, review cards, insights, and other source-derived content.

## Spec

See `../spec.md`.

## Comments

- 2026-10-01 — Split QOTD into the focused public-route spec in issue 05. This parent remains open until every acceptance area above is verified.
