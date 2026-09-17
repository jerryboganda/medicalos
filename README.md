# Medical Learning OS

One account. One connected learning record. One deeply personalized Study Agent. A career-long medical education platform delivered through web, mobile, and desktop.

**Canonical plan:** [`MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md`](./MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md) — the single source of truth for product and engineering requirements. Nothing ships that doesn't trace to it.

## Stack (owner decisions 9–11, ADRs 0002/0003)

| Layer | Technology |
|---|---|
| Backend + APIs | Rust (Axum, Tokio, SQLx, OpenAPI, tracing/OTel) |
| Workers | Rust, queue + transactional outbox |
| Shared core | Rust crates compiled natively (server/Tauri) and to WebAssembly (web) |
| UI (all surfaces) | SvelteKit + TypeScript, static SPA — web/PWA + Tauri 2 shells |
| Marketing/checkout | Astro (`apps/site`) — no learner data |
| Desktop / Mobile | Tauri 2 (Windows, macOS / iOS, Android) — mobile gated by the Phase 0 spike |

## Repository layout

Created so far (greenfield scaffold, extended lazily per §20.4):

```
apps/api                 Rust API (skeleton: /healthz)
apps/client              SvelteKit SPA (skeleton)
apps/site                Astro site (skeleton)
crates/domain-contracts  Shared rules, native + wasm (first rule: QB-11 option counts)
crates/telemetry         Shared tracing init
packages/design-system   §7.1 design tokens
docs/architecture-decisions   ADRs
docs/requirements        Traceability ledger, coverage map, analytics taxonomy
.scratch/phase-0-decisions  Phase 0 spec + tickets (issue tracker)
.github/workflows        CI (§27.1 compute rule)
```

## Hard rules

1. **Compute:** all builds, lints, tests, and artifacts run in GitHub Actions — never on the production VPS or local machines (AGENTS.md, plan §27). No self-hosted runners on excluded machines.
2. **Rust is the contract:** generated TypeScript/API clients are never hand-edited; business rules live in shared crates, never re-implemented in Svelte (§31.1).
3. **No fake anything:** no placeholder analytics, dead buttons, fabricated metrics, or numerical readiness before Phase 7 validation (TRUST-01, decision 12).
4. **Agent workflow:** every implementation task runs the project skill suites (AGENTS.md): ponytail, hallmark for UI, mattpocock `to-spec → tdd → implement → code-review`.
5. **Production VPS serves the live app only** — deployment additionally requires the runtime approval (issue 10).

## Where things live

- Requirement ledger: [`docs/requirements/traceability.md`](./docs/requirements/traceability.md) (~128 IDs, per-phase)
- Section → phase coverage: [`docs/requirements/coverage-map.md`](./docs/requirements/coverage-map.md)
- Phase program: `.scratch/phase-0-decisions/` … `.scratch/phase-7-advanced/` (created per phase)
- Issue tracker: local markdown under `.scratch/` — see [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md)
- Handoff for implementation agents: [`AGENT_IMPLEMENTATION_HANDOFF.md`](./AGENT_IMPLEMENTATION_HANDOFF.md)

## Status

Phase 0 (decisions + evidence) in progress — see `.scratch/phase-0-decisions/spec.md`. Owner decisions pending: pilot exam pack, runtime, budget, brand, store accounts, UI reference asset (issues 09–19).
