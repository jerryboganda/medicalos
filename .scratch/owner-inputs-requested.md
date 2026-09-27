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

## Current verification (2026-09-26)

- Earlier on 2026-09-26, GitHub Actions push [run 36223707295](https://github.com/jerryboganda/medicalos/actions/runs/36223707295) and PR [run 36223709097](https://github.com/jerryboganda/medicalos/actions/runs/36223709097) passed on source commit `7dffa02d5d8d0eec5b29bf73c9d44f3e96956c84`: 112 API integration tests, 19 other Rust tests, 23 Rust-owned TypeScript export tests, and 57 Playwright E2E tests. Formatting, Clippy, dependency/license checks, SQLx cache generation, WASM, strict generated-file drift, client, and site gates passed.
- The image-case response DTOs are now generated from Rust and checked in. ARCH-02 remains in progress because other client-consumed request/response families are handwritten.
- PR run [36245924351](https://github.com/jerryboganda/medicalos/actions/runs/36245924351) on SHA `63e5cd56b4902dae3eb6c821f0b7c31a2c124e03`: 114 API integration tests, 19 other Rust tests, 81 Rust-owned TypeScript contract tests, and 66 Playwright tests passed; full Rust/client/site/E2E CI green.
- GitHub Actions CI run [36246626960](https://github.com/jerryboganda/medicalos/actions/runs/36246626960) on SHA `2bad36192807db97377b7633576700c757564201`: CI conclusion success, 114 API integration tests, 19 other Rust tests, 81 Rust-owned TypeScript contract tests. Playwright discovered 66 tests; 65 passed and one test was flaky (first attempt failed because sentReport was undefined immediately after click; retry passed): `tests/e2e/community-reporting.spec.ts:73`, members can report a post and see only their own report status (not described as 66 passed).
- PR #11 was merged by repository owner jerryboganda at 2026-09-26T13:52:50Z; it is no longer open/draft. Current repo HEAD includes a docs-only follow-up commit `62914e85959765b929cf334e51658bda2c682b27`; app code deployed/tested remains `2bad36192807db97377b7633576700c757564201`. Repository visibility is PRIVATE.
- Deploy workflow [36246630943](https://github.com/jerryboganda/medicalos/actions/runs/36246630943) succeeded for SHA `2bad36192807db97377b7633576700c757564201`, including production-verify. The deployment workflow passed its required nonblank 32-byte pack-signing-key check. Fresh live probes: `/api/healthz` HTTP 200 ok; `/api/version.json` HTTP 200 reports exact SHA `2bad36192807db97377b7633576700c757564201`, deployed_at `2026-09-26T13:53:08Z`.
- The owner decisions above remain unanswered. The requirement ledger counts remain 38 non-complete: 17 not-started, 19 in-progress, 2 blocked; 22 rows still use narrative status wording outside the standard status vocabulary, with row-specific details and any remaining gates.
