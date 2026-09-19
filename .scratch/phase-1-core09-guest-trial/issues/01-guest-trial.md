# 01: CORE-09 guest trial before sign-up

Status: implemented-pending-ci

Requirement ID: CORE-09.

Implement `../spec.md` through the existing auth, question, and practice seams.

- [x] RED HTTP contract exists before implementation.
- [x] Guest start exposes at most three deterministic published/non-quarantined questions.
- [x] Pre-answer payloads contain no answer key or rationale.
- [x] Guest answer uses first-answer-wins and idempotent replay.
- [x] Raw guest tokens are not stored durably and expire after 24 hours.
- [x] Guest token cannot authorize authenticated learner APIs.
- [x] Registration atomically migrates only answered guest items into ordinary practice/attempt records.
- [x] Invalid/expired/converted migration tokens fail closed.
- [x] Login UI offers a compact, accessible sample-question path.
- [x] No new dependency or parallel quiz engine is introduced.
- [x] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [x] Traceability is updated truthfully.
- [ ] Heavy verification is delegated to GitHub Actions; deployment is skipped.

## Comments

- Deployment is explicitly out of scope per owner instruction.
- Abuse control in this slice is deliberately content-bounded: repeated guest starts reveal the same maximum three deterministic samples rather than arbitrary bank items.
- Local closeout: `cargo fmt --all -- --check` and `git diff --check` pass.
- Matt review: Standards 0 findings; Spec 0 findings.
- Ponytail review: lean implementation; no removable abstraction/dependency identified.
- Hallmark closeout: one long mobile CTA was shortened to the spec wording `Try sample questions`; no remaining UI finding in this slice.
- Heavy acceptance remains pending GitHub Actions.
