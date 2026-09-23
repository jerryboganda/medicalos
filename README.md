# Medical Learning OS

One account. One connected learning record. One deeply personalized Study Agent. A career-long medical education platform delivered through web, mobile, and desktop.

**Canonical plan:** [`MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md`](./MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md) — the single source of truth for product and engineering requirements. Nothing ships that doesn't trace to it.

## Stack (owner decisions 9–11; see `docs/adr/`)

| Layer | Technology |
|---|---|
| Backend + APIs | Rust (Axum, Tokio, SQLx, OpenAPI, tracing/OTel) |
| Workers | Rust, queue + transactional outbox |
| Shared core | Rust crates compiled natively (server/Tauri) and to WebAssembly (web) |
| UI (all surfaces) | SvelteKit + TypeScript, static SPA — web/PWA + Tauri 2 shells |
| Marketing/checkout | Astro (`apps/site`) — no learner data |
| Desktop / Mobile | Tauri 2 (Windows, macOS / iOS, Android) — mobile gated by the Phase 0 spike |

## Repository layout

Extended lazily per §20.4:

```
apps/api                 Rust API (Axum + SQLx, migrations, integration suite)
apps/client              SvelteKit SPA (today plan, practice, coach, library, community, admin console)
apps/site                Astro marketing site (landing, exams, pricing, help, legal, .well-known)
crates/calc-engine       Shared deterministic calculations (native + wasm)
crates/domain-contracts  Shared rules, native + wasm (e.g. QB-11 option counts)
crates/competition-scoring  Shared duel/league scoring (native + wasm)
crates/scheduler         Shared plan/schedule rules (native + wasm)
crates/telemetry         Shared tracing init
packages/design-system   §7.1 design tokens shared by client and site
docs/adr                 Architecture decision records (lazy)
docs/requirements        Traceability ledger, coverage map, analytics taxonomy
docs/deployment          Production runbooks (VPS, deploy workflow)
.scratch/<feature-slug>  Per-phase specs + issue tracker (local markdown)
.github/workflows        CI + deploy (§27.1 compute rule)
tests/e2e                Playwright suites (client against the real API; site static)
```

## Hard rules

1. **Compute:** all builds, lints, tests, and artifacts run in GitHub Actions — never on the production VPS or local machines (AGENTS.md, plan §27). No self-hosted runners on excluded machines.
2. **Rust is the contract:** generated TypeScript/API clients are never hand-edited; business rules live in shared crates, never re-implemented in Svelte (§31.1).
3. **No fake anything:** no placeholder analytics, dead buttons, fabricated metrics, or numerical readiness before Phase 7 validation (TRUST-01, decision 12).
4. **Agent workflow:** every implementation task runs the project skill suites (AGENTS.md): ponytail, hallmark for UI, mattpocock `to-spec → tdd → implement → code-review`.
5. **Production VPS serves the live app only** — deployment additionally requires the runtime approval (issue 10).

## Where things live

- Requirement ledger: [`docs/requirements/traceability.md`](./docs/requirements/traceability.md) (~146 IDs, per-phase)
- Section → phase coverage: [`docs/requirements/coverage-map.md`](./docs/requirements/coverage-map.md)
- Phase program: `.scratch/phase-0-decisions/` … `.scratch/phase-7-advanced/` (created per phase)
- Issue tracker: local markdown under `.scratch/` — see [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md)
- Handoff for implementation agents: [`AGENT_IMPLEMENTATION_HANDOFF.md`](./AGENT_IMPLEMENTATION_HANDOFF.md)

## Status

Production runs at [`medicalos.polytronx.com`](https://medicalos.polytronx.com) (client + API; runbooks and deploy evidence in `docs/deployment/`). Phase 1's connected vertical slice is landed and has been validated by repeated GitHub Actions stretches (fmt, clippy, cargo-deny, integration suite, wasm32 core builds, client + site builds, Playwright E2E). The `remaining-phases` stretch (institution SSO, report SLA, session tools, submission receipts, concept mapping, tutoring, plan protection, library rights/imports/extraction, simulation evidence and appeals) is in flight on `codex/remaining-phases` — see `.scratch/remaining-phases/`. Still-open owner decisions live in `.scratch/phase-0-decisions/issues/` and the ledger's `blocked` rows.
