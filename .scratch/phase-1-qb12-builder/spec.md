# QB-12 Phase 1 - Qbank builder

## Problem Statement

Practice currently launches only a single-chapter Today task. QB-12 requires a real one-screen builder that can select multiple curriculum branches, expose truthful counts, apply four exact question pools, filter by difficulty/high-yield state, persist marked questions, and reuse saved/recent configurations without inventing unsupported offline behavior.

## Solution

Extend the existing practice session API and Practice destination instead of creating a parallel quiz engine. A single authenticated builder-read seam returns the curriculum tree with available/attempted/unattempted counts plus filtered availability for the current selection. Existing tutor/timed session creation accepts multiple chapter IDs and builder filters while remaining backward compatible with single chapter_id callers.

Persistent question marks use one minimal user/question table and one idempotent mark-state endpoint. Saved presets and repeat-last setup stay in browser local storage because they are personal UI convenience state, not shared learning evidence.

## User Stories

1. As a learner, I can search and select one or many subjects, systems, or chapters from the existing curriculum hierarchy.
2. As a learner, every hierarchy row shows available, attempted, and unattempted counts derived from my evidence.
3. As a learner, I can choose exactly one of four pools: all, incorrect + skipped, unattempted, or marked.
4. As a learner, incorrect + skipped reflects my latest recorded attempt for each question, so later correct re-practice removes the item from that pool.
5. As a learner, unattempted means I have no recorded attempt for that question.
6. As a learner, I can select one or more difficulties and optionally restrict to high-yield questions.
7. As a learner, I can request 5, 10, 20, 25, 40, 50, 100, a custom count, or all available.
8. As a learner, the product never creates more items than exist and tells me the truthful availability for my current selection.
9. As a learner, I can save named presets locally and repeat my last setup without server-side configuration bloat.
10. As a learner, I can mark/unmark a question and use those durable marks as the marked-only pool later.
11. As a learner, I can launch Quick 10 from Today through the same real session engine.

## Public Contracts

- GET /v1/practice/builder
  - Authenticated.
  - Optional query: chapter_ids (comma-separated UUIDs), pool, difficulties (comma-separated), high_yield.
  - Returns ordered curriculum nodes with available, attempted, unattempted counts.
  - Returns selection counts after the requested difficulty/high-yield filters and an exact matching count for the selected pool.
- POST /v1/practice/sessions
  - Existing chapter_id remains supported.
  - Adds optional chapter_ids, pool, difficulties, high_yield, and all_available.
  - Tutor/timed requests clamp the requested count to matching availability, with a maximum explicit requested count of 100.
  - Response includes requested_count, available_count, actual question_count, and availability_message when clamped.
- PUT /v1/questions/versions/:id/mark
  - Body: marked boolean.
  - Idempotently persists or removes the learner's mark.
- GET /v1/practice/sessions/:id
  - Each item includes the current learner-specific marked state.

## Pool Semantics

- all: every published, non-quarantined question matching curriculum and filters.
- incorrect_skipped: questions whose latest learner attempt is incorrect or explicitly skipped (chosen_index IS NULL).
- unattempted: questions with no learner attempt.
- marked: questions currently present in the learner's durable mark set.
- Due reviews are not a fifth pool.

## UI Decisions

- Preserve the existing Medical OS app shell, tokens, typography, spacing, and controls.
- Practice remains one responsive screen with collapsible sections; no multi-step wizard.
- Selecting a subject/system expands to its descendant chapters in the client request, while the backend receives normalized chapter IDs.
- The tree is searchable by node name.
- Availability refreshes from the builder-read seam whenever selection or filters change.
- Saved presets and last setup use versioned local-storage keys and fail gracefully when storage is unavailable.
- Existing Today-plan launchers remain valid; Practice gains the builder rather than a second visual language.

## Testing Decisions

- API integration contracts cover hierarchy counts, multi-chapter creation, count clamping, difficulty/high-yield filtering, latest-attempt pool semantics, unattempted semantics, durable marks, and marked state in session detail.
- Browser E2E covers the one-screen builder, search/selection, truthful availability, saved preset/repeat-last behavior, real session launch, and Quick 10 from Today.
- Heavy Rust tests, builds, PostgreSQL integration, and Playwright run only in GitHub Actions.
- Local verification remains formatting, static inspection, and diff checks.

## Out of Scope

- Download-pack/offline execution until the real local-pack data layer exists.
- Due-review pooling, adaptive Smart Practice, mock-test builder, or full QB-06 session-tool completion.
- Server-synced saved presets.
- Deployment.

## Completion Boundary

QB-12 Phase 1 is complete when the connected builder, four real pools, truthful availability, filters, durable marked pool, local presets/repeat-last, and Today Quick 10 are implemented. Offline downloaded-pack behavior remains a separately gated local-data capability rather than a fake UI promise.
