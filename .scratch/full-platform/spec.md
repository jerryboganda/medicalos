# Full-platform completion

Status: ready-for-agent
Requirement IDs: every ID in master plan section 29; see requirements.md
Baseline: origin/main c635e570d0fa9267a84c3b68270b16fe9be61e62

## Problem Statement

The platform has substantial automated coverage but does not yet satisfy the
master plan's production release gates. Ledger completion, implemented code,
external acceptance and deployment must be tracked independently. The owner
requests the complete platform, with no omitted feature and no deviation from
the established UI/UX.

## Solution

Continue the existing platform in independently verified vertical slices,
starting with confirmed security, data-integrity and operational blockers.
Retain every requirement in the completion inventory, including external
dependencies and unfinished tails of requirements marked tested. Do not mark
an integration complete from mocks or a clinical claim validated from software
tests. No scope reduction, deletion or unapproved visual redesign.

## User Stories

1. As a learner, I want revoking my device to revoke its authenticated access.
2. As a learner, I want my private data isolated from every other institution.
3. As an institution administrator, I want platform configuration updates to
   preserve the original institution's ownership.
4. As an operator, I want external key retrieval bounded to approved public
   endpoints, preventing internal-network requests and unbounded responses.
5. As a keyboard user, I want mobile navigation to receive and contain focus,
   dismiss with Escape, and restore focus without changing its appearance.
6. As a learner, I want Coach capability labels to describe the actual answer
   implementation regardless of whether a provider credential is configured.
7. As an operator, I want readiness to fail when the database is unavailable.
8. As an operator, I want only the exact successfully tested revision released.
9. As an operator, I want off-site copy verification to precede backup pruning.
10. As an owner, I want every approved requirement and external gate visible.
11. As a learner, I want published shared clinical material to have active
    rights and accountable review.
12. As a learner, I want usable export, privacy and deletion controls consistent
    with the approved data-retention policy.
13. As a learner, I want dependable study, library, simulation, review,
    planning, community and competition flows on every supported client.
14. As an owner, I want native, payment, voice, calendar, ingestion,
    multilingual and accreditation work retained until accepted.

## Implementation Decisions

- Existing master plan and its stable requirement IDs define full scope.
- The locked design system defines every visual and interaction choice:
  existing shared tokens, fonts, components, navigation, themes and motion.
  Accessibility changes preserve the visible structure and control names.
- Existing HTTP integration tests and Playwright journeys are the authorized
  test seams. Regression modules reuse the existing serialized database fixture.
- Keep liveness compatible; add database-dependent readiness with a bounded
  timeout, generic errors and no diagnostic data exposure.
- Cross-tenant platform collisions fail without disclosing the other owner.
  Same-tenant updates remain supported. Do not weaken security for fixtures.
- Preserve compatibility and device limits while binding sessions to registered
  devices; never allow a foreign device binding or silent session transfer.
- Clinical author approval, provider contracts, purchases, signing identities,
  accreditation and outcome validation require genuine external evidence.
  Pending inputs do not block independent engineering.
- Luna max workers use disjoint file ownership. No Antigravity/Gemini workflow.
- Builds, tests, generated contracts, screenshots and performance work run in
  GitHub Actions. The production VPS remains outside this task's runtime scope.

## Testing Decisions

- Tests assert observable permission, data-persistence, failure and recovery
  behavior through existing public seams, not private implementation details.
- Cover foreign ownership collisions, safe same-tenant updates, malicious key
  destinations, revocation denial, unrelated-device preservation and honest
  Coach metadata.
- Cover mobile focus transfer, containment, background inertness, Escape and
  focus restoration under the existing desktop/mobile navigation contracts.
- Prove readiness success against real PostgreSQL and failure after loss of
  database availability while liveness remains available.
- Verify backup remote syntax and failure-before-pruning with a controlled CI
  harness, and perform a real restore against synthetic student data in CI.
- Existing full CI gates remain mandatory. Add reviewed-source evidence and
  real-provider/device/learner acceptance before claiming production readiness.

## Out of Scope

Unrelated user branches/files, production VPS operations, invented clinical
content or credentials, fabricated provider success, and visual redesign are
outside the authorized implementation. No approved platform feature is dropped.

## Further Notes

The user authorized development and preservation of the current design. The
completion inventory is exhaustive; its states distinguish baseline evidence,
implementation pending, CI acceptance pending and external acceptance pending.
