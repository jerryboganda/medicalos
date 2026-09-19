# 01: QB-12 Qbank builder

Status: implementation-complete; acceptance-unverified

Requirement IDs: QB-12, with the minimum durable marking seam required by QB-12's marked pool. This does not claim full QB-06 completion.

Implement ../spec.md through the existing practice routes and app shell.

- [x] Builder-read API exposes the curriculum hierarchy and truthful available/attempted/unattempted counts.
- [x] Tutor/timed session creation supports multiple chapters without breaking existing single-chapter callers.
- [x] Exactly four pools are implemented with latest-attempt semantics.
- [x] Difficulty multi-select and high-yield filtering are real backend filters.
- [x] Explicit counts up to 100 and all-available are supported.
- [x] Requested counts are clamped to actual availability and the response carries a truthful message.
- [x] Question marks persist beyond sessions and feed the marked pool.
- [x] Session detail exposes mark state and the session UI can mark/unmark a question.
- [x] Practice is a single-screen searchable/collapsible builder using the existing design system.
- [x] Saved presets and repeat-last use resilient local storage.
- [x] Today provides a real Quick 10 action.
- [x] RED API/browser contracts are committed before implementation.
- [x] Heavy verification is reserved for GitHub Actions; no heavy local acceptance run was substituted.
- [x] Matt Standards/Spec review and Ponytail review are completed.
- [x] Traceability is updated truthfully.

## Comments

- Incorrect + skipped uses the learner's latest recorded attempt for a question, not historical-ever-wrong semantics.
- Unattempted means no attempt exists for the learner/question.
- Saved presets are UI convenience state and therefore do not justify a new server table.
- Offline/download-pack behavior is intentionally deferred until a real local-pack seam exists.
- Deployment is explicitly skipped.
- Local implementation commit: `c41bea1904e4923e8853df770070fcb9c586672a`.
- Acceptance remains unverified: the implementation commit is not on the remote branch from this execution path, so no GitHub Actions run can be claimed for it. The previously attempted RED workflow run was also blocked before runner steps by the GitHub account billing/spending-limit state.
