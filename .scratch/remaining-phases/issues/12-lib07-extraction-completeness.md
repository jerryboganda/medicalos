# LIB-07 — Document extraction completeness reports

Status: ready-for-human
Requirement IDs: LIB-07, ADMIN-02, TRUST-07
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §12.3
Implementation state: rights-gated manifest, derived coverage states, append-only independent reviews, and the admin QA surface are implemented; parser/OCR and isolated processing infrastructure remain outside this slice.

## Problem Statement

Document extraction can silently omit pages, tables, figures, equations, or
uncertain medical quantities. The API currently has no structured record that
lets an editor see what the parser expected, what it returned, and which
regions still need review.

## Solution

Add a content-free extraction report contract for trusted processing tools to
submit. A report is bound to a source-file checksum, parser version, format,
and active rights grant that permits document extraction. It records expected,
extracted, uncertain, and critical region references, plus the malware-scan
state. The API derives missing regions and a truthful status. A different
admin reviewer may approve or reject the complete report; approved reports
must have a clean scan, no missing regions, and explicit verification of every
critical or uncertain region.

## User Stories

1. As an editor, I want every report tied to a checksum and parser version so
   that a new extraction cannot overwrite the provenance of an earlier one.
2. As an editor, I want to compare expected and extracted region references
   so that omitted pages, tables, figures, equations, or captions are visible.
3. As a reviewer, I want uncertain and critical regions called out explicitly
   so that important tables and medical quantities receive human review.
4. As an operator, I want blocked and unscanned sources to remain visibly
   blocked or pending so that they cannot be described as complete.
5. As an operator, I want a distinct reviewer to approve or reject a report so
   that extraction completeness has a traceable human decision.
6. As an operator, I want report and review mutations audited without storing
   document content or reviewer notes in the general audit stream.

## Implementation Decisions

- Reuse the authenticated admin API and existing admin console.
- Add `document_extraction` to the rights-use vocabulary. Report creation
  requires a current, non-revoked grant that permits this use.
- Store only a filename label, source checksum, format, parser version,
  region-reference lists, scan state, author, and timestamps. Never accept or
  retain source bytes or extracted prose in this report API.
- Allow PDF, DOCX, PPTX, EPUB, structured web export, image, and transcript
  format labels. Region references are stable parser-supplied anchors, not
  document content.
- Reports and review decisions are append-only. A later parser version or
  repeat review creates a new row rather than changing prior evidence.
- Derive `blocked`, `incomplete`, `review_required`, `rejected`, or `complete`
  from scan state, missing/uncertain regions, critical-region review, and the
  review decision. Never accept a client-supplied status.
- Bind review to a different authenticated admin user than the report author.
- Cap report list responses and bound every reference list and free-text field.
- This slice exposes a parser result contract and quality workflow. Actual
  binary parsing, malware scanning, OCR, sandboxing, and production worker
  deployment need isolated infrastructure and remain explicit gates.

## Testing Decisions

- Test at the authenticated HTTP API seam: rights gating, invalid manifests,
  derived missing-region status, blocked/unscanned states, reviewer separation,
  approval/rejection, immutability, and audit privacy.
- Exercise the existing admin surface through Playwright for report entry and
  status review, including a narrow mobile viewport.
- Build and test checks passed in GitHub Actions run 36159484978; do not run
  heavy jobs on local or production hosts.
- The API tests assert user-visible responses and durable restrictions only
  where needed to prove append-only behavior; no parser internals are mocked.

## Out of Scope

- Processing binary files, malware scanning, OCR, multimodal recovery, or
  document uploads.
- Claiming extraction completeness from a report without the eventual trusted
  parser, scanner, and isolated processing service.
- License verification, clinical review, or permission to retain any source.

## Further Notes

The report endpoint does not execute or attest a parser. `complete` means the
submitted manifest satisfies the API's coverage and review rules; production
acceptance still needs a trusted parser/scanner integration and evidence that
the extraction grant matches the actual contract.

## Implementation Record

- Added a content-free manifest API and admin workflow. Reports are bound to
  active `document_extraction` grants, source checksums, formats, and parser
  versions; missing regions and status are derived by the API.
- Added a separate-review requirement for critical or uncertain regions,
  append-only report/review storage, and audit events that omit reviewer notes.
- Added API and mobile browser regression cases; tests and builds passed in
  GitHub Actions run
  [36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978).
