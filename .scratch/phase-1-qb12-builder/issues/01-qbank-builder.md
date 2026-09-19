# 01: QB-12 Qbank builder

Status: in-progress

Requirement IDs: QB-12, with the minimum durable marking seam required by QB-12's marked pool. This does not claim full QB-06 completion.

Implement ../spec.md through the existing practice routes and app shell.

- [ ] Builder-read API exposes the curriculum hierarchy and truthful available/attempted/unattempted counts.
- [ ] Tutor/timed session creation supports multiple chapters without breaking existing single-chapter callers.
- [ ] Exactly four pools are implemented with latest-attempt semantics.
- [ ] Difficulty multi-select and high-yield filtering are real backend filters.
- [ ] Explicit counts up to 100 and all-available are supported.
- [ ] Requested counts are clamped to actual availability and the response carries a truthful message.
- [ ] Question marks persist beyond sessions and feed the marked pool.
- [ ] Session detail exposes mark state and the session UI can mark/unmark a question.
- [ ] Practice is a single-screen searchable/collapsible builder using the existing design system.
- [ ] Saved presets and repeat-last use resilient local storage.
- [ ] Today provides a real Quick 10 action.
- [ ] RED API/browser contracts are committed before implementation.
- [ ] Heavy verification runs only in GitHub Actions.
- [ ] Matt Standards/Spec review and Ponytail review are completed.
- [ ] Traceability is updated truthfully.

## Comments

- Incorrect + skipped uses the learner's latest recorded attempt for a question, not historical-ever-wrong semantics.
- Unattempted means no attempt exists for the learner/question.
- Saved presets are UI convenience state and therefore do not justify a new server table.
- Offline/download-pack behavior is intentionally deferred until a real local-pack seam exists.
- Deployment is explicitly skipped.
