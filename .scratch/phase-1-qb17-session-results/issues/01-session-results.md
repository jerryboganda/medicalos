# 01: QB-17 Phase 1 session results and actions

Status: in-progress

Requirement IDs: QB-17 (Phase 1 only; Phase 2 answer-change analytics deferred).

Implement `../spec.md` through the existing practice submission and session-page seams.

- [ ] Submission returns server-authoritative `time_taken_seconds`.
- [ ] Result arithmetic remains truthful: total = correct + incorrect + skipped; score is the existing integer percentage.
- [ ] Result UI shows score, correct, incorrect, skipped, total, and time taken.
- [ ] Exactly one primary next-best action is emphasized.
- [ ] Completed-session review is read-only and uses real submitted-session data.
- [ ] Retry reuses the current chapter/session configuration when supported.
- [ ] Practice-missed starts a real revision session from the submitted source session.
- [ ] Revision selection includes incorrect attempts, explicit skips, and truly unanswered items.
- [ ] Unsupported similar-question/weak-chapter actions are not fabricated.
- [ ] Answer-change analytics remains explicitly Phase 2.
- [ ] Red API/browser contracts are committed before implementation.
- [ ] Heavy verification runs only in GitHub Actions.
- [ ] Matt Standards/Spec review and Ponytail review are completed.
- [ ] Traceability is updated truthfully.

## Comments

- Existing `score` already represents integer percent, so adding a duplicate `percentage` field would be unnecessary API surface.
- Existing revision-session behavior is the backend seam for “practice missed”; the selection query must include explicit skips (`chosen_index IS NULL`) as well as incorrect and truly unanswered items.
