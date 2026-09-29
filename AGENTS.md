# Medical OS — Project Instructions

## Mandatory skill suites — NO DEVELOPMENT WITHOUT THEM

Every frontend, backend, or full-stack task in this repository MUST run through these three installed skill suites, regardless of task size. Skipping them because a change "looks small" is a violation, not a shortcut.

1. **Ponytail — every implementation task (frontend and backend).** Write like the laziest senior dev in the room: YAGNI, reuse before rewrite, stdlib before new dependencies, smallest change that actually works. Safety (validation, security, error handling) is never cut. Finish every diff with `ponytail-review`; use `ponytail-audit` for whole-repo cleanups and `ponytail-debt` to keep `ponytail:` shortcuts tracked.
2. **Hallmark — every frontend/UI task.** Before writing any UI (new pages, components, redesigns, styling, design extraction), load `hallmark` and use the right verb: default (build), `audit`, `redesign`, or `study`. UI written without hallmark is rejected.
3. **Matt Pocock engineering skills — the workflow.** Non-trivial features follow `to-spec` → `tdd` → `implement` → `code-review`. Debugging starts with `diagnosing-bugs`, never with guess-fixes. Architecture and domain questions use `domain-modeling` / `codebase-design`; exploratory work uses `research` / `wayfinder`.

### Where the skills live

- **ZCode & Claude Code:** `~/.agents/skills/` (global install)
- **Codex in this repo:** `.codex/skills/` (project mirror)
- If a suite is missing from your session's skill menu, read its `SKILL.md` directly from those paths and follow it. An invisible skill is not an excuse to skip it.

### Codex delegations

Delegated implementation tasks inherit this file (Codex reads `AGENTS.md`) and have the full suite in `.codex/skills/`. Keep delegation prompts pointed at this mandate. A delegation prompt must name the effort's `.scratch/<slug>/STATE.md` path and require a final STATE.md update before the completion report.

### Updating

Refresh all three suites with `npx skills update` (sources: `mattpocock/skills`, `dietrichgebert/ponytail`, `nutlope/hallmark`).

## Agent Experience (AX) protocol — MANDATORY FOR EVERY TASK

Every effort externalizes its working memory to disk: conversation context compacts and dies at session end, the STATE file persists. Full protocol: `docs/agents/agent-experience.md`.

1. **Every effort maintains `.scratch/<feature-slug>/STATE.md` from its first edit**: objective, decided constraints, do-not-touch, completed items with evidence, active diffs, verification gates, exact next action.
2. **No verification gate may be marked PASS without raw output evidence** — a GitHub Actions run URL plus the job name, or a trimmed log under `.scratch/<slug>/evidence/`. "Should pass" is not a result; NOT RUN stays visible.
3. **Work the loop**: PLAN → EXECUTE → VERIFY → RECORD STATE → CHECK GIT (`git status`/`git diff` must match the recorded diffs) → next. Before any context refresh or session end, update STATE.md first — a fresh context reconstructs from STATE.md + git, never from old conversation memory.
4. **Shared worktree**: before starting, reconcile with `git status` + recent commits — another agent's uncommitted diff is do-not-touch. Concurrent efforts isolate via linked git worktrees branched off `origin/main`.
5. **On close**: flip ledger IDs per `docs/requirements/traceability.md`'s own update rule, then close the STATE file.

Enforced in CI by `scripts/ax-audit.mjs` (evidence-less PASS/tested claims fail the build), next to the TRUST-01 audit.

## Compute workload policy — STRICT RULES

These rules are non-negotiable for every task, agent, and delegation in this project.

1. **GitHub Actions first.** Prioritize GitHub Actions for ALL compute-intensive tasks whenever technically possible.
2. **The production VPS is not a compute machine.** It must never be used as a general-purpose compute machine or overloaded with builds, processing, testing, or other heavy workloads.
3. **No heavy jobs on the VPS or local dev machines** unless there is a strict technical requirement that makes them unavoidable — and state that requirement explicitly before running anything heavy there.
4. **Move it to CI.** Builds, tests, data processing, automation, and any other heavy workload go to GitHub Actions wherever feasible.
5. **The production VPS serves the live app. Nothing else.** It must remain focused on serving the live application reliably, with minimal CPU, memory, and disk pressure.

When planning or reviewing any task that compiles, tests, processes data, or automates: the default target is GitHub Actions. Choosing the VPS or a local machine for heavy work requires an explicit technical justification, not convenience.

## Agent skills

### Issue tracker

Issues live as local markdown under `.scratch/<feature-slug>/` (repo has no remote yet). See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary; each label string equals its role name. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root, created lazily. See `docs/agents/domain.md`.

## Deployment target — Orca Desktop (local) only

The project is developed and run **locally on Orca Desktop** (WSL distro `oet-ci`, stack at `/home/ci/medicalos-local`, app at http://localhost:8081). Treat the live production server as out of scope: do not target, verify, or plan work against it unless the owner explicitly asks. "Deploy" means: push to `main` (CI builds the images), then pull and restart the Orca stack. See `docs/deployment/orca-desktop.md`.
