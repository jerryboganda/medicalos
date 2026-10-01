# Enforce current display eligibility for the question of the day

Status: in-progress

## Problem Statement

A QOTD selected while its display grant was active can remain in the shared daily pick after that grant expires, is revoked, or stops covering the learner audience or source assets. The learner can then receive question text or attempt to answer content that is no longer eligible for display.

## Solution

Make QOTD discovery, the persisted exam/day pick, learner payloads, and answer submission respect the question version's current display eligibility. Preserve the stable shared pick and learner answer evidence, but withhold question content while the grant is unavailable.

## User Stories

1. As a learner, I want my QOTD to use a question with a current learner-display grant, so that my daily study prompt is eligible to be shown.
2. As a learner, I want an already-selected QOTD to stop returning its prompt and options as soon as its grant becomes unavailable, so that a stale daily pick does not expose question content.
3. As a learner, I want the QOTD and engagement-status endpoints to agree about availability, so that the same question is not hidden on one screen and served on another.
4. As a learner, I want answer submission to recheck current eligibility, so that a question ID cached before revocation cannot bypass the display rule.
5. As a learner, I want an active QOTD to remain the same for everyone selecting that exam on that day, so that the shared daily question remains consistent.
6. As a learner, I want an unavailable daily pick to preserve my answered state and community aggregates without returning question content, so that non-content learning evidence survives a rights change.
7. As a learner, I want no QOTD advertised when the exam has no eligible published question, so that the app does not promise a prompt it cannot serve.
8. As a learner, I want a question to become available again if its current display grant is restored, so that a temporary rights interruption does not destroy the daily selection or its history.

## Implementation Decisions

- Use the shared current display-rights rule for candidate discovery, the persisted daily pick, response payloads, and answer submission. Check the exact question version's rights reference, source reference, source references, and media references.
- Keep the existing stable exam/day selection. If the persisted question becomes ineligible, return it as unavailable; do not silently substitute a different question for the day.
- Keep the existing public API contract and current QOTD UI pattern. Unavailable responses contain no question version ID, vignette, options, or answer key. Existing answer status and content-free community results may remain visible.
- Do not add a migration or dependency for this slice. Commit any generated SQLx cache records required by the new query text.

## Testing Decisions

- Test through the learner-facing HTTP routes, not a private eligibility helper or database-only assertion. Issue 03 already requires public-route regression coverage, so the existing `/v1` and `/api/v1` QOTD aliases and engagement payload are the agreed seam.
- Verify active rights preserve the shared daily pick. Then cover revoked, expired, wrong-audience, unsupported-seat-limit, and incomplete-asset-scope grants; assert the response omits question content and the answer route refuses the stale question ID.
- Verify an answered learner retains answered state and content-free aggregates after rights loss, while the response omits the question payload.
- Use the existing deterministic per-exam/day QOTD integration coverage as prior art, extending it with the rights-state transitions.

## Out of Scope

- Competition question pools, retests, marks, notes and exports, review/tutoring cards, insights, and other issue 03 acceptance areas.
- External verification that a supplied license is authentic, consuming-LMS certification, offline-pack behavior, and deployment.
- Changes to the visual design or QOTD interaction pattern.

## Further Notes

The current daily question is a shared exam/day record; rights changes affect whether it can be served, not which question won that day's selection. Stored answers remain learner-scoped and do not restore permission to display the source question.

## Comments

- 2026-10-01 — Public-route regression coverage is being added for both API aliases, engagement status, rights-state changes, stale answers, stable picks, and preserved answered aggregates. Exact-source GitHub Actions acceptance is pending.
