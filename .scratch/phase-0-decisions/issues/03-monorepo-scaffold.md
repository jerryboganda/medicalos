# 03 — Monorepo scaffold (§20.4)

Status: implemented — CI verification pending first run (no GitHub remote yet; compute rule bars local builds)
Requirement IDs: ARCH-01 (native + WASM pipeline proof)

## Task

Create the §20.4 tree lazily — only what Phase 0/1 exercises:

- Root `Cargo.toml` workspace; `crates/domain-contracts` (first real rule: 2–10 options, QB-11) + `crates/telemetry` (tracing init)
- `apps/api`: minimal Axum service with `/healthz`
- `apps/client`: minimal SvelteKit SPA skeleton (adapter-static, SPA fallback)
- `apps/site`: minimal Astro skeleton
- `packages/design-system`: §7.1 tokens (issue 05)
- `.gitignore`, `rust-toolchain.toml` (stable)

Deferred (created when first needed, per §20.4 'no empty packages'): apps/worker, crates/{exam-engine, learner-model, planner, scheduler, sync, policy, calc-engine, content-pipeline, agent-tools, core-wasm, tauri-plugins}, packages/api-client (generated, ARCH-02), content/, tests/, .github mobile/desktop jobs.

## Acceptance evidence

CI job builds the workspace natively (Linux) and `crates/domain-contracts` for `wasm32-unknown-unknown`. First run happens once the GitHub remote exists (issue 15 accounts for store/org setup; repo creation is part of it).
