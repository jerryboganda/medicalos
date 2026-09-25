Status: in-progress
Type: task
Requirement IDs: COMP-01, COMP-02

# Materialize recurring competitions

Implement the schedule behavior in `../spec.md` using the existing
admin-authenticated competition creation and learner list endpoints. Keep the
series window, approved question pool, and scoring snapshot durable. API, unit,
and browser coverage is verified by the final GitHub Actions batch.

## Acceptance checklist

- [x] Create daily, weekly, and monthly series; preserve one-off and live events.
- [x] Validate question pool, cadence, question count, and non-overlapping windows.
- [x] Snapshot scoring policy and materialize randomized, published same-exam occurrences.
- [x] Preserve monthly day/end-of-month anchors and rotate fresh questions first.
- [x] Bound catch-up, lock each series row, and enforce unique occurrence starts.
- [x] Author concurrent/idempotent API coverage and monthly calendar unit coverage.
- [x] Run the final formatter, API/client checks, Playwright suite, and GitHub Actions batch (run 36159484978).

## Implementation record

Added migration `0052_competition_series_leagues`, series creation in
`routes/engagement.rs`, and bounded occurrence materialization on the
authenticated competition list. Each event receives a separate ID and copies
the series scoring snapshot. Practice now displays cadence and UTC windows.
GitHub Actions run [36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978)
passed the integration and unit suites, WASM build, client/site builds, and
browser suite on source commit `a885734c5b28344254a68e6881c13bdee29fce6e`.
