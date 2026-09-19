# CORE-05 — Five-destination learner navigation

## Problem Statement

The learner web client currently routes authenticated users primarily through Today, Review, and Account. The master plan requires five primary learner destinations — Today, Practice, Learn, Coach, and Progress — with profile/settings concerns kept outside that primary navigation. Four of those destinations are missing, so later Phase 1 capabilities have no stable learner-facing home.

## Solution

Add the five-destination authenticated learner shell and four deep-linkable destination routes while reusing only capabilities that already exist. Today remains the default planning surface. Practice starts real study sessions. Learn exposes the real spaced-review queue. Coach exposes real persisted plan-change evidence rather than pretending an AI conversation exists. Progress exposes real learner chapter evidence. Account and sign-out remain secondary controls outside the primary destination set.

The slice is intentionally frontend-only. It does not invent backend endpoints, fake analytics, fake coach activity, or dead buttons.

## User Stories

1. As an authenticated learner, I want Today, Practice, Learn, Coach, and Progress to be the only primary learner destinations so that the app has a stable information architecture.
2. As an authenticated learner, I want the current destination to be visually and semantically identifiable so that I know where I am.
3. As a keyboard user, I want every primary navigation item to have visible focus behavior so that the shell is usable without a pointer.
4. As a mobile learner, I want the primary navigation to remain usable at narrow widths without horizontal page overflow.
5. As a learner, I want Account and Sign out to remain available without becoming primary study destinations.
6. As a learner in a study session, I want the study workspace to remain focused and not gain unrelated destination content.
7. As a learner, I want Practice to let me start a real tutor session using the existing session API so that the destination has immediate utility.
8. As a learner, I want Practice to let me start a real timed session using the existing session API so that the destination reflects currently implemented practice modes.
9. As a learner, I want Practice to handle the existing active-session conflict honestly and require explicit takeover so that a second session is never silently created.
10. As a learner, I want Learn to surface my real spaced-review queue so that an implemented learning activity lives under Learn.
11. As a learner with no review cards, I want Learn to show a truthful empty state so that the app does not fabricate learning content.
12. As a learner, I want Learn to link into the existing Review workflow when cards are available so that no duplicate review engine is introduced.
13. As a learner, I want Coach to show actual persisted plan revisions and their explanations so that agent activity is evidence-backed.
14. As a learner with no plan revisions, I want Coach to say that no plan changes are recorded yet rather than showing invented coaching activity.
15. As a learner, I want Progress to show the same persisted chapter evidence used by Today so that progress information has one source of truth.
16. As a learner with sparse evidence, I want Progress to preserve the current honest low-evidence language so that no mastery claim is fabricated.
17. As a learner, I want each destination to be deep-linkable so that future notifications and shared links can target a real route.
18. As a learner, I want loading, error, retry, and empty states on these destinations to be explicit so that network failures are not mistaken for missing data.
19. As a maintainer, I want the new destinations to reuse the existing API client, tokens, button classes, card classes, and auth guard pattern so that the slice adds no new framework or design abstraction.
20. As a maintainer, I want browser E2E coverage across the five destinations and their real capabilities so that navigation regressions are caught at the highest existing seam.

## Implementation Decisions

- Keep SvelteKit route ownership unchanged and add four additive routes: Practice, Learn, Coach, and Progress.
- Keep Today as the authenticated brand/home target.
- Render the five primary destinations in the shared authenticated shell. Account and Sign out remain secondary actions.
- Reuse the existing authenticated API client and auth state. No new state library, router abstraction, UI framework, icon package, or backend endpoint is introduced.
- Practice uses the existing `POST /v1/practice/sessions` contract for tutor and timed presets. Active-session conflicts preserve the existing explicit takeover contract.
- Learn uses the existing `GET /v1/reviews/queue` contract and routes into the existing Review page for the actual card-rating workflow.
- Coach uses the existing `GET /v1/me/today` response and displays only persisted plan revisions/explanations. It does not claim live AI chat, voice, or autonomous activity.
- Progress uses the existing `GET /v1/me/today` learner evidence array and preserves the current low-evidence disclosure rules.
- Each new page follows the existing auth-on-mount redirect pattern and truthful loading/error/empty states.
- Existing Medical OS design tokens and shared `.btn`, `.card`, `.chip`, and typography rules remain authoritative. New shell styles are limited to navigation layout/active state and responsive behavior.
- The primary destination shell remains visible on ordinary learner pages. Session workspace behavior is not broadened in this slice beyond preserving current focus.

## Testing Decisions

- The acceptance seam is Playwright against the built Svelte client, real API, and real database, matching the existing learner-loop tests.
- Tests assert external behavior only: destination links, route headings, real API-backed states, real session creation, and honest empty/evidence states.
- The first TDD slice is navigation itself: an authenticated learner can traverse all five primary destinations from the shell.
- Subsequent slices cover Practice session creation, Learn review-queue state, Coach persisted revision state, and Progress persisted learner evidence.
- Heavy browser/build/test work runs only in GitHub Actions. Local verification is limited to static inspection and `git diff --check`.

## Out of Scope

- New QBank builder controls, full content library, notebook/download manager, live Coach conversation/voice, advanced analytics, readiness prediction, portfolio, competition, billing, notification inbox, or global command/search.
- New backend endpoints or schema migrations.
- Redesign of Today, Review, Account, or the study-session workspace.
- Deployment.

## Further Notes

- This slice closes the structural CORE-05 navigation requirement using real Phase 1 capabilities already present in the repository. It does not claim completion of the deeper feature inventories assigned to Practice, Learn, Coach, or Progress.
- The five destinations are information architecture, not five separate engines. Existing review/session/plan/evidence flows are reused in place.
