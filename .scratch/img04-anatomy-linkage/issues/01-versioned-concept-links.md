# IMG-04 — Version-pinned image concept links

Status: complete
Requirement IDs: IMG-04, ARCH-02
Triage label: complete
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
- 2026-09-26: Acceptance verified in GitHub Actions runs
  [36221287354](https://github.com/jerryboganda/medicalos/actions/runs/36221287354)
  and [36221289903](https://github.com/jerryboganda/medicalos/actions/runs/36221289903):
  API authorization/version-pinning integration coverage, author-to-learner
  browser E2E, checked-in generated response types, and the full client/site
  gates passed. DICOM privacy, pixel integrity, image rights, and clinical
  review remain outside this issue and are tracked separately.
