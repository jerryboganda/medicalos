# QB-17 Phase 1 — Session results, elapsed time, and real result actions

## Problem Statement

Completed practice sessions currently show only score, correct, incorrect, and skipped counts. QB-17 requires a truthful completion summary with time taken and useful result actions, while the current evidence model deliberately enforces first-answer-wins and therefore cannot yet support answer-change analytics.

## Solution

Extend the existing practice submission and session page rather than creating a new results service. The submission response adds server-authoritative elapsed seconds derived from persisted session timestamps. The existing result card shows total and time taken and exposes only actions backed by real existing behavior: review the completed session, retry the same chapter/configuration, practice missed questions through the revision preset, and return to Today.

Answer-change analytics remains QB-17 Phase 2. No answer-change figures are inferred or fabricated from first-answer-only evidence.

## User Stories

1. As a learner, I can see score, correct, incorrect, skipped, total, and time taken after submitting a session.
2. As a learner, I see one clear primary next-best action rather than several equally weighted calls to action.
3. As a learner with incorrect or skipped questions, I can start a real revision session containing those missed questions.
4. As a learner, I can review the completed session without creating a duplicate session or mutating submitted answers.
5. As a learner, I can retry the same chapter with the same tutor/timed configuration when the source session has a chapter.
6. As a learner, I can always return to Today.
7. As a learner, I am not shown answer-change analysis until the evidence model actually records answer changes.

## Implementation Decisions

- `POST /v1/practice/sessions/:id/submit` remains the single result-calculation seam.
- Add `time_taken_seconds` to `SubmitResponse`; compute it from database-owned `created_at` and `submitted_at` timestamps, never from the browser clock.
- `score` remains the integer percentage already used by the product; no duplicate percentage field is introduced.
- The submitted-session review reuses the existing session route and question UI in read-only mode.
- After submission, completed-session detail may reveal answer keys/rationales for review; open sessions keep the existing reveal restrictions.
- Retry reuses `createSession` with the same chapter, question count, tutor/timed preset, and timed limit when applicable.
- “Practice missed questions” reuses `preset=revision` with `source_session_id`.
- The revision pool must include incorrect attempts, explicit skips (`chosen_index IS NULL`), and truly unanswered session items.
- Primary action order: practice missed questions when any missed items exist; otherwise retry when a chapter exists; otherwise return to Today.
- No new dependency, service, migration, result endpoint, analytics table, or speculative action is introduced.

## Testing Decisions

- API integration contract asserts `time_taken_seconds` is a non-negative integer and result arithmetic remains truthful.
- A dedicated integration contract asserts an explicit skip is eligible for a revision session.
- Browser E2E asserts total/time visibility, the real result actions, and read-only review entry on the existing learner loop.
- Heavy Rust tests, builds, PostgreSQL integration, and Playwright run only in GitHub Actions. Local work is limited to formatting/static inspection/diff review.

## Out of Scope — QB-17 Phase 2

- Right-to-wrong, wrong-to-right, or wrong-to-wrong answer-change analysis.
- Time-to-first-answer, time-on-explanation, rushed/over-long item analytics, community averages, percentiles, or mock pass-mark analysis.
- Practice-similar and weak-chapter actions until real backend selection contracts exist.
- Deployment.

## Completion Boundary

QB-17 remains partial after this slice: Phase 1 results/actions are implemented, while answer-change analytics stays pending until answer-change evidence is modeled and persisted.
