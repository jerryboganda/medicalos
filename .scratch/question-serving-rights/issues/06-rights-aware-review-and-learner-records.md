# Withhold source-linked review content when question rights change

Status: complete (full exact-source PR run 36908529573 on b6c464adcc90f29f06bd4f03a76b43bde7d458b9 passed)

## Problem Statement

Due retests, marks, source-linked notes, and review cards can outlive the
question's current learner-display grant. Their learner-facing payloads may
continue returning question text or content copied from that question after
revocation, expiry, audience restriction, seat limits, or a source-scope gap.

## Solution

Recheck current display eligibility in every learner-facing read that returns
question content or source-linked material. Keep each learner's stored marks,
retest schedule, note, and card unchanged; hide the associated payload while
its source question is unavailable so that it may return when eligibility is
restored.

## User Stories

1. As a learner, I want a due retest to use only a currently displayable
   question or family variant, so that a saved schedule cannot bypass current
   rights.
2. As a learner, I want marks and linked review materials hidden while their
   question is ineligible, so that copied question content does not outlive
   its display grant.
3. As a learner, I want private source-linked notes omitted from note lists,
   concept views, and exports while their source is ineligible, so that
   question-derived material is not redistributed after revocation.
4. As a learner, I want my underlying notes, marks, review state, and retest
   schedule preserved, so that restored rights make my existing study state
   available again without reconstruction.
5. As a learner, I want unrelated private notes and cards to remain available,
   so that rights changes affect only content linked to the affected question.

## Implementation Decisions

- Use `question_display_rights_active` on the exact linked question version at
  the point each response is built.
- Due retests may use only a currently displayable base version or an
  unattempted, currently displayable family variant. If neither is eligible,
  omit the queue item and preserve its saved schedule.
- Omit an ineligible marked question from the marks response while retaining
  its database row.
- Omit a source-linked note and its backlink title from note-list,
  concept-note responses while the source is ineligible. Note and account exports omit the
  entire ineligible linked note. Unlinked notes continue to work normally.
- Omit a source-linked review card from the review queue and deck export while
  its question is ineligible. Cards without question provenance are unaffected.
- Preserve attempts, answer receipts, notes, marks, retest cards, review
  schedules, account ownership, and aggregate insights. No rights check should
  rewrite or delete learner-owned evidence.
- Keep the existing response design and UI. This slice changes which rows are
  eligible for display, not page structure or visual treatment.

## Testing Decisions

- Exercise `/v1/me/retests`, `/v1/me/marks`, `/v1/notes`, concept notes,
  `/v1/notes/export`, `/v1/reviews/queue`, and `/v1/me/decks/export` through
  authenticated learner routes; exercise `/api/v1` aliases where registered.
- Verify eligible linked material is visible; then cover revoked, expired,
  wrong-audience, unsupported-seat-limit, and incomplete-asset-scope states.
  Assert linked text is omitted while unrelated user content remains.
- Restore eligibility and verify the same stored note, mark, schedule, and
  card reappear. Assert their underlying rows were retained throughout.
- Keep query-only counts and other non-content aggregates available where
  they do not expose question text or question-derived material.

## Out of Scope

- QOTD (issue 05), competition pools and attempts (issue 07), offline device
  behavior (issue 04), external license authenticity, LMS certification, and
  deployment.
- Changing the user's underlying note, mark, retest schedule, or review card,
  or replacing its current UI presentation.

## Further Notes

The record remains owned by its learner even when its linked source question
cannot currently be displayed. Source eligibility controls the learner-facing
content projection; it does not erase or transfer the private record.
