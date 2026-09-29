# Agent Experience (AX) protocol for Medical OS

Status: in-progress (docs layer done → audit script → PR)
Requirement IDs: none (process/docs infrastructure — like the existing `docs/agents/` files, this carries no master-plan §29 ID)

## Problem

Agent working memory lives in conversation context, which compacts and dies at session end. This repo already has the ground-truth layers (strict ledger, CI gates, per-effort `.scratch/` task dirs), but nothing mandates a persistent per-task state file, and nothing mechanically prevents "verified" claims without raw output evidence. Parallel agents (ZCode + Codex) share one worktree and have no written handoff discipline.

## Decision

Adopt the AX protocol (`docs/agents/agent-experience.md`):

1. Every effort maintains `.scratch/<feature-slug>/STATE.md` — externalized working memory (objective, decided, do-not-touch, completed with evidence, active diffs, verification gates, next action).
2. No verification gate may be marked PASS without raw output evidence (Actions run URL + job, or a trimmed log under `.scratch/<slug>/evidence/`).
3. The loop: PLAN → EXECUTE → VERIFY → RECORD STATE → CHECK GIT → next.
4. Mandated in `AGENTS.md` (inherited by Codex delegations) and enforced by a CI audit gate (`scripts/ax-audit.mjs`) in the same spirit as the TRUST-01 audit.

This effort runs inside a linked git worktree (`D:\Projects\medicalos-ax`, branch `zcode/ax-protocol` off `origin/main`) because `zcode/role-aware-admin` is actively being worked in the main worktree — the shared-worktree isolation rule from the protocol itself, applied to its own implementation.

## Acceptance

- Protocol doc exists at `docs/agents/agent-experience.md` with the STATE.md template, gate list mirroring `ci.yml`, loop, session rituals, multi-agent rules, ledger integration.
- `AGENTS.md` carries a mandatory AX section; issue-tracker, handoff, and README point at the protocol.
- `scripts/ax-audit.mjs` wired into `ci.yml` exits 0 against the current ledger and every existing STATE.md.
- The two loose 2026-09-28 CI failure logs are tracked under `.scratch/evidence/`; `.gitignore` evidence rule no longer dead (`_ci*.log` → `*.raw.log`).
- PR to `main` green (rust / client / site / e2e), run URL recorded in STATE.md.
