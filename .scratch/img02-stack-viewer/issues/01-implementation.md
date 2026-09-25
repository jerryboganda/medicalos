# IMG-02 — Viewer and independent annotation workflow

Status: needs-triage
Requirement IDs: IMG-01, IMG-02, TRUST-07
Spec: `.scratch/img02-stack-viewer/spec.md`

## Implementation record

- Added active `display` rights checks under shared locks for authoring and learner list/detail reads; a case is hidden if any image grant is inactive. The learner summary omits findings, which are returned by the rights-checked detail route only after explicit disclosure.
- Added the learner still/stack viewer with ordered buttons, slider, wheel navigation, native image rendering, CSS-only zoom, approved annotation markers, responsive layout, and education-only copy. Added structured findings with a migration for legacy text, plus an admin authoring queue and independent one-time reviewer decisions with required plain-text notes.
- Added authenticated API integration cases and Playwright coverage for missing, non-display, future, expired, and revoked grants; hidden summaries and structured details; ordered controls; plain-text annotations; required independent review notes; out-of-order findings responses; findings-fetch errors; and narrow viewports.
- The independent standards review found no hard violations; its license-policy duplication and findings-error visibility findings are fixed. Its spec review findings for wheel navigation and structured findings are implemented.
- A fresh two-axis review found no spec gap. The standards review caught that migration `0047` was missing from the shared schema registry; it is now registered. Its up migration is replay-safe and only backfills default-empty findings, preserving structured edits. An integration regression covers replay and legacy backfill; a static inventory check confirms all 47 forward migrations have matching down files and registry entries.
- Tests/builds are deferred to the final GitHub Actions pass. DICOM window/level, normal/abnormal comparison, reference-device validation, and clinical validation remain outside this code slice.
