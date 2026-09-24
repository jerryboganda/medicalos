# TRUST-01 — honest-UI audit sweep

Requirements: TRUST-01 (no fake scores, charts, citations, active-agent
states), CORE-06 (truthful loading/error/empty states)
Source: master plan §2.3, §28 ("this is not permission to show nonfunctional
buttons as finished features"); ledger row TRUST-01.

## Audit scope

Every shipped surface: `apps/client/src` (all routes) and `apps/site/src`.

Violation classes:

1. **Fabricated values** — numbers, percentages, or counts rendered in the
   UI that are not derived from fetched server data (`Math.random`,
   hard-coded percent text in markup, invented "N learners"-style copy).
2. **Dead controls** — buttons/links with no handler and no target, or
   controls advertising unbuilt features as finished.
3. **Active-agent claims** — copy implying the agent is "thinking",
   "working", or "typing" when no such process exists, or claiming an
   action happened without a server receipt.
4. **Placeholder copy** — lorem, "coming soon", "sample data", TODO/FIXME
   markers reachable in shipped UI.

## Method

1. Static sweep (this pass): pattern greps over shipped sources plus a
   manual read of every page's data bindings.
2. Durable enforcement: `scripts/trust01-audit.mjs` encodes the pattern
   checks and runs as a CI step, so regressions fail the build. Findings
   that are legitimate (e.g. an input `placeholder` attribute) are covered
   by the pattern design, not by ignores.
3. Fixes for every real finding, validated through the public-CI → merge
   main → deploy → production-verify → private loop.

## Acceptance

- The audit script is green on the shipped sources and wired into ci.yml.
- Every real finding from the manual sweep is fixed or converted into an
  honest state (loading/error/empty/unavailable with a reason).
- TRUST-01 ledger row updated to tested with the enforcement evidence.
