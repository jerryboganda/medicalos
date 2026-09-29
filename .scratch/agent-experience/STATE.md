# STATE — agent-experience

Updated: 2026-09-29 23:15 (+0500)
Requirement IDs: none (process/docs infrastructure)
Branch: `zcode/ax-protocol` (worktree `D:\Projects\medicalos-ax`, cut from `origin/main` @ `7485cae`)

## Objective

Make the AX protocol mandatory for every future task in this repo: persistent per-effort STATE.md (externalized working memory), verification gates that require raw output evidence, the PLAN → EXECUTE → VERIFY → RECORD → CHECK GIT loop, and a CI audit gate — delivered as a green PR to `main`.

## Status

All changes written; local gates PASS; commit + PR next.

## Decided

- 2026-09-29: per-effort `.scratch/<slug>/STATE.md` (not a single root SESSION_STATE.md) — the repo is multi-effort and multi-agent; per-effort files cannot clobber each other and reuse the existing `.scratch/` convention.
- 2026-09-29: enforcement = AGENTS.md mandate + `scripts/ax-audit.mjs` CI gate (TRUST-01 pattern: static, stdlib-only, fails on evidence-less PASS/tested claims), wired into the `client` job next to TRUST-01.
- 2026-09-29: this effort runs in a linked worktree because `zcode/role-aware-admin` (role-aware admin gates) was actively worked in the main worktree — do not switch branches there.
- 2026-09-29: no new ledger IDs; no STATE.md retrofit onto the 36 existing `.scratch` dirs (they get one when next touched); the audit deliberately does NOT fail on missing STATE.md files in effort dirs (only the working agent can add it) — absence stays a review concern.
- 2026-09-29: loose `.scratch/ci-*.log` originals in the main worktree were copied (not moved) to `.scratch/evidence/` here — multi-agent rule: don't delete another agent's untracked files mid-flight.

## Do-not-touch

- Main worktree `D:\Projects\Medical OS`: branch `zcode/role-aware-admin` is another agent's active effort (commits `db4fac8`/`6d197d5`/`edb9afa`/`6151ea9`, pushed to origin). Never `git checkout`/reset there.
- Loose `.scratch/ci-*.log` in the main worktree: left in place; tracked copies live here under `.scratch/evidence/`. Delete the originals only when the main worktree is next quiet.
- `origin/zcode/role-aware-admin` content: out of scope for this PR.

## Completed

- [x] Recon: fingerprinted the parallel effort (4 commits on `zcode/role-aware-admin`, active minutes ago; origin/main at `7485cae`) — evidence: `git log`/`git branch -a` in session; conclusions in Decided.
- [x] Linked worktree `D:\Projects\medicalos-ax` created on `zcode/ax-protocol` off `origin/main`; 2026-09-28 CI failure logs copied to `.scratch/evidence/` — evidence: worktree creation output (HEAD at `7485cae`).
- [x] Protocol doc `docs/agents/agent-experience.md` (three-layer model, STATE.md template, ci.yml gate list, loop + rituals, multi-agent worktree rules, Codex delegation rule, ledger integration, enforcement).
- [x] Mandates wired: `AGENTS.md` (new mandatory AX section + delegation prompt rule), `docs/agents/issue-tracker.md` (STATE.md + evidence conventions), `AGENT_IMPLEMENTATION_HANDOFF.md` (source-of-truth #4, workflow chain, honesty sentence), `README.md` (hard rule 4, layout line, where-things-live).
- [x] `scripts/ax-audit.mjs` + `ci.yml` step "AX audit (no fake evidence)" in the `client` job — evidence: [evidence/ax-audit-local-2026-09-29.txt](evidence/ax-audit-local-2026-09-29.txt) (clean run exit 0; negative fixtures exit 1; cross-link check all OK).

## Active diffs

- Modified: `.github/workflows/ci.yml`, `.gitignore`, `AGENTS.md`, `AGENT_IMPLEMENTATION_HANDOFF.md`, `README.md`, `docs/agents/issue-tracker.md` (27 insertions total).
- New: `docs/agents/agent-experience.md`, `scripts/ax-audit.mjs`, `.scratch/agent-experience/` (spec + STATE + evidence), `.scratch/evidence/` (2 tracked log excerpts).
- Matches `git status --short` at 23:05; nothing else.

## Verification gates

| Gate | Runs where | Status | Raw-output evidence |
|---|---|---|---|
| `node scripts/ax-audit.mjs` | local + ci.yml `client` job | PASS | [evidence/ax-audit-local-2026-09-29.txt](evidence/ax-audit-local-2026-09-29.txt) — clean, 152 ledger rows, 1 STATE file, exit 0 (local; CI run pending below) |
| Audit negative test (catches evidence-free claims) | local fixtures | PASS | same evidence file — exit 1 with 2 findings on bad fixtures |
| Cross-links in new/edited docs resolve | local check | PASS | same evidence file — all 20 paths OK |
| PR CI (rust / client / site / e2e) | GitHub Actions on PR | NOT RUN | — run URL recorded in a follow-up commit after push |

## Blockers / owner inputs

None.

## Next action

Stage exactly the files in Active diffs → commit on `zcode/ax-protocol` → push → open PR to `main` → record the Actions run URL in the gates table (follow-up commit on this branch).

## Log

- 2026-09-29 22:40 — effort started; worktree isolated from active `zcode/role-aware-admin` effort; spec + STATE seeded.
- 2026-09-29 22:55 — protocol doc + 4 mandate edits + audit script written.
- 2026-09-29 23:05 — audit clean (152 rows, 1 STATE file), negative test verified, cross-links verified; evidence saved.
- 2026-09-29 23:15 — ponytail-review: 2 findings applied (existsSync shrink, sync-marker comment), script re-verified, evidence regenerated. Lean. Ship.
