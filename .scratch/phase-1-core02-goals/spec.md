# CORE-02 — Versioned learner goals, exam dates, and protected commitments

## Problem Statement

Learners need a reliable way to declare the time they can realistically study, their current exam date (or no exam date while exploring), and commitments that planning must treat as protected. These constraints are foundational to feasible planning, but they do not yet exist as a versioned learner-owned record.

The system must preserve learner agency: changes need to be explicit, versioned, conflict-safe, and reversible. Automatic planning may read these constraints, but it must not silently rewrite the learner's exam date or protected commitments.

## Solution

Add a learner-owned, versioned goals snapshot exposed through the authenticated API and surfaced as a compact editor on Today.

The snapshot contains the learner's daily available study minutes (which is also the daily goal), an optional exam date, and protected dated commitments. Updates use optimistic version checks, create a new version only when the content actually changes, and can be undone by restoring the previous snapshot as a new version.

The automatic planner remains read-only with respect to this record. This slice introduces no automatic mutation path for exam dates or protected commitments.

## User Stories

1. As a learner, I want to see whether I have configured a study goal so that the product does not pretend to know my available time.
2. As a learner, I want to set my realistic daily study minutes so that future plans can stay within my approved capacity.
3. As a learner, I want my declared available time to serve as my daily study goal so that onboarding does not ask the same question twice.
4. As a learner, I want to set an exam date so that planning can respect the deadline I actually chose.
5. As a learner in exploratory mode, I want to leave the exam date unset so that the product does not invent one.
6. As a learner, I want to record protected dated commitments so that planning can treat them as hard constraints rather than movable suggestions.
7. As a learner, I want protected commitments to have explicit titles and dates so that I can understand exactly what is protected.
8. As a learner, I want invalid or impossible inputs rejected clearly so that corrupt planning constraints are not stored.
9. As a learner, I want an edit to create a new version only when something meaningfully changed so that version history remains useful.
10. As a learner, I want stale edits rejected instead of overwriting a newer edit so that concurrent devices cannot silently lose my changes.
11. As a learner, I want to undo my most recent goal change so that availability and exam-date edits are reversible.
12. As a learner, I want undo to create a new version rather than deleting history so that the record remains auditable.
13. As a learner, I want another learner's constraints to remain inaccessible so that private planning data stays isolated.
14. As a learner, I want automatic plan creation/revision to leave my exam date and protected commitments unchanged so that automation cannot exceed its authority.
15. As a learner, I want Today to stay compact while still letting me review and edit these constraints without navigating through a large settings surface.
16. As a keyboard or assistive-technology user, I want properly labelled native inputs and truthful loading/error/saved states so that the editor is usable without hidden interaction requirements.

## Implementation Decisions

- Add one versioned learner-goals record family keyed by learner and monotonically increasing version.
- A snapshot contains daily available minutes, an optional exam date, protected commitments, and audit timestamps.
- Protected commitments are learner-entered dated constraints with a short title. This slice does not model external calendars, recurring shifts, or institutional assignment objects.
- The initial read returns an explicit unconfigured state (version zero, no daily minutes, no exam date, no commitments) rather than inventing defaults.
- Learner updates require the expected current version. The API rejects stale writes with a conflict response.
- Daily minutes must be within a physically possible day. Exam dates and protected commitment dates may not be in the past.
- Titles are trimmed, must remain non-empty, and are bounded to a short human-readable length.
- Semantically identical updates are no-ops and do not create a new version.
- Undo restores the immediately previous stored snapshot as a new current version. History is append-only.
- Automatic planning has no write route/service for learner goals in this slice. Existing automatic plan operations must not mutate the goals version.
- The authenticated surface is learner-scoped; no learner identifier is accepted from the client.
- The Today UI uses the existing design tokens/components and remains compact: a summary is visible by default and editing is progressively disclosed.
- No new client state library, form library, validation dependency, or design-system abstraction is introduced.

## Testing Decisions

- Test through the existing authenticated HTTP integration seam, not repository internals.
- Cover the unconfigured state, first save, meaningful-version increment, no-op update, stale-version rejection, input validation, learner isolation, planner non-mutation, and undo.
- Reuse the existing API integration fixture and PostgreSQL migration reversibility test.
- Test the UI through the existing Playwright learner seam: register, configure the goal, observe persisted summary, change it, and undo it.
- Browser assertions target user-visible behavior and accessible controls, not implementation details.
- Heavy checks, builds, database tests, and Playwright run only in GitHub Actions per project policy.

## Out of Scope

- Full onboarding flow.
- Exam registry selection/aliases/blueprints.
- Recurring calendars, shift recurrence rules, external calendar sync, or timezone scheduling.
- Institution-authored protected deadlines and the multi-tenant authorization model that will own them.
- Capacity-aware next-best-action scheduling beyond preserving these constraints for later planner consumption.
- Predicted readiness, pass probability, or any other unvalidated score.
- Deployment.

## Further Notes

- Maps directly to CORE-02 and prepares PLAN-02 / AI-04 without claiming those later requirements complete.
- The master plan states that learner changes to availability or exam date must be easy and reversible, and that official exam dates/protected deadlines are not automatic-agent actions.
- Existing source of truth for UI styling remains the current Medical OS design tokens and shipped learner surfaces.
