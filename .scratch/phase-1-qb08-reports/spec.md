# Spec — QB-08: Question issue reporting + quarantined-item exclusion

Requirement IDs: QB-08 (Phase 1, partial — learner reporting + pool exclusion;
editorial review console and SLA timers arrive with ADMIN-06/QB-09 in Phase 2).
Source: master plan §11.2 (immediate issue reporting), §19.4 (issue reports +
quarantine). Triage: `ready-for-agent`.

## Problem Statement

A learner who spots a defective question (wrong answer key, broken explanation,
typo) has no way to report it, and the engine keeps serving reported-defective
items to other learners. Suspected defects must quarantine an item out of the
practice pools until an editor clears it (§19.4).

## Solution

Learner-scoped reporting on question versions: report once per question (same
category twice is a no-op), a validity threshold quarantines the item out of
tutor/timed/revision pools, and editors resolve reports to re-admit items.
Reported items carry their status on the session detail endpoint so the UI can
say so honestly.

## User Stories

1. As a learner, I want to report the question I am looking at as wrong-answer /
   bad-explanation / typo / duplicate / outdated / broken-image / other, so
   that bad items get fixed.
2. As a learner, I want my duplicate report of the same question to be accepted
   without creating a second record, so that retries are safe.
3. As a learner, I want quarantined questions to stop appearing in new practice
   sessions, so that I am not tested on defective items.
4. As a learner, I want to see that a question I answered was later flagged,
   so that I know what happened honestly.
5. As an editor (Phase 2 console will use this API), I want to resolve reports
   as fixed / rejected, so that items return to the pools with an audit trail.

## Implementation Decisions

- **Seam (testing):** the HTTP API only — POST
  /v1/questions/versions/{id}/reports, GET /v1/questions/versions/{id}/reports
  (own reports + quarantine status), POST /v1/reports/{id}/resolve (editor
  role arrives with CORE-04; until then the route exists, is authenticated,
  and returns 501 `editor_console_pending` — never a fake resolution).
- **Pool exclusion:** `insert_session` pool queries (`tutor`/`timed` presets)
  add `AND NOT EXISTS (SELECT 1 FROM question_reports WHERE
  question_version_id = qv.id AND status = 'quarantined')`; the revision pool
  query gains the same predicate. Quarantined items already in an open
  session stay answerable (never yank a live session's items).
- **Quarantine rule:** a report starts `open`; the third distinct-learner
  `open` report on the same version flips it (and its siblings) to
  `quarantined`. One learner = one vote per version (UNIQUE
  (question_version_id, reporter_id)).
- **Report detail:** free-text note capped at 2000 chars, category enum
  validated in Rust; reporter sees own report ids only (no cross-learner
  disclosure); resolve flips status to `resolved_fixed` / `resolved_rejected`
  with a resolution note + resolved_at.
- **Schema:** migration 0005_item_reports (up + down, §31.1); new
  `query!` macros regenerate `.sqlx/` in CI before merge (deploy.yml builds
  with the committed cache).
- **Session detail honesty:** `get_session` items gain `report_status:
  open|quarantined|resolved_fixed|null` so the UI can label affected items.

## Testing Decisions

- Tests verify public behavior through the API only, never internals (tdd
  skill). Prior art: `apps/api/tests/integration.rs` (same setup/LOCK/call
  helpers, same seam).
- Cases: report → duplicate same-learner report is idempotent; second and
  third distinct learners trigger quarantine; quarantined items excluded from
  new tutor sessions but a live session keeps them; resolve route returns 501
  until the editor console lands; session detail carries report_status;
  invalid category and over-long note are 422; migration up→down→up in the
  existing reversibility test scope.
- Fixtures: seeded synthetic ('gloopoid') content only; unique random emails
  per test; shared CI database safe via user-scoped UUID rows.

## Out of Scope

Editorial console UI + SLA timers (ADMIN-06/QB-09, Phase 2), answer-key
correction propagation (§19.4 dependency graph — LIB-05), 'corrected' badges
and changelogs, analytics reversal on affected attempts, notifications to
reporters, bulk import hooks.

## Further Notes

QB-16 (psychometric screening thresholds) stays Phase 2: this slice uses a
fixed 3-vote quarantine rule, not item statistics. QB-09 consumes the
`question_reports` table the console reviews.
