# 01: Versioned learner goal profile

**What to build:** Let an authenticated learner view, edit, persist, and undo their daily study minutes, optional exam date, and protected dated commitments from Today, while stale edits are rejected and automatic planning cannot silently change the learner-owned constraints.

**Blocked by:** None (can start immediately).

**Status:** resolved — CI run 35424883754 passed Rust, client, site, and browser E2E gates.

- [x] An unconfigured learner receives an honest version-zero goal state with no invented exam date or study capacity.
- [x] A valid learner update persists daily minutes, optional exam date, and protected commitments and creates version 1.
- [x] A semantic no-op does not create another version.
- [x] A meaningful update creates the next version and a stale expected version is rejected with a conflict.
- [x] Invalid daily minutes, past dates, or invalid commitment titles are rejected without changing the current version.
- [x] Learners cannot read or mutate another learner's goal state.
- [x] Existing automatic plan creation/revision leaves the goal version, exam date, and protected commitments unchanged.
- [x] Undo restores the immediately previous snapshot as a new version without deleting history.
- [x] The Today UI presents a compact, accessible editor with truthful loading, validation, save, and error states.
- [x] Playwright verifies save, persistence, edit, and undo through the real client/API/database path.
- [x] The migration remains reversible under the repository's up/down/up test.
- [x] GitHub Actions passes; no heavy build/test/browser workload is run locally and deployment is not triggered.

## Comments

- Final acceptance evidence: GitHub Actions run 35424883754. The run passed Rust formatting, schema apply, Clippy, dependency/license checks, tests, SQLx offline cache generation, wasm shared-core build, client build, site build, and browser E2E. Deployment was intentionally skipped.
