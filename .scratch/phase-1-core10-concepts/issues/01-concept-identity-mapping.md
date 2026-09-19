# 01: Concept identity mapping foundation

Status: in-progress (foundation implemented; GitHub Actions acceptance billing-blocked; learner-evidence migration remains pending)

Requirement IDs: CORE-10 (partial foundation; learner evidence remains chapter-keyed until a later behavioral migration).

Implement the concept-identity foundation in `../spec.md` through the existing seeded PostgreSQL integration seam.

- [x] Stable concept identities and immutable numbered concept versions are persisted.
- [x] Curriculum nodes map many-to-many to concept versions.
- [x] Question versions map many-to-many to concept versions with explicit primary/secondary relations.
- [x] The database enforces at most one primary concept per question version.
- [x] Synthetic chapters each receive one published version-1 concept mapping.
- [x] All four synthetic question versions receive the correct primary concept mapping.
- [x] Existing chapter-based practice, planning, and learner-state behavior remains unchanged in this foundation slice.
- [x] The red integration contract was committed before schema/seed implementation (`e0303aa`).
- [x] Heavy acceptance is delegated to GitHub Actions; local verification stays lightweight.
- [x] Matt Standards/Spec review and Ponytail review are completed for this foundation slice.
- [x] Traceability remains truthful about the pending learner-state and cross-modality migrations.

## Comments

- The testing seam is the existing seeded PostgreSQL integration contract because the current product has no public curriculum interface that should expose concept identifiers.
- No speculative curriculum endpoint or unmodeled content-modality table is part of this ticket.
- Red CI run `35431732049` executed zero runner steps because GitHub reported failed account payments or an insufficient spending limit; it is not functional red evidence.
- Implementation CI run `35431943529` was blocked by the same billing/spending-limit state before any runner step executed, so no functional green claim is made.
- Matt Standards/Spec review found one delivery-risk issue: new `query!` calls would have required fresh tracked SQLx offline metadata while CI cannot generate it. Commit `6c093fa` replaced only the new CORE-10 seed inserts with runtime `sqlx::query` bindings; no remaining Standards or Spec finding remained after that repair.
- Ponytail review: Lean already. Ship the foundation; no dependency, service layer, endpoint, or speculative modality model was added.
