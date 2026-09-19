# 01: QB-13 Session tools baseline

Status: implementation-complete-local — heavy CI/browser acceptance blocked before runner start

Requirement IDs: QB-13, plus the QB-04 assisted-evidence path exercised by tutor hints.

Implement ../spec.md through the existing practice session API and Medical OS session workspace.

- [x] RED API/browser contracts are committed before implementation.
- [x] Shared Rust calculator engine is exposed without duplicate formulas.
- [x] Responsive session tool tray provides calculator, converter, and exactly four text sizes.
- [x] Tutor-only hint action records durable hint use and answer attempts become assisted.
- [x] Timed sessions reject hint access.
- [x] Countdown warning derives from the existing server deadline and auto-submit path.
- [x] Submission summary includes assisted count alongside existing server-derived result fields.
- [x] No new third-party dependency is introduced.
- [x] Heavy verification is reserved for GitHub Actions.
- [ ] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [x] Traceability is updated truthfully.

## Comments

- Public test seams are the authenticated practice/tools HTTP API and the real session browser workspace.
- Deployment is explicitly skipped.
- Matt Standards/Spec and Ponytail reviews were completed against the working diff. Review fixes added server-side rejection of post-answer hints, explicit invalid-calculator coverage, blank converter handling, and keyboard focus transfer/return for the responsive tools sheet.
- Static Hallmark/token/accessibility review is complete. Real-browser Hallmark closeout at the required responsive widths remains pending the GitHub Actions/browser acceptance run.
- Local lightweight evidence: `cargo fmt --all -- --check` PASS and `git diff --check` PASS. No local integration/build/Playwright run was performed. A direct Svelte parser probe was unavailable because this worktree has no installed `node_modules`.
- GitHub Actions run `35436593968` was created for heavy acceptance but failed before any runner steps because the account reported failed payments or an Actions spending-limit requirement. Rust, E2E, site, and client jobs therefore provide no code-verification evidence from that run.
