# 01: UX-01 / UX-02 session workspace navigation and focus controls

Status: implemented; heavy browser/build acceptance pending GitHub Actions

Requirement IDs: UX-01, UX-02.

Implement `../spec.md` through the existing session browser workspace.

- [x] RED browser contract is committed before implementation.
- [x] Previous/next navigation works without forcing an answer.
- [x] Navigator exposes current, answered, unanswered, not-visited, and independent marked status.
- [x] Marked/unanswered filters and direct question jump work.
- [x] Submission status counts and first-unanswered shortcut are present.
- [x] N/Right, P/Left, F, E, H, and option-letter keyboard behavior matches the spec outside editable controls.
- [x] Elimination mode and touch elimination never count as an answer.
- [x] Horizontal question swipe and option long-press/swipe gestures are implemented conservatively.
- [x] Current question, draft selections, visited state, and eliminated options restore from validated browser-local state.
- [x] Focus Mode uses fullscreen and progressive wake-lock support with honest Do Not Disturb messaging.
- [x] No new dependency is introduced.
- [x] Heavy verification is reserved for GitHub Actions.
- [x] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [x] Traceability is updated truthfully.

## Comments

- Public test seam: the real session browser workspace. Existing API contracts remain authoritative for answers, marks, hints, deadlines, and submission.
- RED contract checkpoint: `a38654a`.
- Standards review found no hard documented-standard violations. Spec review findings were closed in implementation: wake-lock denial is surfaced, duplicate acquisition is guarded, fullscreen exit errors are accurate, and touch coverage now uses a touch-capable browser context with swipe/long-press restore behavior.
- Ponytail closeout kept the change inside the existing route with no new dependency or helper abstraction. Hallmark closeout replaced the asymmetric Focus Mode side stripe with normal tokenized containment.
- This worktree has no usable installed `node_modules`, so no local Svelte compiler, build, or Playwright acceptance is claimed. Heavy acceptance remains delegated to GitHub Actions per repository policy; the account currently has a known billing/spending-limit runner-start blocker.
- Deployment is explicitly skipped.
