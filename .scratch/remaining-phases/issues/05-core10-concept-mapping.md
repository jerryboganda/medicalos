# CORE-10 — Curriculum mapping to concept identities

Status: ready-for-human
Requirement IDs: CORE-10, QB-12, ADMIN-06
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§2.1, 5.5, 11.1
Implementation state: API/browser CI acceptance passed in run 36159484978.

## Domain rule

Curriculum nodes remain the exam-scoped navigation and reporting taxonomy.
Concept identities are stable and reusable across nodes and exams; their
descriptions are immutable numbered versions. A node can map to multiple
concepts, and a concept can map to multiple nodes. Learner-entered note tags
remain distinct from canonical concept identities.

## Acceptance

- Administrators can create a stable concept identity with its first numbered
  version, list current versions, and add later immutable versions.
- Administrators can replace the concept mappings for a hierarchy node. Empty
  mappings clear the set; invalid identities fail without partial changes.
- A concept can be reused across multiple nodes, and one node can map to more
  than one concept.
- The editorial console exposes concept creation, versioning, and node mapping
  within the existing app shell.
- Mutations are audited, require the existing admin authorization, and do not
  rewrite learner-entered note tags.

## Verification boundary

Author API integration and Playwright coverage. Do not run local builds,
tests, or screenshots; final GitHub Actions is the acceptance gate.

## Implementation record

- Added stable concept identities, immutable numbered descriptions, and
  many-to-many curriculum mappings in migration `0034_core10_concepts`.
- Added admin-authenticated APIs for listing/creating concepts, adding a
  version, and replacing or reading a node's mapping. Mapping changes and
  audit records commit together.
- Added responsive editorial controls for concept creation, versioning, and
  node mapping, with API and mobile browser coverage authored.
- API/browser coverage and the shared WASM build passed in GitHub Actions run
  [36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978).
