# AGENT IMPLEMENTATION HANDOFF — Medical Learning OS

**Read this before writing any code.** Version 2.0 stack, supersedes any v1.0 handoff (TypeScript backend / Capacitor are withdrawn per owner decision 9; Capacitor survives only as the §20.1 mobile fallback).

## Source of truth

1. `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` — all requirements. If code contradicts the plan, the plan wins until an ADR says otherwise.
2. `docs/requirements/traceability.md` — every ticket must cite ≥1 requirement ID from this ledger.
3. `AGENTS.md` — skill-suite mandate + compute policy + AX protocol.
4. The effort's `.scratch/<slug>/STATE.md` (AX protocol, `docs/agents/agent-experience.md`) — live task state; ground truth stays git + CI + the ledger.

## Stack (do not substitute without an ADR)

- **Rust** backend/APIs/workers; shared `crates/*` compiled natively + to WebAssembly. Rust types are the contract; generated TS is never hand-edited.
- **SvelteKit + TypeScript** SPA (static mode) — one `apps/client` for web/PWA and Tauri 2 shells (Windows, macOS, iOS, Android).
- **Astro** marketing/checkout site — holds no learner data.
- **Tauri mobile is spike-gated** (Phase 0 issue 08); fallback = Capacitor mobile shell only, same UI + Rust backend.

## Non-negotiable guardrails (plan §31.1)

- Business rules (sessions, timers, scoring, scheduling, entitlements, sync) live in shared crates — never re-implemented in Svelte components.
- No `unsafe` without a recorded review; no panics in request/sync paths; typed errors mapped to the single API error format.
- SQL through compile-time-checked queries + service methods; every migration has a tested rollback.
- Tauri commands: narrow, validated, capability-scoped. The webview never receives secrets, pack keys, or unreleased answer keys.
- Shared-crate changes pass native **and** wasm32 builds before merge.
- Swift/Kotlin plugin code is owned and tested — no copied snippets.
- Don't change medical answer keys, delete tests, weaken gates, or mark tasks done to green CI. Clinical and rights decisions go to the human owner.

## Compute policy (hard)

All builds, lints, tests, screenshots, analysis, and artifacts run in **GitHub Actions** — not on the production VPS, not on local machines. Inspecting and editing files locally is fine; building/testing locally is not. No self-hosted runners on excluded machines. Device testing uses CI-built artifacts (ruling pending: issue 18). Production deployment additionally requires the runtime approval (issue 10) — no silent exception.

## Workflow per slice (plan §31)

requirement IDs → data contract → backend behavior → UI integration → permissions → failure states → tests → evidence → STATE.md upkeep → documentation. One complete vertical flow before proliferating modules. Skills per AGENTS.md: `to-spec` → `tdd` → `implement` → `code-review`; UI through hallmark; end every diff with `ponytail-review`; debugging starts with `diagnosing-bugs`.

## Reporting honesty

Completion reports state implemented / tested / partial / blocked / deliberately deferred. A mocked third-party integration is not a production integration. An attractive screen is not a working backend — verify through evidence, never inference. No verification gate is passed without raw output evidence (AX protocol: `docs/agents/agent-experience.md`).

## Current state

Production serves the connected Phase 1 slice at medicalos.polytronx.com; the ledger (`docs/requirements/traceability.md`) is the live per-ID status. The `remaining-phases` stretch runs on branch `codex/remaining-phases` (`.scratch/remaining-phases/`). Before assuming any unresolved choice, check the still-open owner decisions in `.scratch/phase-0-decisions/issues/` and the ledger's `blocked` rows — they gate billing, pricing, deep-link identifiers, and site deployment, not the core loop.
