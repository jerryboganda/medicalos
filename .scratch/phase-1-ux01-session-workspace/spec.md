# UX-01 / UX-02 Phase 1 — Session workspace navigation and focus controls

## Problem Statement

The practice session already supports real answers, marks, tutor hints, timing, and the responsive tool tray, but the workspace still behaves like a mostly linear page. Learners cannot freely navigate unanswered questions, see the required question-status map, eliminate options without answering, use the complete desktop keyboard map, enter a focused fullscreen study mode, or reliably return to the same in-progress question state after an interruption.

## Solution

Extend the existing session page instead of creating a second quiz shell. Add a compact question navigator and bottom navigation controls, browser-local draft/elimination/visit state that restores after reload, the remaining keyboard shortcuts, touch swipe/long-press gestures, and a progressive-enhancement Focus Mode using browser fullscreen plus Screen Wake Lock when available. Server-owned answers, marks, deadlines, and evidence remain authoritative.

## User Stories

1. As a learner, I can move to the previous or next question without being forced to answer the current question.
2. As a learner, I can open a navigator showing every question in the current session.
3. As a learner, the navigator distinguishes current, answered, unanswered, and not-visited questions with both iconography and color.
4. As a learner, a marked question remains visibly marked in the navigator independently of whether it is answered.
5. As a learner, I can filter the navigator to marked questions.
6. As a learner, I can filter the navigator to unanswered questions, including questions I have not visited yet.
7. As a learner, I can jump directly to a question from the navigator.
8. As a learner, I can see submission-status counts and jump to the first unanswered question before submitting.
9. As a keyboard learner, N or Right Arrow moves to the next question.
10. As a keyboard learner, P or Left Arrow moves to the previous question.
11. As a keyboard learner, F toggles the current question mark.
12. As a keyboard learner, E toggles elimination mode.
13. As a tutor-session learner, H requests the existing tutor hint before answering.
14. As a keyboard learner, letter keys select options when elimination mode is off.
15. As a keyboard learner, letter keys toggle option elimination when elimination mode is on.
16. As a learner typing in an input, select, textarea, or editable notes surface, session shortcuts do not fire.
17. As a touch learner, I can swipe horizontally on the question body to move between questions without accidental vertical-scroll navigation.
18. As a touch learner, I can long-press or horizontally swipe an option to eliminate it and repeat the gesture to restore it.
19. As a learner, eliminating an option never records or submits an answer.
20. As a learner, an eliminated option remains available to restore and is visually struck out without being disabled.
21. As a learner, my current question, unsent option selection, visited-state, and eliminations survive a page reload or app/browser restart that preserves local storage.
22. As a web or desktop learner, I can enter and exit Focus Mode using the browser fullscreen capability when available.
23. As a supported mobile/web learner, Focus Mode requests a screen wake lock when the browser exposes it.
24. As a learner, Focus Mode explicitly suggests enabling Do Not Disturb rather than claiming the app changed the device setting.
25. As a learner on a browser that denies fullscreen or wake-lock access, the session remains usable and reports the limitation without losing answer state.

## Implementation Decisions

- Keep the existing session route as the only learner workspace; no new route, service, or dependency is added.
- Keep answers, marks, deadlines, hint assistance, and submission receipts server-authoritative.
- Store only UI-draft state locally: current question index, per-question draft selection, visited indices, and eliminated option indices. Validate restored indices against the freshly loaded session before using them.
- Treat `marked` as an independent navigator overlay. The primary navigation state is current, answered, unanswered, or not visited.
- “Unanswered” navigator filtering includes both visited-unanswered and not-visited items because both still require an answer.
- Use the existing mark API for the F shortcut; no duplicate mark state is introduced.
- E and F are reserved command keys. Option E/F remain selectable by pointer/touch; other supported letter keys continue to select by keyboard.
- Elimination is presentation-only state. It never calls the answer API and never changes evidence counts.
- Implement touch gestures with native touch events and conservative horizontal-distance/direction thresholds. Text selection/highlighting suppresses question-swipe navigation.
- Implement Focus Mode as progressive enhancement with `requestFullscreen`/`exitFullscreen`; request Screen Wake Lock only when available and release it on exit. Do Not Disturb remains a suggestion because web clients cannot enable it portably.
- Reuse the existing Medical OS design tokens and session page styling. No new theme, font, package, or visual system is introduced.
- Deployment is out of scope for this continuation.

## Testing Decisions

- The primary seam is the real session workspace in Playwright, because navigation, shortcuts, gestures, local restore, and fullscreen are user-observable browser behavior.
- Browser E2E covers navigator statuses/filters/jump, previous/next keyboard controls, F/E/H behavior, elimination not answering, reload restore, and Focus Mode with browser APIs stubbed only where headless fullscreen/wake-lock behavior is environment-dependent.
- Touch behavior is tested through a touch-capable browser context and synthetic touch events at the DOM boundary, not through private functions.
- Existing API integration coverage remains the source of truth for answer persistence, marking, hints, deadlines, and submission; this slice does not add duplicate API tests.
- Heavy Playwright/build acceptance runs only in GitHub Actions. Local verification remains formatting, static inspection, and diff checks.

## Out of Scope

- Native iOS/Android APIs for hiding operating-system bars or enabling Do Not Disturb directly.
- Haptic feedback, because it is optional and not required for correct elimination behavior.
- Question/media prefetching and offline-pack synchronization; that is a separate session-delivery slice.
- Notes, highlighting, lab values, or theme expansion beyond the already implemented QB-13 tool tray baseline.
- Deployment, merge, tag, release, or workflow activation.

## Further Notes

The browser can suggest Do Not Disturb but cannot honestly claim to enable it. Fullscreen and wake-lock failures must therefore degrade to the normal session rather than block learning.
