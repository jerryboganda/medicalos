# EX-08 Configurable mock grace and integrity policy

Status: CI-verified
Requirement: EX-08

## Outcome

Admin-configured practice mocks carry an immutable, server-enforced policy for
late offline answer uploads and time away from the assessment. Learners can
review accepted late answers, but a mock containing any late upload is never
included in percentile ranking.

## Policy

- `late_sync_grace_seconds` is configurable from 0 through 600 seconds and
  defaults to 600, matching the existing practice grace. A value of zero turns
  late sync off. A late answer is accepted only while the session remains open,
  before the configured grace expires, and when its client timestamp falls
  between the server-created session and its server-issued deadline. The server
  records that timestamp and marks the answer assisted.
- `integrity_policy` is `log_only`, `warn`, or `auto_submit`; the default is
  `log_only`. `warn` and `auto_submit` require an away timeout from 15 through
  3600 seconds. The selected values are copied to the session at start and are
  unaffected by later mock edits.
- The server records one `away_since` timestamp when it first receives a
  `background` or `window_blur` signal. A `foreground` signal closes the away
  interval. Duplicate leave/return signals cannot restart or repeat an action.
- `warn` returns one warning for an away interval that meets the configured
  timeout. `auto_submit` is enforced by a bounded server worker after the
  timeout, even if the learner does not return; a returning client receives
  the persisted submission receipt. Submission uses the existing idempotent
  completion path.
- A mock containing an answer uploaded after its deadline has a learner-facing
  score and answer review, but is excluded from percentile calculations. The
  same filter applies when question corrections recalculate stored percentiles.
- Existing practice `tutor` and `timed` sessions retain their current 600-second
  late-sync allowance. Other sessions retain zero unless an assessment sets a
  policy explicitly.

## Acceptance

- Admin mock creation validates and persists grace and integrity settings;
  listing returns them and start snapshots them onto the practice session.
- A late answer outside the configured grace or with an invalid client time is
  rejected. A valid late answer is durably recorded as assisted.
- A valid late answer contributes to the learner's score but excludes that
  mock attempt from ranked takers and percentile results.
- Leave/return signals record one server-timed away interval. `warn` produces a
  single warning after its timeout; `log_only` never changes session status.
- The production API worker submits an overdue `auto_submit` session using the
  normal persisted receipt and completion side effects.
- Browser UI reports warning/auto-submit results without blocking its timer,
  answering, or manual submit behavior.
- GitHub Actions verifies API integration, migration down/up, builds, and
  browser E2E. No local builds or test suites are part of the project compute
  policy.

## Boundaries

- Competition grace and leaderboard policy need a separate slice because
  competitions use their own answer and scoring tables.
- Browser signals remain best-effort observations. They do not prove why a
  learner left, detect screenshots or recording, or establish misconduct.
- Native attestation, offline high-stakes assessment, institutional policy
  approval, and human review remain separate acceptance gates.

## Implementation Record

- 2026-09-26: API integration, migration rollback/replay, and browser warning/receipt coverage passed in GitHub Actions run [36215825107](https://github.com/jerryboganda/medicalos/actions/runs/36215825107) on commit `05601c5a8e4d8df4c8a28db263151d02b61828ff` (111 API integration tests and 55 Playwright E2E tests in the full suite).
- Browser signals remain best-effort; this does not verify native device-clock resistance or competition-specific policy.
