# 01: UX-01 / UX-02 session workspace navigation and focus controls

Status: ready-for-agent

Requirement IDs: UX-01, UX-02.

Implement `../spec.md` through the existing session browser workspace.

- [ ] RED browser contract is committed before implementation.
- [ ] Previous/next navigation works without forcing an answer.
- [ ] Navigator exposes current, answered, unanswered, not-visited, and independent marked status.
- [ ] Marked/unanswered filters and direct question jump work.
- [ ] Submission status counts and first-unanswered shortcut are present.
- [ ] N/Right, P/Left, F, E, H, and option-letter keyboard behavior matches the spec outside editable controls.
- [ ] Elimination mode and touch elimination never count as an answer.
- [ ] Horizontal question swipe and option long-press/swipe gestures are implemented conservatively.
- [ ] Current question, draft selections, visited state, and eliminated options restore from validated browser-local state.
- [ ] Focus Mode uses fullscreen and progressive wake-lock support with honest Do Not Disturb messaging.
- [ ] No new dependency is introduced.
- [ ] Heavy verification is reserved for GitHub Actions.
- [ ] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [ ] Traceability is updated truthfully.

## Comments

- Public test seam: the real session browser workspace. Existing API contracts remain authoritative for answers, marks, hints, deadlines, and submission.
- Deployment is explicitly skipped.
