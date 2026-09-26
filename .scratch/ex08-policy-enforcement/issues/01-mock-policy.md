# EX-08 — Configurable mock grace and integrity policy

Status: in-progress
Requirement IDs: EX-08
Triage label: in-progress
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §11.3; `.scratch/ex08-policy-enforcement/spec.md`

## Acceptance

- Admin mock configuration persists a bounded late-sync grace and a `log_only`,
  `warn`, or `auto_submit` away policy.
- Sessions snapshot policy at start; valid late uploads are assisted and
  never contribute to percentile ranking.
- Server timestamps bound away intervals; warning is once per interval and
  auto-submit runs from a durable server worker through the existing submit
  receipt path.
- The browser handles return actions without blocking normal timed-session
  behavior.
- API integration and browser E2E pass in GitHub Actions, including migration
  rollback/replay.

## Implementation decisions

- Reuse `mocks`, `practice_sessions`, `integrity_events`, and the existing
  `practice::submit` completion path.
- Preserve the existing 600-second grace for `tutor` and `timed` practice;
  default mock grace to 600 seconds and allow administrators to set zero.
- Keep `log_only` as the default. Only an explicit `warn` or `auto_submit`
  configuration enables a timeout action.
- Keep late answers in a learner's result while marking the whole attempt
  unranked; correction-driven percentile recalculation must also filter them.
- Use an idempotent, bounded server worker for auto-submit so enforcement does
  not depend on a browser returning or a process-local timer surviving restart.

## Testing decisions

- Add real-router integration coverage for configuration validation, session
  snapshots, late answer acceptance/ranking exclusion, warning behavior, and
  worker submission.
- Extend the existing Playwright timed-session flow to verify the return
  signal reaches the API and the displayed action remains usable.
- Run tests and builds only through the final GitHub Actions gate.

## Comments

- 2026-09-26: Scope derived from §11.3's proposed practice grace and per-test
  log/warn/auto-submit policy. Competition's separate scoring path is excluded.
