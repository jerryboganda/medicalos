# LIB-01 — Versioned articles and references

## Problem Statement

The Learn destination currently contains only spaced review. Phase 1 also requires a connected knowledge-library foundation where reviewed articles and reference objects have stable identities, immutable published versions, and explicit links to the same concept/question model used by practice. Without that foundation, later search, citations, notebooks, and source-grounded tutoring would have no trustworthy content source of record.

## Solution

Add the smallest connected-library slice: stable library item identities whose published versions are immutable and numbered. Phase 1 supports two object kinds, `article` and `reference`, because those are the LIB-01 requirement. Each published version carries its title, body, provenance class, source label, optional source URL, effective date, and jurisdiction, and can link to concept versions and question versions.

Expose authenticated read APIs for the latest published version of every library item and for an explicit numbered version of one item. Extend the existing Learn destination with a compact Library section that lists the latest published items and opens their current content without creating another navigation system.

## User Stories

1. As a learner, I want reviewed articles in Learn, so that I can study beyond question explanations.
2. As a learner, I want reference objects alongside articles, so that concise source material has a first-class home.
3. As a learner, I want only published library versions, so that drafts are never served accidentally.
4. As a learner, I want the latest published version listed by default, so that I normally see current material.
5. As a learner, I want an explicit historical version to remain retrievable, so that citations can resolve to the content version they originally referenced.
6. As a learner, I want title, effective date, jurisdiction, and source metadata visible, so that time- and jurisdiction-sensitive medical content is not presented without context.
7. As a learner, I want article/reference provenance identified, so that later search and tutoring can distinguish editorial material from other source classes.
8. As a learner, I want library items connected to concept versions, so that navigation placement does not become the learning identity.
9. As a learner, I want relevant question versions linked to a library version, so that questions and reading can share a trustworthy content graph.
10. As a content owner, I want published versions immutable, so that an old citation cannot silently change meaning.
11. As a content owner, I want version numbers unique within one library item, so that references are deterministic.
12. As a platform owner, I want the library to reuse existing authentication, concepts, questions, Learn UI, and database patterns, so that Phase 1 does not create a second content platform.
13. As a platform owner, I want later semantic search, document ingestion, private uploads, media, and notebooks kept out of this slice, so that LIB-01 remains a small trustworthy foundation.

## Implementation Decisions

- Introduce one stable library-item identity table and one immutable numbered-version table.
- Restrict Phase 1 item kinds to `article` and `reference`.
- Restrict served versions to `published`; published rows are treated as append-only by the application contract.
- Store provenance class, source label, optional source URL, effective date, and jurisdiction on each version because those attributes can change between versions.
- Link library versions directly to existing immutable concept versions and question versions.
- Use one authenticated library route module with a latest-list endpoint and an explicit numbered-version detail endpoint.
- The list endpoint returns latest published metadata only; full body and graph links are returned by the detail endpoint.
- Seed only synthetic reviewed library content already aligned to the repository's fictional concepts/questions; do not introduce copyrighted or external medical text.
- Extend the existing Learn destination rather than adding a sixth primary learner destination.
- Add no dependency and no search/indexing abstraction; exact/semantic retrieval belongs to LIB-02.

## Testing Decisions

- The primary seam is the existing HTTP integration-test boundary.
- The contract test proves authentication, latest-version selection, article/reference kinds, historical-version retrieval, metadata, concept links, question links, and 404 behavior for an unknown version.
- Seeded content is synthetic and deterministic so expected version/title/link values come from known fixtures rather than recomputing implementation logic.
- Frontend behavior reuses the existing Learn loading/error patterns; heavy browser acceptance remains in GitHub Actions.

## Out of Scope

- Semantic or full-text search, entitlement filtering, private document visibility, uploads/ingestion, OCR, ebooks, procedural guides, algorithms, glossaries, tables, annotated figures, media, notebooks, AI-generated notes, source-grounded tutoring, editorial authoring UI, and deployment.

## Further Notes

- This is the §12.1 Phase-1 base only. LIB-02 through LIB-08 and NOTE-01 through NOTE-03 remain separate requirements.
- Deployment remains skipped per owner instruction.
