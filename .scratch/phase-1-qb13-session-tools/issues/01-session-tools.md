# 01: QB-13 Session tools baseline

Status: implementation-started

Requirement IDs: QB-13, plus the QB-04 assisted-evidence path exercised by tutor hints.

Implement ../spec.md through the existing practice session API and Medical OS session workspace.

- [ ] RED API/browser contracts are committed before implementation.
- [ ] Shared Rust calculator engine is exposed without duplicate formulas.
- [ ] Responsive session tool tray provides calculator, converter, and exactly four text sizes.
- [ ] Tutor-only hint action records durable hint use and answer attempts become assisted.
- [ ] Timed sessions reject hint access.
- [ ] Countdown warning derives from the existing server deadline and auto-submit path.
- [ ] Submission summary includes assisted count alongside existing server-derived result fields.
- [ ] No new third-party dependency is introduced.
- [ ] Heavy verification is reserved for GitHub Actions.
- [ ] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [ ] Traceability is updated truthfully.

## Comments

- Public test seams are the authenticated practice/tools HTTP API and the real session browser workspace.
- Deployment is explicitly skipped.
