# TRUST-02 — Versioned learner-owned data archive

Status: in-progress
Type: task

## Problem

`GET /v1/me/export` returns profile basics, selected attempts, some notes, reviews, and portfolio rows while TRUST-02 describes it as a full export. Students need an honest, versioned archive that covers their exportable records without leaking credentials, protected content, or another learner's private data.

## Acceptance criteria

- The response identifies its archive format and version and explicitly enumerates included and excluded categories.
- The archive includes the safe learner-owned account, learning, study-material, planning, Coach, engagement, library-activity, professional-evidence, community/competition, identity/device, entitlement, and offline metadata recorded in `.scratch/full-platform/privacy-data-map.md`.
- Rows are scoped by direct ownership or a verified learner membership/contribution. A second learner and a different institution cannot retrieve another learner's private rows.
- Rows carry stable IDs, relevant timestamps, provenance/version, and disposition where those concepts apply; rights-inactive source-linked flashcards and their reviews are withheld alongside notes, Coach/report text, and appeals.
- One read-only repeatable-read snapshot is used for all categories.
- Password/session/device/pack/provider secrets, one-use/shared tokens, signed proof material, protected course/question content, binary imports, and other learners' private text are excluded.
- Rights-inactive question-linked learner text follows the existing rights contract and is reported honestly as excluded.
- An oversized export fails explicitly and never produces a partial-success archive.
- The existing Account page's layout, tokens, responsive behavior, and component patterns remain intact; Playwright verifies a real download and failure state.
- Exact-source GitHub Actions pass before the feature is marked accepted.

## Remaining gate

Large-account asynchronous delivery is a separate open TRUST-02 acceptance item. The repository currently has no durable private export-artifact storage/expiry seam; do not mark TRUST-02 or this feature production-ready until that path and the owner-approved retention policy are resolved.
