Status: in-progress
Type: task
Requirement IDs: COMP-01, COMP-03, COMMUNITY-03

# Weekly opt-in league cohorts

Implement the weekly exam leagues in `../spec.md` through the authenticated
API and Practice page. Keep membership explicit, cohorts capped, standings
private to participants, and promotion/relegation tied to completed events.
Add API and browser coverage; do not run the deferred final verification
batch.

## Acceptance checklist

- [x] Require an explicit join and an opted-in community handle.
- [x] Create Monday-UTC exam cohorts with a 30-member limit and leave support.
- [x] Rank completed same-exam entries by points, accuracy, time, then stable ID.
- [x] Apply top-three promotion and bottom-three relegation only for eligible cohorts.
- [x] Return handles and standings for the caller's cohort only.
- [x] Author API coverage for opt-in, rollover, division movement, privacy, and capacity.
- [x] Author Practice browser coverage for cadence, join, standings, and leave.
- [ ] Run the final formatter, API/client checks, Playwright suite, and GitHub Actions batch.

## Implementation record

Added authenticated league state/join/leave routes. Missing current-week roster
memberships materialize in bounded exam-locked batches, prioritize the
requesting learner, and carry previous cohorts through score-based movement.
Practice now offers exam-scoped opt-in and private weekly standings. Coverage is
authored and intentionally unrun pending the final verification batch.
