# 04 — CI runner matrix (§27.1)

Status: resolved for Phase 0 scope — core gates green in run 35273824336. Later-phase gates (Postgres integration/SQLx, contract drift, E2E, signing, perf budgets) are added by the phase that first has code for them.
Requirement IDs: OPS-01, OPS-07

## Task

`.github/workflows/ci.yml` with the gates that have code to exercise now:

- Linux: fmt check, clippy `-D warnings`, cargo-deny (advisories + licenses), workspace tests, `cargo build -p domain-contracts --target wasm32-unknown-unknown`, Rust compile caching (mandatory per §27.1)
- Node: apps/client build, apps/site build

Gates added by later phases (not stubbed now): ephemeral-PostgreSQL integration tests + SQLx compile checks + contract-drift check (P1), browser E2E (P1 web), desktop E2E + macOS/Windows signing jobs (P1 betas / P3 store), iOS/Android build + device-farm distribution (P1 beta / P3), performance-budget job from the §30.2 table (P3), model-evaluation suite (P2).

Enforcement: no builds/tests outside GitHub Actions (AGENTS.md compute policy). No self-hosted runners on the excluded machines.

## Acceptance evidence

Workflow file exists; first green run recorded once the GitHub remote exists.
