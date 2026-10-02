# Full account archive

## Problem Statement

Students can download a JSON file from Account, but it currently contains only a small subset of their Medical OS data. A successful download therefore looks more complete than it is, and relationships, stable identifiers, and the archive's exclusions are not described.

## Solution

Provide a self-describing, versioned archive of the authenticated learner's exportable records. Every included row must be selected by direct account ownership or an explicit membership/contribution relationship. The archive must state included and excluded categories, preserve identifiers and provenance needed to interpret the learner's records, and use one consistent database snapshot. It must never contain credentials, tokens, other learners' private records, or protected course/question content. Large exports need bounded, asynchronous delivery with explicit completion and failure states.

The existing Account download remains the student-facing entry point and keeps its current visual system and interaction pattern.

## User Stories

1. As a student, I want an archive format and version in my download so that I can identify and safely process it later.
2. As a student, I want the archive to list what it includes and excludes so that I can understand its coverage before relying on it.
3. As a student, I want my profile, account preferences, accommodations, notification settings, and engagement settings so that I can retain the personal settings I entered.
4. As a student, I want my practice sessions, session items, attempts, mock attempts, concept evidence, marks, reports, re-test cards, and re-test history so that I can retain my learning record.
5. As a student, I want my own decks, rights-eligible cards and reviews, notes, note concepts, links, and collections so that I can move my study materials.
6. As a student, I want my goals, protected commitments, plans, tasks, revisions, and intervention outcomes so that I can preserve my planning history.
7. As a student, I want my Coach history and learner memory so that I can retain information I supplied and the assistance I received.
8. As a student, I want my notifications, QOTD answers, daily engagement, XP, and achievements so that I can keep my engagement history.
9. As a student, I want my article-read and source-change assignment metadata, plus private-import metadata, so that I can retain my library activity without receiving content I do not own.
10. As a student, I want my simulation runs, team membership, my own handovers, invitation and acknowledgement metadata, appeals, feedback, portfolio, CE activities, and exam outcomes so that I can retain my professional learning evidence without copying peer-authored text.
11. As a student, I want my own community, duel, challenge, competition, and league contributions so that I can retain my participation without receiving other learners' private data or shared invite tokens.
12. As a student, I want safe device, identity-link, entitlement, referral, institutional-membership, and offline-download metadata so that I can understand the account links and access records associated with me.
13. As a student, I want a failed or oversized export to report a clear failure rather than produce a truncated file that appears complete.
14. As a student with a large account, I want an asynchronous export with an explicit ready, failed, or expired state so that the download does not depend on one long browser request.
15. As a student, I want linked notes and learner annotations handled without embedding or licensing protected course or question content.
16. As a student, I want my archive isolated from every other account and institution so that no other learner's private information is exposed.

## Implementation Decisions

- Keep the existing authenticated `GET /v1/me/export` route and Account download as the public seams.
- Use a documented JSON archive format with a monotonically managed format version. Preserve the existing account-export fields where practical and add explicit coverage metadata.
- Read all archive data from one read-only repeatable-read transaction.
- Select fields explicitly. Do not serialize whole database rows because future credential or internal columns must not silently enter an archive.
- Use direct `user_id` ownership where available. For indirect records, require an explicit learner membership or contribution. Never export a whole institution, cohort, group, duel, competition, or shared conversation to one member.
- Export safe metadata for identity, devices, leases, receipts, private imports, and licensed material. Exclude credential material, one-use or shared tokens, signatures usable as proof, protected source text/media, and other people's private content.
- Preserve the existing rights checks for question-linked learner text until the rights contract explicitly permits an export; include the exclusion in the archive metadata.
- Apply the same live question-rights check to source-linked flashcards and their review history. Preserve only the authenticated learner's role/outcome for shared challenges, and only that learner's authored handovers; omit peer text and bearer invitation tokens.
- Bound synchronous resource use and fail the whole request explicitly if the limit is exceeded. Asynchronous delivery requires a durable, private artifact-storage and expiry path; do not store unencrypted archives in an improvised location.
- Preserve the Account page's existing type, spacing, card, button, responsive, and state patterns. Change only the copy/state contract needed to describe the versioned archive accurately.

## Testing Decisions

- Test the authenticated HTTP route as the highest existing seam: format/version, included/excluded coverage, stable IDs/timestamps, read-snapshot behavior where observable, per-user isolation, and security-sensitive field exclusions.
- Seed two learner accounts with private records and shared institution/group membership; assert that one archive includes only the requesting learner's contribution and membership metadata.
- Exercise cross-institution denial and learner records linked to rights-revoked content.
- Test size-bound failure as an all-or-error response with no partial archive.
- Test the actual Account download in Playwright, parse the downloaded JSON, and verify the format and category contract plus failure messaging.
- Use the existing Rust integration harness and `tests/e2e/account.spec.ts`; run the repository's exact-source GitHub Actions gates. Avoid local heavy test/build workloads.

## Out of Scope

- Legal decisions about erasure, retention exceptions, audit retention, backups, shared contributions, and statutory response deadlines.
- Exporting licensed course/question text, image binaries, private-document contents, or other learners' private text.
- Claiming deletion because account access has been disabled.
- New visual patterns or a redesign of the Account page.

## Further Notes

The current storage implementation has no durable object-store seam for private export artifacts. The archive's ownership, versioning, and coverage can be implemented at the existing HTTP boundary, but large-account asynchronous delivery remains open until a protected storage/expiry design is available. This spec is not a production-acceptance claim.
