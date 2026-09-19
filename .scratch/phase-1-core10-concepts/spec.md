# CORE-10 — Versioned concept identities mapped to curriculum taxonomy

## Problem Statement

Medical OS currently locates published questions through the Exam -> Subject -> System -> Chapter taxonomy, and learner evidence is still keyed by chapter. The master plan requires that navigation remain a taxonomy rather than becoming the learning source of truth: questions, lessons, media, flashcards, OSCE criteria, and blueprint objectives must ultimately connect to the same stable, versioned concept identities.

Without a separate concept identity, changing curriculum placement would also change the implied learning identity, and future cross-modality evidence would have no stable shared anchor.

## Solution

Introduce stable concept identities with immutable numbered concept versions. Map curriculum nodes to concept versions and map published question versions directly to concept versions with an explicit primary or secondary relationship.

For the Phase 1 foundation, seed one published concept version for each existing synthetic chapter and map every existing synthetic question version to the corresponding concept version as its primary concept. Existing chapter-based practice, planning, and learner-state behavior remains unchanged until a later behavioral migration moves learning evidence onto concept identities.

## User Stories

1. As a learner, I want the learning identity behind a topic to remain stable when navigation changes so that my evidence can remain meaningful across curriculum reorganizations.
2. As an editor, I want a curriculum chapter to map to a concept version instead of redefining the concept so that taxonomy and learning meaning remain separate.
3. As an editor, I want a concept identity to survive multiple numbered definitions so that published meaning can evolve without mutating history.
4. As an editor, I want published concept definitions to be immutable so that existing content and evidence always retain their original semantic anchor.
5. As an editor, I want a curriculum node to map to more than one concept version when needed so that navigation does not force one-to-one learning semantics.
6. As an editor, I want a concept version to appear in more than one curriculum placement when needed so that cross-cutting concepts are not duplicated.
7. As a question author, I want each published question version to identify its primary concept version so that its principal learning target is explicit.
8. As a question author, I want a question version to support secondary concept versions so that important supporting concepts can be represented without redefining the primary target.
9. As a question author, I want at most one primary concept per question version so that the principal learning target is unambiguous.
10. As a platform maintainer, I want concept relationships enforced by database constraints so that integrity does not depend on duplicated application checks.
11. As a platform maintainer, I want current chapter-based practice and session behavior preserved in this foundation so that CORE-10 does not silently rewrite unrelated learner flows.
12. As a platform maintainer, I want the synthetic seed to expose concept mappings immediately so that later modules have a deterministic fixture to build on.
13. As a platform maintainer, I want existing question-version immutability preserved so that adding concept mappings does not alter published question content.
14. As a platform maintainer, I want concept versions to use explicit positive version numbers so that version order is deterministic.
15. As a platform maintainer, I want only real modeled modalities mapped now so that the schema does not invent lesson, video, flashcard, image, OSCE, or blueprint records before those modules exist.
16. As a platform maintainer, I want no new public curriculum endpoint merely to reveal internal identifiers so that the product interface stays demand-driven.
17. As a platform maintainer, I want learner-state migration tracked separately so that a schema foundation is not misreported as complete concept-based learning evidence.
18. As a platform maintainer, I want heavy database acceptance to run in GitHub Actions so that local development and production infrastructure remain protected from compute load.

## Implementation Decisions

- A concept identity is a stable learning entity and is separate from every curriculum placement.
- A concept version belongs to exactly one concept identity, has a positive integer version, and represents an immutable published definition.
- Curriculum nodes map to concept versions through a many-to-many relation.
- Question versions map directly to concept versions through a many-to-many relation carrying `primary` or `secondary` semantics.
- Database constraints enforce valid relationship values and at most one primary concept per question version.
- The synthetic seed creates one published concept version for each existing synthetic chapter.
- The four existing synthetic question versions map to the concept version associated with their current chapter as their primary concept.
- Existing `chapter_id` fields and chapter-based practice, plan, and learner-state behavior remain intact in this foundation slice.
- No new provider, repository, service, interface, dependency, or public curriculum endpoint is introduced for this schema-only foundation.

## Testing Decisions

- The agreed seam is the existing seeded PostgreSQL integration contract because CORE-10 currently has no justified public curriculum interface.
- The red contract asserts that every seeded chapter maps to a distinct published version-1 concept and that every seeded question version has exactly one matching primary concept mapping.
- Database constraints provide the minimal integrity implementation for valid relation values, positive concept versions, uniqueness, and one-primary-per-question-version semantics.
- Existing integration coverage continues to prove that chapter-based learner flows remain operational once the mapping foundation is present.
- Heavy compilation, PostgreSQL integration execution, builds, and browser testing run only in GitHub Actions. Local checks are limited to formatting, static inspection, and diff validation.

## Out of Scope

- Migrating learner concept evidence from `chapter_id` to concept identities or concept versions.
- Mapping lessons, video timestamps, images, flashcards, OSCE criteria, blueprint objectives, or other modalities that are not yet modeled.
- Curriculum browsing or editorial-management endpoints and UI.
- Multi-parent curriculum navigation and blueprint-management workflows.
- Replacing existing chapter-based practice/session selection in this slice.
- Deployment.

## Further Notes

- This foundation advances CORE-10 but does not complete it. CORE-10 remains partial until learner evidence and additional modeled modalities use concept identities rather than chapter placement as the learning anchor.
