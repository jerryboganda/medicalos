# 06 — Companion files update (§32, mandatory before implementation agents start)

Status: resolved (both files written to the v2.0 stack)
Requirement IDs: §32 'Companion files'

## Task

`AGENT_IMPLEMENTATION_HANDOFF.md` and `README.md` in the project root still describe the version 1.0 stack (TypeScript backend, Capacitor) — or don't exist here. Write both to decisions 9–11: Rust backend, Tauri desktop+mobile, SvelteKit SPA, Astro site, shared Rust core as single source of truth, GitHub Actions compute rule, §31.1 guardrails, pointer to master plan + traceability ledger + phase program.

## Acceptance evidence

Both files describe the v2.0 stack with no TypeScript-backend or Capacitor-primary claims (Capacitor named only as the §20.1 mobile fallback).
