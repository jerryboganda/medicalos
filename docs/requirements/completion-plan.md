# Medical OS — Completion Plan

Snapshot date: **2026-09-27** · Scope: the 152-ID requirement ledger in
[`traceability.md`](traceability.md) (source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §29).
This document is the working roadmap for the remaining 25%: what is open, in what
order it closes, and what only the owner can unblock. Update the tables below when
a batch lands; do not duplicate ledger rows here.

---

## 1. Where the project stands

The platform is live and production-verified. PR #11 merged the entire
"remaining-phases" stretch into `main` on 2026-09-26 (remote `main` =
`62914e8`), and deploy run 36246630943 production-verified
medicalos.polytronx.com serving `2bad361`.

Last fully green CI run (36245924351 on `63e5cd5`):

- 114 API integration tests + 19 other Rust tests (real Postgres)
- 81 Rust-owned TypeScript contract tests (ts-rs, drift-checked in CI)
- 66 Playwright E2E tests · TRUST-01 "no fake anything" audit gate
- fmt / clippy `-D warnings` / cargo-deny / wasm32 core builds / client + site builds

Stack: Rust/Axum + SQLx (Postgres 17, 56 migrations) · SvelteKit 5 static SPA
(25 pages) · Astro site (12 pages, built, **not yet served** — owner decision) ·
5 shared crates compiled native + wasm32 · GitHub-Actions-only compute with
SHA-verified deploys. Zero TODO/FIXME markers in any source file; incompleteness
is tracked in the ledger and `.scratch/`, never hidden in code.

## 2. Quantified completion (verified row-by-row, 2026-09-27)

| Bucket | IDs | % of plan |
|---|---|---|
| `tested` (acceptance evidence filed) | 94 | 61.8% |
| CI-verified (narrative wording; named external tails) | 18 | 11.8% |
| Resolved (OPS-02) / previously CI-tested, needs fresh run (OPS-04) | 2 | 1.3% |
| **Done or verified-equivalent** | **114** | **75.0%** |
| In-progress | 19 | 12.5% |
| Not-started | 17 | 11.2% |
| Blocked | 2 | 1.3% |
| **Remaining** | **38** | **25.0%** |

Honest framing: **engineering-ready work is ~75–90% done**; the remaining 38 IDs
split into **25 engineering-ready** (startable once CI billing is fixed) and
**13 owner-gated** (payment provider, reviewers, devices, store accounts,
content rights, Tauri shells, one external calibration record). Roughly half of
what remains cannot be coded today even in principle — it waits on owner
decisions and third parties, not engineering.

## 3. The 38 open IDs, classified

### 3a. Engineering-ready (25) — startable when CI is unblocked

| ID | Requirement tail | Notes |
|---|---|---|
| OPS-01 | Actions compute-enforcement finalization | essentially done, needs closure evidence |
| ARCH-01 | wasm runtime-parity spike | 3+ crates already proven native+wasm |
| CORE-04 | dedicated tenant partitioning | isolation tests already proven |
| AI-05 | event/queue workers | deterministic handlers exist today |
| AI-14 | multi-tenant deployments | user-scoping done; deploy story pending |
| ADMIN-02 | real license verification | hooks engineering; verification itself external |
| UX-01 | native-app gesture polish | web gestures shipped; native part ties to Batch 7 |
| ENG-01 | QOTD reminder scheduling + remote delivery | core QOTD tested + deployed |
| ARCH-02 | TS contract generation sweep | in flight on `codex/sr08-retest-enrollment` |
| EX-08 | native device-clock resistance, competition policy | native part ties to Batch 7 |
| SR-08 | retest SLA timers | in flight on `codex/sr08-retest-enrollment` |
| LIB-06 | binary extraction; real license verification | verification external |
| LIB-07 | trusted parsers, malware scanning, OCR sandbox | engineering |
| LIB-08 | licensed-asset verification | player itself tested (run 36245924351) |
| OFF-01 | production signing secret, device assurance, download receipts | key may already be provisioned — confirm |
| OFF-02 | receipt scope | CI-validated core; scope tail |
| PROT-02 | device attestation, anti-scraping, pack encryption | native parts tie to Batch 7 |
| INST-06 | LTI 1.3 launch + certification | QTI 2.1 export shipped |
| PLAN-05 | calendar read/write scopes | engineering |
| IMG-05 | low-device-capability fallbacks | engineering |
| AI-15 | evaluated multilingual tutoring | needs evaluation harness |
| COM-03 | coupons, referrals, regional price tiers | engineering |
| ADMIN-05 | revenue, licenses, royalties, renewals | engineering |
| OPS-03 | signed releases, reversible migrations | rollback CI in flight; signing ties to Batch 7 |
| CAREER-03 | accreditation/provider workflow | CE records tested; provider gate Phase 7 |

### 3b. Owner-gated (13) — blocked on decisions, accounts, or third parties

| ID | Gate (owner action) |
|---|---|
| COM-02 | Payment route/provider + store developer accounts |
| COM-04 | Commercial decision (§32/§26.1) — `blocked` |
| TRUST-04 | Named clinical reviewers |
| CAREER-02 | Named clinical reviewers |
| UX-03 | Reference test devices |
| TRUST-06 | Reference test devices |
| PROT-03 | Store developer accounts |
| ENG-03 | Tauri shells (mobile/desktop) |
| ARCH-03 | Tauri shells |
| LIB-09 | Licensed offline media content rights |
| IMG-03 | DICOM/anatomy content rights (privacy workflow is engineering) |
| QB-10 | Externally validated calibration record |
| GROW-02 (deploy tail) | Site deployment origin — ID itself `tested`, site unserved |

## 4. Batch program

Rules for every batch: mattpocock workflow (`to-spec` → `tdd` → `implement` →
`code-review`); `ponytail-review` on every diff; `hallmark` for any UI; all
compute in GitHub Actions (VPS serves, never computes); batch CI/CD at end of
stretch; ledger row set to `tested` with evidence run links; production-verify
after each merge to `main`.

| Batch | Scope | Entry criteria | Exit criteria |
|---|---|---|---|
| **B1 — Land in-flight stretch** | PR `codex/sr08-retest-enrollment` (23 commits: SR-08, EX-07, OPS-03 rollback CI, ARCH-02 sweep, PROT-01 watermark); merge draft PR #12 | PUBLIC flip per §5.1 protocol | Green CI → merge → deploy → verify → PRIVATE flip → ledger rows updated (expect 2–4 IDs → `tested`) |
| **B2 — Acceptance tails** | LIB-08 final acceptance; OFF-01 signing secret/device assurance/receipts; OFF-02 receipt scope; OPS-04 fresh recovery-drill CI; OPS-01 closure; canonicalize 18 narrative ledger rows into `tested` vocabulary with run links | B1 merged | All targeted rows `tested`; zero narrative-status rows left |
| **B3 — Platform depth** | CORE-04, AI-05, AI-14, ADMIN-02, INST-06, ARCH-01; optional (tracked debt): split the 21,657-line `integration.rs` | B2 done | Rows `tested`; institution platform depth proven |
| **B4 — Learner polish** | UX-01 (web part), ENG-01, EX-08 (web part), PLAN-05, IMG-05, AI-15 | B3 done | Rows `tested` |
| **B5 — Library/media depth** | LIB-06, LIB-07, LIB-09 (subject to rights), LIB-08 asset verification | B2+ | Rows `tested` or external tail named |
| **B6 — Commerce + institutional** | COM-02, COM-03, ADMIN-05, TRUST-04, CAREER-02, UX-03, TRUST-06, GROW-02 deploy, AI-04 seat allocation, COM-04, IMG-03 | Fires as each owner decision lands; subsets run independently | Rows `tested`; honest defaults retired |
| **B7 — Native shells** | Tauri desktop + mobile: ARCH-03, ENG-03, PROT-01 completion, PROT-02, PROT-03, OPS-03 signed releases, GROW-01 deep links | Store developer accounts provisioned | Store submissions ready or shipped |
| **B8 — Close-out** | QB-10, CAREER-03 provider workflow, ledger reconciliation to 152/152, §30 release-gate audit, final production verification | All above | Ledger fully `tested`/`deferred` with owner sign-off |

Progress log: **2026-09-27 — B1 landed** (PR #13 + PR #12 merged;
runs 36353096430/36354446229 green; production verified `82d5d05` -> `12b54a6`).
EX-07 -> tested; OPS-03/PROT-01/SR-08/ARCH-02 advanced with named tails.

**2026-09-28 — B2 ledger pass**: 18 narrative "CI-verified" rows
canonicalized to the standard `tested` vocabulary with run links;
LIB-08, OFF-02, OPS-04, OPS-01 flipped to `tested`. OFF-01 receipt slice
in review (PR #14); signing secret confirmed provisioned.

**2026-09-28 — B2 ✅ landed**: PR #14 (OFF-01 verified download
receipts, migration 0058) green and production-verified `26565fa`;
LIB-08, OFF-02, OPS-04, OPS-01 -> `tested`; 18 narrative rows
canonicalized; OFF-01 -> `tested` with attestation (PROT-02) and licensed
media (LIB-09) named as the remaining external boundaries.

**2026-09-28 — B3 partial**: ARCH-01 -> tested (wasm runtime
parity spike: calc-engine cdylib + node-executed shared parity vectors,
PR #15) and ARCH-02 -> tested (issues 42/43 closed: numeric wire types,
scenario request contracts verified already landed; 14 stale caveat
clauses stripped across rows). Production verified `fef8486`.
Remaining B3: CORE-04, AI-05, AI-14, INST-06 — larger slices queued
for the next stretch.

**2026-09-28 — B3 CORE-04 landed**: PR #16 (row-level security
tenant isolation, migration 0059) green and production-verified
`0c2a52b` after a production incident: the first deploy crash-looped the
API because the least-privilege production role cannot CREATE ROLE —
fixed by guarding the confined-role creation behind a CREATEROLE check
(RLS arms regardless) and extending the deploy health wait for
migration-heavy startups (Codex). Remaining B3: AI-05, AI-14, INST-06.

Pace reference: the last stretch converted ~45 IDs in 3 days of agent stretches.
At that pace the 25 engineering-ready IDs ≈ 1.5–3 weeks of batched agent work
once billing is fixed.

## 5. Owner action items (hard gates)

1. **Restore Actions capacity.** All CI runs since 2026-09-26 ~14:23Z
   fail-to-start with the payments/spending-limit annotation. This repo has hit
   this before: the signature matches the private-repo Actions billing limit,
   and the standing, owner-mandated protocol fixes it — flip the repo PUBLIC
   (free Actions) before the next CI/deploy stretch, verify production, then
   flip back PRIVATE. Only if a public-era run also fails to start is a real
   Billing & plans payment/limit fix required. Highest-leverage unblock either
   way; nothing else can proceed without it.
2. **Answer the six decisions** in `.scratch/owner-inputs-requested.md`:
   site origin · 7C pricing · payment provider · clinical reviewers ·
   reference devices · seat-allocation semantics. Safe defaults are shipped;
   each answer converts directly into a pre-scoped slice.
3. **As available:** store developer accounts · SIM-03 voice model provider ·
   DICOM/anatomy content rights · external calibration validation record (QB-10).

## 6. Risk register

| # | Risk | Impact | Mitigation |
|---|---|---|---|
| 1 | CI billing outage | All validation frozen | Owner action (§5.1); B1 queued and ready |
| 2 | Owner decisions unanswered | 13 IDs gated | Safe defaults live; pre-scoped slices ready per decision |
| 3 | Third-party dependencies (store review, calibration record, license verification) | Timeline outside repo control | Named tails kept explicit in ledger; fail-closed behavior shipped |
| 4 | 21,657-line single integration test file | Compile-time scaling | Optional split in B3, tracked as `ponytail:` debt |
| 5 | Narrative ledger statuses | Evidence hygiene | B2 canonicalization |
| 6 | Concurrent Codex work on shared branches | Clobbered edits | Worktrees + `find -mmin -60` check before touching shared files |

## 7. Working rules (unchanged, mandatory)

Ponytail + hallmark + mattpocock suites on every task (repo `AGENTS.md`);
GitHub-Actions-only compute (production VPS serves the live app, nothing else);
Rust types are the contract; no fake data (TRUST-01); ledger update rule — when
a ticket completes, set its ID to `tested` and link the evidence.
