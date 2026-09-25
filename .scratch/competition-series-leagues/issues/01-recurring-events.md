Status: in-progress
Type: task
Requirement IDs: COMP-01, COMP-02

# Materialize recurring competitions

Implement the schedule behavior in `../spec.md` using the existing
admin-authenticated competition creation and learner list endpoints. Keep the
series window, approved question pool, and scoring snapshot durable. Add API
integration coverage; do not run the deferred final verification batch.

## Acceptance checklist

- [x] Create daily, weekly, and monthly series; preserve one-off and live events.
- [x] Validate question pool, cadence, question count, and non-overlapping windows.
- [x] Snapshot scoring policy and materialize randomized, published same-exam occurrences.
- [x] Preserve monthly day/end-of-month anchors and rotate fresh questions first.
- [x] Bound catch-up, lock each series row, and enforce unique occurrence starts.
- [x] Author concurrent/idempotent API coverage and monthly calendar unit coverage.
- [ ] Run the final formatter, API/client checks, Playwright suite, and GitHub Actions batch.

## Implementation record

Added migration `0052_competition_series_leagues`, series creation in
`routes/engagement.rs`, and bounded occurrence materialization on the
authenticated competition list. Each event receives a separate ID and copies
the series scoring snapshot. Practice now displays cadence and UTC windows.
Integration and unit coverage is authored but intentionally unrun until the
final implementation batch.
