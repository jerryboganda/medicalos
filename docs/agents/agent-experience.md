# Agent Experience (AX) protocol

Mandatory for **every task** in this repo — ZCode sessions, Codex delegations, and any future agent. Skipping it because a change "looks small" is a violation, not a shortcut.

AX = externalized working memory. An agent's conversation context compacts and dies at session end; important state lives on disk instead, so any agent (or a fresh context of the same agent) can reconstruct the effort in seconds and continue it without archaeology.

## The three layers

| Layer | File | Lifetime | Holds |
|---|---|---|---|
| Permanent project rules | `AGENTS.md` (+ `AGENT_IMPLEMENTATION_HANDOFF.md`) | Months/years | Skill mandates, compute policy, guardrails, reporting honesty |
| **Current task state** | `.scratch/<feature-slug>/STATE.md` | Hours/days | Objective, decisions, diffs, gates, next action |
| Ground truth | git + GitHub Actions + `docs/requirements/traceability.md` | Always | What actually changed, what actually passed, what actually shipped |

The STATE file never overrides ground truth. When they disagree, git and CI win and the STATE file gets corrected.

## STATE.md — required for every effort

Every effort creates `.scratch/<feature-slug>/STATE.md` at its **first edit** — uniform rule, no size threshold. The effort slug matches the existing issue-tracker convention (`.scratch/<slug>/spec.md`, `issues/NN-<slug>.md`); STATE.md is the effort's live working memory, spec and issues stay the what/why.

Canonical template (all sections required — `scripts/ax-audit.mjs` checks the headers and gate evidence):

```markdown
# STATE — <feature-slug>

Updated: <ISO date + time>
Requirement IDs: <ledger IDs, or "none (reason)">   <!-- reference only; status lives in the ledger -->
Branch: `<branch>` (worktree `<path>` if not the main worktree)

## Objective
One paragraph: what done looks like.

## Status
One line: where the effort is right now.

## Decided
- <date>: <decision made during this effort> — binding until the owner overrides it.

## Do-not-touch
- <files/subsystems/branches deliberately left alone, with why>

## Completed
- [x] <item> — evidence: <run URL / evidence file / commit sha>

## Active diffs
- <files/areas currently modified, uncommitted or on branch>

## Verification gates
| Gate | Runs where | Status | Raw-output evidence |
|---|---|---|---|
| <gate> | Actions <job> / local | PASS · FAIL · NOT RUN | <run URL + job, or evidence/ file> |

## Blockers / owner inputs
<or "None.">

## Next action
The exact next executable step (command or file to touch).

## Log
- <date time> — <one line, append-only>
```

**Update triggers** — the STATE file is written:
1. At effort start (objective, gates planned).
2. After every meaningful step (completed item, decision made, gate result).
3. **Before any context refresh, compaction, or session end** — this is the whole point: a fresh context reconstructs from STATE.md + `git status`, never from memory of the old conversation.
4. At close (final gate statuses, ledger rows flipped, pointer to the PR).

## Verification gates — no PASS without raw output

The gates mirror `.github/workflows/ci.yml` (the repo's canonical gate list):

| Gate | Command / job |
|---|---|
| Format | `cargo fmt --all -- --check` |
| Migration rollback | `bash scripts/verify-migrations.sh` (CI-only by design) |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Deps/licenses | `cargo deny check` |
| Rust tests | `cargo test --workspace -- --test-threads=1` |
| TS contract drift (ARCH-02) | type-export test + `git diff --exit-code -- apps/client/src/lib/generated` |
| sqlx offline cache | `cargo sqlx prepare --workspace` (refresh is a post-merge chore from green runs) |
| WASM parity | wasm32 core builds + `node scripts/wasm-parity.mjs` |
| Client | `npm run build -w @medical-os/client` + `node scripts/trust01-audit.mjs` |
| Site | `npm run build -w @medical-os/site` |
| E2E | Playwright suite (`tests/e2e/`) |
| Zitadel | `bash infra/zitadel/provision.sh` twice (idempotency) |
| Deploy verify | cosign verify + `version.json` sha match (deploy.yml `production-verify`) |

Rules:

1. A gate is marked **PASS** only with raw evidence: a GitHub Actions run URL **plus the job name**, or a trimmed log excerpt saved under `.scratch/<slug>/evidence/`. "The implementation should pass" is not a gate result — it is the failure mode this file exists to prevent.
2. **NOT RUN is a valid, honest state** and must stay visible. An effort closes only with every relevant gate PASS (or the blocker named).
3. Full raw GH log dumps belong in `*.raw.log` (git-ignored). Evidence files are **trimmed excerpts** of the failing/passing section, tracked in git.
4. Per the compute policy, heavy gates run in GitHub Actions. Local runs (e.g. `.scratch/e2e-local.sh` PASS/FAIL lines, `node scripts/ax-audit.mjs`) are supplementary evidence, clearly labeled `local`. The VPS never runs gates.
5. Marking a ledger row `tested` follows the ledger's own rule: acceptance evidence filed, Actions run cited.

## The loop

```
START   read AGENTS.md → docs/agents/* → the effort's STATE.md → `git status` + recent `git log` to reconcile
PLAN    objective + gate list into STATE.md
EXECUTE smallest step that moves one gate
VERIFY  run the gate (Actions preferred), capture raw output
RECORD  update STATE.md (completed, evidence, next action)
CHECK   `git status` / `git diff` — recorded Active diffs must match reality
  ↓
repeat, or CLOSE: ledger rows flipped → STATE.md closed → PR
```

Session start ritual: reconcile first. If the working tree contains diffs that are not yours, fingerprint them (`git diff --stat`, recent commit/mtimes) and record them under Do-not-touch — another agent may be mid-flight.

## Multi-agent shared worktree

Codex and ZCode work the same clone. Therefore:

1. Before starting: `git status` + recent commits + STATE.md freshness across `.scratch/*/STATE.md`. If another effort has uncommitted changes or commits newer than your last observation, it is **active** — do not branch-switch or reset the shared worktree.
2. Concurrent efforts isolate via **linked git worktrees**: `git worktree add ../<name> -b <branch> origin/main`. Leave the shared worktree's branch alone.
3. Record claims before long operations: if you begin a step another agent could collide with, write it in your STATE.md first (the Log timestamps intent).
4. Never delete or "clean up" files you did not create without checking recency (`git status`, mtimes) — they may be another effort's evidence.
5. Untracked evidence in the shared worktree that your effort needs: **copy** it into your worktree as tracked evidence; leave the original.

## Codex delegations

A delegation prompt must contain: the narrow absolute workspace, the effort's `.scratch/<slug>/STATE.md` path, the instruction "do not delegate further", and the requirement to **update STATE.md (Decided / Completed / gates / Next action) before reporting completion**. Codex inherits this protocol through `AGENTS.md`; keep delegation prompts pointed at it. Completion reports follow the handoff honesty rule: implemented / tested / partial / blocked / deferred, with evidence.

## Ledger integration

STATE.md references requirement IDs but **never restates their status** — `docs/requirements/traceability.md` is the single per-ID source of truth. On close, flip the IDs your effort completed per the ledger's update rule (`tested` + evidence link), then record the batch as a dated evidence blockquote in the ledger header. The user's stack maps as: PROJECT_ARCHITECTURE → `CONTEXT.md` + `design.md` + `docs/adr/`; TASKS.json → `.scratch/` issues + the ledger; VERIFICATION.md → the Verification gates section above.

## Enforcement

`scripts/ax-audit.mjs` runs in CI (rust job, next to the TRUST-01 audit) and fails when: a ledger row claims `tested`/`resolved` with an empty Evidence cell; a STATE.md gate row marked PASS has empty evidence; a STATE.md is missing required section headers. Docs mandate + mechanical check — same pattern as TRUST-01 "no fake anything".
