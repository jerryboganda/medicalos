# 01: Versioned learner goal profile

**What to build:** Let an authenticated learner view, edit, persist, and undo their daily study minutes, optional exam date, and protected dated commitments from Today, while stale edits are rejected and automatic planning cannot silently change the learner-owned constraints.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] An unconfigured learner receives an honest version-zero goal state with no invented exam date or study capacity.
- [ ] A valid learner update persists daily minutes, optional exam date, and protected commitments and creates version 1.
- [ ] A semantic no-op does not create another version.
- [ ] A meaningful update creates the next version and a stale expected version is rejected with a conflict.
- [ ] Invalid daily minutes, past dates, or invalid commitment titles are rejected without changing the current version.
- [ ] Learners cannot read or mutate another learner's goal state.
- [ ] Existing automatic plan creation/revision leaves the goal version, exam date, and protected commitments unchanged.
- [ ] Undo restores the immediately previous snapshot as a new version without deleting history.
- [ ] The Today UI presents a compact, accessible editor with truthful loading, validation, save, and error states.
- [ ] Playwright verifies save, persistence, edit, and undo through the real client/API/database path.
- [ ] The migration remains reversible under the repository's up/down/up test.
- [ ] GitHub Actions passes; no heavy build/test/browser workload is run locally and deployment is not triggered.
