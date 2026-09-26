# ENG-01 — Use declared daily time as an engagement goal

Status: in-progress
Requirement IDs: ENG-01
Triage label: in-progress
Source: `.scratch/eng01-time-goal/spec.md`

## Acceptance

- The daily available-minutes value is stored per learner and returned by engagement status.
- A learner can select question or minute mode; question mode remains the default and keeps existing behavior.
- In minute mode, the persisted available-minutes value is the target, with current progress and units shown in Today.
- Completed practice answers and the answered QOTD contribute bounded recorded time; skips, unanswered items, and missing/invalid durations contribute none.
- Goal met and streak state use the selected mode and the existing database calendar day.
- Mode and available minutes persist through reload; the learner can switch modes without losing the question-count target.
- Existing daily-goal and streak disable controls still work.
- GitHub Actions passes API integration and learner-loop browser E2E coverage.

## Comments

- 2026-09-26: Specified from master plan §§6.1 and 17.2. The public seams are the existing engagement settings/status endpoints and Today page. Implementation is in progress.
