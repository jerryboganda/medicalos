# 01: CORE-09 guest trial before sign-up

Status: ready-for-agent

Requirement ID: CORE-09.

Implement `../spec.md` through the existing auth, question, and practice seams.

- [ ] RED HTTP contract exists before implementation.
- [ ] Guest start exposes at most three deterministic published/non-quarantined questions.
- [ ] Pre-answer payloads contain no answer key or rationale.
- [ ] Guest answer uses first-answer-wins and idempotent replay.
- [ ] Raw guest tokens are not stored durably and expire after 24 hours.
- [ ] Guest token cannot authorize authenticated learner APIs.
- [ ] Registration atomically migrates only answered guest items into ordinary practice/attempt records.
- [ ] Invalid/expired/converted migration tokens fail closed.
- [ ] Login UI offers a compact, accessible sample-question path.
- [ ] No new dependency or parallel quiz engine is introduced.
- [ ] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [ ] Traceability is updated truthfully.
- [ ] Heavy verification is delegated to GitHub Actions; deployment is skipped.

## Comments

- Deployment is explicitly out of scope per owner instruction.
- Abuse control in this slice is deliberately content-bounded: repeated guest starts reveal the same maximum three deterministic samples rather than arbitrary bank items.
