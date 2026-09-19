# 01: QB-17 Phase 1 session results and actions

Status: in-progress

Requirement IDs: QB-17 (Phase 1 only; Phase 2 answer-change analytics deferred).

Implement `../spec.md` through the existing practice submission and session-page seams.

- [x] Submission returns server-authoritative `time_taken_seconds`.
- [x] Result arithmetic remains truthful: total = correct + incorrect + skipped; score is the existing integer percentage.
- [x] Result UI shows score, correct, incorrect, skipped, total, and time taken.
- [x] Submitted results survive reload from persisted evidence.
- [x] Exactly one primary next-best action is emphasized.
- [x] Completed-session review is read-only and uses real submitted-session data.
- [x] Retry reuses the current chapter/session configuration when supported.
- [x] Practice-missed starts a real revision session from the submitted source session.
- [x] Revision selection includes incorrect attempts, explicit skips, and truly unanswered items.
- [x] Unsupported similar-question/weak-chapter actions are not fabricated.
- [x] Answer-change analytics remains explicitly Phase 2.
- [x] Red API/browser contracts are committed before implementation.
- [x] Heavy verification runs only in GitHub Actions.
- [x] Matt Standards/Spec review and Ponytail review are completed.
- [x] Traceability is updated truthfully.

## Comments

- Existing `score` already represents integer percent, so adding a duplicate `percentage` field would be unnecessary API surface.
- Existing revision-session behavior is the backend seam for “practice missed”; the selection query must include explicit skips (`chosen_index IS NULL`) as well as incorrect and truly unanswered items.
- RED contract commit: `b09d0c78ac8f6f039db6f1f1e5ed1e8c35eac6dc`. GitHub Actions run `35432635774` stopped before any runner steps because of the account billing/spending-limit state, so it is infrastructure evidence only, not a functional RED result.
- Matt Standards review: no substantive defect remained after aligning the `get_session` documentation with the submitted-session reveal policy. The implementation preserves existing routes, authorization, score semantics, and first-answer-wins evidence.
- Matt Spec review: Phase 1 acceptance points are implemented, including persisted result reconstruction, server-authoritative elapsed time, read-only review, truthful revision selection, retry, and exactly one primary result action. Answer-change analytics remains Phase 2 because the evidence model does not record answer changes.
- Ponytail review: Lean already. The slice reuses the existing submit/session/create-session/revision seams and adds no service, table, migration, endpoint, or dependency.
- Local verification is intentionally lightweight: formatting/static diff checks only. Heavy API/browser acceptance remains unexecuted until GitHub Actions can acquire runners.
- Post-implementation CI run `35433044640` for commit `586559975d96392b7cd10c2f455510e6044458d3` again failed before any runner steps. GitHub's annotation states: `The job was not started because recent account payments have failed or your spending limit needs to be increased.` No functional acceptance result was produced.
