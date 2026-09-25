# Owner inputs and remaining external gates — 2026-09-25

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
| 6 | Finite-seat content-rights allocation | AI-04 recommendations fail closed for seat-limited grants until an allocation lifecycle exists | Define per-learner allocation, consumption, release, and concurrency semantics before enabling these grants |

Related owner-gated items beyond this table: store developer accounts
(COM-02 store route, PROT-03, OPS-03 signing), Tauri shells (ARCH-03,
PROT-01, deep links, ENG-03), SIM-03 voice adapters (model-provider
decision), DICOM/anatomy content rights (IMG-03/04), verified offline-pack
download receipts, and the accessibility capability vocabulary used by AI-04.

## Codex lane status (2026-09-25)

Issue 16 (IMG-02 stack viewer) and the remaining implementation batch are
committed on `codex/remaining-phases`. GitHub Actions run
[36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978)
passed on source commit `a885734c5b28344254a68e6881c13bdee29fce6e`, including
105 API integration tests, 19 other Rust tests, 54 Playwright E2E tests,
formatting, Clippy, dependency/license checks, WASM builds, and client/site
builds. No deployment, merge, or production verification is implied.
