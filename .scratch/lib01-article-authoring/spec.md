# LIB-01 / LIB-03 / LIB-04 — Article authoring and regional reading

Status: in-progress (implementation complete; final acceptance pending)
Requirement IDs: LIB-01, LIB-03, LIB-04, TRUST-01, ARCH-02

## Problem Statement

Library article rows and versions currently come from direct database fixtures. Learners can search excerpts, but there is no editorial workflow to create or revise articles, no reader route for the full version, and no interface for selecting citation, jurisdiction, and effective-date metadata.

## Solution

Complete the existing article path through the authenticated HTTP API and SvelteKit client. Administrators create and edit unpublished drafts, publish an immutable version, and attach typed citations and optional regional validity windows. Learners search and read only published versions. Their explicitly selected country and date resolve to an exact-country version first and a global version second; content for a different country is never used as a fallback.

## User Stories

1. As an editor, I want to create a draft article with a stable slug and title so that an approved library entry can be prepared without being visible to learners.
2. As an editor, I want to create a new draft from the latest published version so that revisions retain a clear version history.
3. As an editor, I want to edit citations and regional validity only while a version is a draft so that published evidence remains stable.
4. As an editor, I want to publish a validated draft explicitly so that learners never see work in progress.
5. As an editor, I want citations to identify source, page, figure, or timestamp targets and an anchor in the article so that references remain locatable.
6. As an editor, I want optional country and inclusive effective-from/effective-to dates so that guidance can vary by jurisdiction and time.
7. As a learner, I want search results to link to the complete article and show the version's scope so that an excerpt is not mistaken for the full guidance.
8. As a learner, I want to select a country and an as-of date so that the displayed version matches the context I intend to study.
9. As a learner, I want an exact-country version to take priority over a global version, and a clear unavailable state when neither applies, so that advice from another country is never silently substituted.
10. As a learner, I want citation targets rendered as text and article content rendered as plain text so that references are visible without executing imported markup.
11. As an operator, I want create, revise, and publish actions audited without copying full article bodies into the audit log.
12. As a maintainer, I want Rust-owned HTTP DTOs and observable API/browser behavior so that the client contract cannot drift silently.

## Implementation Decisions

- Reuse the existing articles, article_versions, article_citations, and audit_events tables; do not add a migration or a new dependency.
- Keep article slug and title stable on the article record. Create a new monotonically increasing version for content, source reference, citations, jurisdiction, and effective dates.
- Permit one draft per article. Create the next draft by copying the latest published version; allow edits only while status is draft. Publishing changes only the draft status; published versions remain immutable and the prior version remains available in history.
- Require an authenticated user and the existing global admin token on every editorial endpoint. Audit actor, action, article/version IDs, status, scope, and citation count; omit article body and secrets.
- Validate slug, title, source reference, citation kind/anchor/target, two-letter uppercase jurisdiction codes, and date ordering at the HTTP boundary. Blank jurisdiction means global guidance. Effective date bounds are inclusive.
- Add optional jurisdiction and as-of query parameters to article search and article read. With no jurisdiction, resolve only globally scoped versions. With a jurisdiction, resolve an exact-country published version first, then a global published version. Filter by the requested as-of date; default to the server's current UTC date. Never fall back to a different country.
- Return an honest not-available response when no published version matches the selected scope and date. Do not infer a learner's country from IP, browser locale, or account data.
- Add an admin article workspace and a learner article reader. Render body and citation values as escaped text; do not use raw HTML.
- Generate client-consumed article DTOs from Rust with the existing ts-rs workflow. Keep transport and route strings in the existing API client.
- Preserve the separate clinical-review, source-rights, and production acceptance gates. Publishing via the admin workflow does not claim clinical validation or license clearance.

## Testing Decisions

- Use the existing authenticated Axum integration seam for admin authorization, draft creation and edit, version numbering, citation validation, immutable published versions, audit metadata, exact-country/global resolution, date boundaries, and no cross-country fallback.
- Use Playwright for admin draft authoring, citation editing, publication, learner search-to-reader navigation, scope selection, unavailable state, and narrow/desktop layout.
- Use Rust TypeScript export tests and the existing generated-file drift gate for every new client DTO.
- Run no build, unit, integration, or browser test before the final implementation slice; the user explicitly deferred execution. Final acceptance uses fresh GitHub Actions.

## Out of Scope

- Clinical approval, medical correctness, source-rights verification, private-document ingestion, media playback, automatic geolocation, user profile region management, full ISO country catalogs, and locale translation.
- Raw HTML/rich-text editing, semantic/vector search, content rollback UI, or deletion of published history.
- Deploying or merging this branch.

## Further Notes

The reader labels content as published, not clinically reviewed. Clinical reviewer assignment and evidence remain governed by TRUST-04 and the owner inputs.
