# IMG-04 — Version-pinned image concept links

Status: in-progress
Requirement IDs: IMG-04, ARCH-02
Triage label: ready-for-agent
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §15; `.scratch/img04-anatomy-linkage/spec.md`

## Acceptance

- An authorized editor replaces a case's concept mappings; every link stores
  the current immutable concept version and the audit log records the change.
- Learner image-case responses include linked concepts only after the existing
  display-rights gate succeeds.
- The annotation workspace supports mapping; the learner image page shows
  the pinned concept details without implying diagnostic validity.
- API integration and real-API browser E2E cover the complete author-to-learner
  flow. Generated response bindings remain current in CI.

## Comments

- The existing concept identity/version model is canonical; image cases keep
  references to it instead of creating a second anatomy vocabulary.
