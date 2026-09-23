# LIB-06 — Rights-checked private document imports

Status: ready-for-human
Requirement IDs: LIB-06, ADMIN-02, TRUST-07
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§12.1, 12.2, 12.5, 19.1–19.2
Implementation state: admin rights ledger, scoped terms, owner-scoped text/Markdown imports, and rights-gated private search are implemented; final API, browser, migration rollback, license-evidence, and private-data-retention gates remain.

## Problem Statement

Learners cannot import their own permitted documents into the Library. Existing
rights references are required for media and images, but the ledger does not
yet gate private content storage or retrieval, and rights cannot be revoked
through an auditable route.

## Solution

Let operators record and revoke rights grants in the admin console. Let a
learner import a bounded plain-text or Markdown document under an active rights
record that permits both `private_import` and `display`. Keep each import
private to its owner. Expired or revoked rights immediately prevent further
content reads while preserving the learner's metadata and delete control.

## User Stories

1. As an operator, I want to record that a rights grant permits private import
   and display so that the API can enforce its stated scope.
2. As an operator, I want to revoke a rights grant with an audit record so that
   retrieval can stop when permission changes.
3. As a learner, I want to see the current rights references available for
   private import so that I can select the correct grant.
4. As a learner, I want to import a text or Markdown file under an active grant
   so that I can keep permitted material in my private Library.
5. As a learner, I want to retrieve and delete only my own imports so that
   another account cannot read my documents.
6. As a learner, I want expired or revoked material to become unreadable while
   its title and status remain visible so that the restriction is clear and I
   can remove it.
7. As a learner, I want a stable content checksum and source rights reference
   returned with an import so that I can identify the stored version.

## Implementation Decisions

- Keep the existing authenticated HTTP API and Library page as the seams.
- Add `private_import` to the rights ledger's allowed-use vocabulary.
- Store private documents with an owner, rights-record foreign key, title,
  media type, text content, SHA-256 checksum, and creation time.
- Accept only `text/plain` and `text/markdown` payloads up to 1 MiB. The browser
  reads `.txt` and `.md` files through the native File API.
- Limit each account to 25 documents and 10 MiB total. Search private content
  only when the same active rights record also permits `search`.
- The Library page tells learners not to upload patient-identifiable information.
- Import and retrieval require an unexpired, non-revoked record that permits
  both `private_import` and `display`.
- Keep expired/revoked document metadata visible to its owner, exclude its body
  from reads, and permit owner deletion.
- Revoke rights idempotently and audit only the rights-record ID and reason;
  never place imported content in the audit stream.
- Do not expose imports to Coach, other users, institutions, or offline packs.
  PDF/DOCX extraction, OCR, table/image completeness, AI processing, and offline
  copies remain separate requirements and must use their own rights checks.
- Test the public API contract with integration requests, the operator rights
  form, and the learner Library route with Playwright. Keep all execution
  deferred to final GitHub Actions acceptance.

## Testing Decisions

- API integration cases cover allowed import, invalid/expired/revoked grants,
  content retrieval, owner isolation, deletion, and revocation idempotency.
- Browser coverage imports a Markdown file, opens it, handles an unavailable
  rights list, tests operator grant/revocation, and checks mobile layouts for
  horizontal overflow.
- Assertions observe HTTP and browser behavior. No local database fixture
  queries are needed to establish user-visible outcomes.

## Out of Scope

- Binary file storage, PDF/DOCX parsing, OCR, extracted table/image analysis,
  and extraction completeness reporting (LIB-07).
- Sharing, institutional visibility, Coach retrieval, offline copies, and
  third-party license verification.

## Further Notes

The rights ledger is an operator-supplied record, not proof of an external
license. Production acceptance still requires evidence that the entered grant
matches the actual contract and that applicable private-data retention rules
are satisfied.

## Implementation Record

- Added owner-scoped text/Markdown imports with a 1 MiB limit and server-side
  SHA-256 digest.
- Import and read paths require active rights that include both
  `private_import` and `display`; rights revocation is idempotent and writes
  its audit event in the same transaction.
- The existing Library page lists active grants and owner imports, reads
  current documents, and keeps expired/revoked metadata deletable.
- The admin console records grants and shows their status; a required reason
  accompanies rights revocation and the refreshed audit trail.
- API and mobile browser regression cases are authored. No tests or builds
  have been run; GitHub Actions remains the acceptance gate.
- Rights records retain contract/version, asset and audience scope, seat,
  offline, quotation, AI, derivative, attribution, and royalty terms. This
  records the operator's grant details; it does not verify the external license.
