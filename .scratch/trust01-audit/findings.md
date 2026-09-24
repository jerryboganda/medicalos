# TRUST-01 audit findings — 2026-09-24

Manual sweep over every shipped route and page (`apps/client/src` — 17 route
files; `apps/site/src` — 12 pages), plus the static enforcement now running
in CI (`scripts/trust01-audit.mjs`).

## Sweep results

| Violation class | Method | Findings |
|---|---|---|
| Fabricated values (`Math.random`, hard-coded percent text, invented counts) | static scan + read of every data binding | **None.** The only client SVG is the share-card download, built from real card data; progress/analytics render server-derived stat rows only. |
| Dead controls (no handler / no target) | per-button handler verification (8 multi-line candidates checked) | **None.** All buttons carry handlers or are form submits; all hrefs resolve to real routes. |
| Active-agent claims | copy sweep (`thinking`, `is typing`, `agent is working`) | **One reviewed-and-accepted:** the Coach button's `Thinking…` label renders only while a real Coach-turn request is in flight (`busy` guard) — a truthful loading state, not a claimed autonomous process. |
| Placeholder copy (lorem, coming soon, sample data, TODO/FIXME) | static scan | **Two fixed:** TODO comments in `apps/site/src/lib/site.js` (never rendered, but rephrased declaratively to keep shipped sources clean). The deep-link files themselves still claim nothing until the first signed Tauri beta. |

## Enforcement

`scripts/trust01-audit.mjs` runs in CI (client job) and fails the build on
the mechanically-detectable classes (Math.random, placeholder copy,
TODO/FIXME markers) across both shipped surfaces. Dead controls and
active-agent claims need behavioral context and are covered by the
Playwright suite, which drives every control against the real API.

## Related honesty guards already in place

- Site e2e: no fabricated outcome claims, no payment forms, prices marked
  "to be announced" (`tests/e2e/site-pages.spec.ts`).
- Integration: percentile hidden below 20 takers; community stats gated by
  the minimum sample; share cards refuse without a real sample
  (`tests/e2e`, `apps/api/tests/integration.rs`).
- Session UI: honest empty/error/loading states verified (CORE-06, slice 2).
