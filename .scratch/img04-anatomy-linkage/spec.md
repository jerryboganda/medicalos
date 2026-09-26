# IMG-04 — Versioned anatomy and image-case links

Status: in-progress
Requirement IDs: IMG-04, ARCH-02

## Outcome

Editorial staff can map an image case to stable curriculum concept identities.
Each link pins the concept version selected at mapping time. Learners see the
linked concept name, key, version, and definition beside an image case only
when that case remains available under its display-rights grant.

## Decisions

- Use existing `concepts` / `concept_versions` as the shared identity and
  immutable description source. Do not duplicate anatomy terminology in the
  image-case record.
- Staff replace the complete mapping set in one authenticated, audited API
  operation. Empty input clears the mapping; unknown or duplicate identities
  are rejected without changing the current set.
- Version changes do not rewrite old image links. Staff must explicitly save a
  new mapping to select the newer concept version.
- A concept link is an educational navigation aid, not a diagnosis or claim of
  clinical validity. Existing image rights checks continue to gate learner
  image-case reads.

## Acceptance

- Admin API GET/PUT reads and replaces concept links, validates at most 50
  distinct existing concepts, pins current versions transactionally, and
  records before/after values in the audit log.
- Learner list/detail responses include the pinned concept links after the
  active image-display-rights check.
- The editorial image-annotation workspace can select a case, map concepts,
  save, and report success or failure.
- API integration coverage proves authorization, version pinning, and learner
  visibility. Browser E2E creates a real case and concept, maps it in the
  editor, changes the concept version, and verifies that the learner still
  sees the pinned version.
- Generated TypeScript for the response DTO is checked through the ARCH-02 CI
  export pipeline.

## Deliberate limits

Concept mapping does not establish clinical correctness, replace image review,
or clear image rights. DICOM de-identification and pixel-integrity workflows
remain part of IMG-03.
