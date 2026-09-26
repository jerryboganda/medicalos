# ENG-01 — Declared-time daily goal

Status: CI-verified
Requirement: ENG-01
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§6.1, 17.2

## Problem Statement

The learner can enter daily available minutes to plan or replan tasks, but that value is transient. The streak goal is measured only as a fixed question count, so learners who want a time-based goal cannot use the time they declared as their engagement target.

## Solution

Persist the existing daily available-minutes value and let the learner choose whether the daily goal is measured in answered questions or recorded answering time. In minutes mode, the same declared availability is the goal target; do not add a second minutes-target prompt. Preserve question-count mode as the default for existing and new accounts.

The server reports the current mode, target, progress, and met state through the existing engagement endpoint. Recorded minutes include bounded elapsed time on completed practice answers and the QOTD answer. Skips, unanswered sessions, and missing or out-of-range durations contribute no minutes. The same existing disable controls govern the goal and streak.

## User Stories

1. As a learner, I want my daily available-minutes setting to persist, so that I do not have to re-enter it each day.
2. As a learner, I want the saved availability to remain the value used by daily planning, so that planning and my declaration stay consistent.
3. As a learner, I want to choose a question-count or minutes goal, so that I can measure a study day in the way that suits me.
4. As a learner who chooses minutes, I want my declared available time to become the target automatically, so that I do not configure the same number twice.
5. As a learner, I want to see current progress and units, so that I know whether today's goal is met.
6. As a learner, I want time to count only after I submit an answer, so that opening a session or leaving a question unanswered does not complete my goal.
7. As a learner, I want answered practice-item and QOTD durations to contribute to a minutes goal, so that both daily question paths count consistently.
8. As a learner, I want question mode to preserve the existing answered-question behavior, so that current goals continue to work after the migration.
9. As a learner, I want the goal's existing on/off control to apply in either unit, so that I can disable the mechanic without changing my study plan.
10. As a learner, I want invalid time targets rejected and invalid durations ignored, so that corrupt values cannot produce a false goal result.
11. As a learner, I want today's progress and streak decision to use the same database calendar day as the existing engagement system, so that answers near midnight are handled consistently.
12. As a learner, I want the goal mode and available minutes to survive sign-out and reload, so that the setting is account-scoped rather than browser state.

## Implementation Decisions

- Reuse the existing engagement settings endpoint and Today available-minutes control; add no new route or persistence service.
- Persist a `questions` or `minutes` goal mode and the learner's available minutes. Keep question-count mode as the migration and account default.
- In minutes mode, derive the target from saved available minutes. Keep the current configured question count unchanged so switching modes is reversible.
- Sum elapsed milliseconds only from completed practice answers and QOTD answers for the database calendar day. Use the existing practice-answer bound of 0–3,600,000 ms. Older QOTD clients that omit timing record zero.
- Return both progress measures from engagement status while selecting the configured measure for target comparison and streak completion.
- Report `daily_goal.met` against the current mode and target. Keep already-earned streak credit for the day even if the learner later changes modes or raises the target.
- Keep the current response fields and route behavior compatible; add the mode, units, time progress, and available-minutes fields.
- On Today, initialize the daily planning input from the persisted setting, save edits through the existing engagement settings call, and present a unit selector inside the engagement card.
- Preserve the existing question-goal, streak, QOTD, per-learner disable, and global kill-switch behavior.

## Testing Decisions

- Test behavior at authenticated HTTP endpoints and the Today browser flow; do not couple assertions to private functions.
- API integration coverage verifies persistence across reads, unit switching, validation, question-mode compatibility, completed-answer time aggregation, QOTD time aggregation, skipped/unanswered exclusion, and streak completion.
- Browser E2E verifies the learner can select minutes mode, see minute units and the persisted available-time target, reload, and switch back to question mode.
- GitHub Actions is the build/test authority. The existing engagement integration tests and learner-loop Playwright suite are the prior art.

## Out of Scope

- Measuring time spent on reading, flashcards, coaching, simulations, or other activity types that do not record per-answer elapsed time.
- Treating learner-reported answering duration as a proctored or integrity-grade time measurement.
- Scheduling QOTD reminders or delivering remote push notifications; the QOTD push provider is a separate open ENG-01 boundary.
- Changing the existing streak-freeze rules or daily question-goal limits.

## Further Notes

This implements the “questions or minutes” choice in §17.2 and makes the existing daily availability control persistent. It does not by itself complete ENG-01; QOTD reminder scheduling and remote delivery remain separate work.

## Implementation Record

- 2026-09-26: API integration, migration rollback/replay, and Today browser coverage passed in GitHub Actions run [36215825107](https://github.com/jerryboganda/medicalos/actions/runs/36215825107) on commit `05601c5a8e4d8df4c8a28db263151d02b61828ff`. The response reports `met` against the current mode and target while preserving already-earned streak credit after settings change.
- No deployment or production behavior change is implied by CI acceptance.
