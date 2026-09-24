# Owner inputs requested — 2026-09-24

Presented via structured questions this session; unanswered. Safe defaults
below are already reflected in the shipped code, and execution per decision
is pre-scoped so each answer converts directly into a slice.

| # | Decision | Default while unanswered (shipped) | Unlocks when answered |
|---|---|---|---|
| 1 | Site deployment origin | Site stays built + CI-gated, unserved | Deploy wiring for `apps/site` to the chosen origin (subdomain recommended — existing VPS nginx) |
| 2 | 7C pricing numbers | Pricing page shows "to be announced" (honest, shipped) | Real prices on the pricing page + entitlement wiring |
| 3 | Payment route / provider for web checkout | No checkout anywhere (COM-02 stays blocked; §26.1 per-market rule) | COM-02 build: checkout API + UI + entitlement service link |
| 4 | Clinical reviewers | TRUST-04 / CAREER-02 stay blocked | Review-assignment workflow build (TRUST-04, CAREER-02) |
| 5 | Reference test devices | UX-03 / TRUST-06 stay blocked | Device-budget + accessibility matrix (UX-03, TRUST-06) |

Related owner-gated items beyond these five: store developer accounts
(COM-02 store route, PROT-03, OPS-03 signing), Tauri shells (ARCH-03,
PROT-01, deep links, ENG-03), SIM-03 voice adapters (model-provider
decision), DICOM/anatomy content rights (IMG-03/04).

## Codex lane status at close

Issue 16 (img02 stack viewer) is drafted but has **no implementation** —
no commits, no working-tree edits. Nothing to validate. When its
implementation lands and settles: run the full public-CI → merge main →
deploy → production-verify → private loop on the merged tip, then re-run
CI, and take the next disjoint slice from the ledger.
