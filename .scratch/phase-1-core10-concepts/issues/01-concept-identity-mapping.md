# 01: Concept identity mapping foundation

Status: ready-for-agent

Requirement IDs: CORE-10 (partial foundation; learner evidence remains chapter-keyed until a later behavioral migration).

Implement the concept-identity foundation in `../spec.md` through the existing seeded PostgreSQL integration seam.

- [ ] Stable concept identities and immutable numbered concept versions are persisted.
- [ ] Curriculum nodes map many-to-many to concept versions.
- [ ] Question versions map many-to-many to concept versions with explicit primary/secondary relations.
- [ ] The database enforces at most one primary concept per question version.
- [ ] Synthetic chapters each receive one published version-1 concept mapping.
- [ ] All four synthetic question versions receive the correct primary concept mapping.
- [ ] Existing chapter-based practice, planning, and learner-state behavior remains unchanged in this foundation slice.
- [ ] The red integration contract is committed before schema/seed implementation.
- [ ] Heavy acceptance executes only in GitHub Actions; local verification stays lightweight.
- [ ] Matt Standards/Spec review and Ponytail review are completed before closeout.
- [ ] Traceability remains truthful about the pending learner-state and cross-modality migrations.

## Comments

- The testing seam is the existing seeded PostgreSQL integration contract because the current product has no public curriculum interface that should expose concept identifiers.
- No speculative curriculum endpoint or unmodeled content-modality table is part of this ticket.
