# AI-08 — Protected tasks and plan-churn controls

Status: ready-for-agent
Requirement IDs: AI-08, AI-07, PLAN-01, PLAN-02
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§8.5–8.6, 9.3–9.4
Implementation state: implementation and regressions drafted; fresh review and API/browser/rollback CI pending.

## Problem statement

Capacity replanning currently removes pending tasks without a learner-protectable
flag, accepts no plan version, and can race another replan. Repeated practice
sessions can also create an unbounded sequence of automatic plan revisions.
It also compares question counts directly with minute budgets, even though those
units differ.

## Solution

Add a learner-controlled protection flag to plan tasks. Capacity replans preserve
protected and completed tasks, account only for remaining pending work, reject a
budget that cannot fit protected work, and apply only to the plan version the
learner reviewed. Keep plan changes within explicit daily limits and make
automatic revisions idempotent by source session. Expose these controls in the
existing Today page. Store a separate estimated duration for each task; use a
transparent baseline of 1.5 estimated minutes per question, rounded up, and
carry that estimate through plan versions. Question count continues to control
session size, while the estimate controls minute budgets.

## Acceptance

- Learners can protect or unprotect a pending task on their current plan; a task
  belonging to another learner or an outdated plan cannot be changed.
- A capacity replan includes the expected plan version. Stale requests return a
  conflict with the current version and do not mutate plan data.
- Completed work is preserved and excluded from the remaining time budget.
  Protected pending work is preserved; if it alone exceeds the requested
  capacity, the server explains the required minutes and makes no plan change.
- Unprotected pending work is kept in plan order while it fits and deferred work
  is listed in an undoable revision receipt. Protection state survives a plan
  fork.
- Capacity calculations use `estimated_minutes`, never `question_count`; Today
  shows both the estimate and question count so learners can distinguish them.
- A plan day permits at most eight version-changing revisions and at most three
  automatic revisions. Repeated automatic handling of one source session does
  not create another task or revision.
- A task has one stable identity across plan versions. A session launched from
  Today carries that identity, and submission completes only the linked task;
  a same-chapter sibling remains pending. Linked sessions must match the
  planned question count and fail closed when the eligible pool is too small.
- Replaying the registered migration stack is safe and does not overwrite a
  learner's custom task estimate. This includes the registered OIDC migration
  that precedes the AI-08 schema changes.
- The eighth version-changing revision remains undoable; Undo does not consume
  a ninth revision slot.
- Today displays task protection, remaining-plan workload controls, the current
  plan version, and the automatic revision cap. It explains failures and
  refreshes after a stale-version response.

## Verification seam

Use API integration coverage for task ownership/protection, capacity behavior,
stale concurrent requests, version limits, and automatic idempotency. Use the
existing Today route in Playwright to exercise protection and replan request
payloads. Do not run local builds, test suites, screenshots, or generated
artifacts; final GitHub Actions remains the acceptance gate.

## Implementation record

- Capacity is measured from the explicit `estimated_minutes` field, not
  `question_count`. Legacy tasks receive a transparent baseline of 1.5 minutes
  per question, rounded up; new cold-start tasks carry a 15-minute estimate for
  10 questions. Today labels estimates and question counts separately.
- Regression coverage now distinguishes question count from estimate, verifies
  capacity selection and protected-minute calculations, checks automatic
  revision estimates, and covers the Today estimate display.
- Migration `0035` is registered in the shared schema path. Protected minutes
  are reserved before optional tasks are considered, regardless of plan order.
- Migrations `0034` and `0035` are replay-safe; `0036` assigns stable task keys
  and stores the selected key on plan-launched practice sessions.
- Capacity undo restores the exact deferred work from its source plan version;
  including task identity; Today exposes deferred task titles from the receipt.
- Undo also resolves pre-AI-08 title-only capacity receipts against their
  immutable source plan, and stale Undo refreshes Today before showing the error.
- First-use plan creation and task-specific session completion share the user
  lock with replanning. Completion updates matching task identities across
  same-day plan versions so undo cannot resurrect completed work as pending.
- Automatic revision creation uses the same session-then-user lock order as
  completion. Session rows use a no-key-update lock so a plan fork's foreign
  key check does not deadlock with a submit retry holding the learner lock.
- Task-linked launch validates ownership, current-plan state and exact planned
  question count; it refuses a pool that cannot satisfy that count.
- Legacy task snapshots with defaulted creation timestamps are normalized by
  task shape and per-plan ordinal, not by `created_at`; the replay fixture now
  copies rows without preserving that timestamp.
- Task-linked launch holds the learner row lock from current-plan validation
  through session/item insertion, serializing it with capacity replanning while
  remaining compatible with the session foreign-key check. A concurrent
  launch/replan API regression is authored.
- Linked revision sessions count skipped questions as missed, select exactly
  the planned number from eligible published versions, and refuse shortages
  after quarantine.
- The daily cap counts revision records. Reversing the eighth revision forks a
  new plan version without consuming an additional revision slot.
- Timed task launch preserves the task's question count. Regression cases cover
  this alongside sibling-task completion, migration replay, concurrent first
  use, submit/replan interleavings, the revision cap/undo boundary, count
  mismatches and eligible-pool shortfall. Browser coverage uses a protected
  six-minute task plus a fifteen-minute optional task under a ten-minute budget.
- Final API/browser/rollback acceptance remains pending in GitHub Actions.
