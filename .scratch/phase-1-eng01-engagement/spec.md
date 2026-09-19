# ENG-01 Phase 1 — Daily goal, streak freezes, and question of the day

## Problem Statement

Medical OS already stores learner study capacity, submitted practice-session timing, protected commitments, notification timezone preferences, and real question attempts. It does not yet expose the Phase 1 engagement mechanics required by §17.2: daily-goal progress, a non-punitive streak with earned freezes, and one question of the day per exam. Those mechanics must remain optional and must not introduce a second practice engine or client-only counters that can drift from server evidence.

## Solution

Add one server-authoritative engagement surface that derives daily goal progress from real submitted-session elapsed time, derives streak/freeze state from dated goal evidence and protected commitments, and exposes a deterministic published question of the day per learner/exam/local date. Reuse the existing practice-session insertion and answer-secrecy behavior to start QOTD as a normal one-question tutor session. Keep a small learner preference record for enabling/disabling each mechanic and for the learner-chosen QOTD time.

## User Stories

1. As a learner, I can see today's study-minutes progress against my saved daily-minute goal.
2. As a learner, only submitted study sessions contribute elapsed minutes to daily-goal progress.
3. As a learner, my engagement day follows my saved notification timezone rather than an unrelated client clock.
4. As a learner, I can disable daily-goal progress without disabling streak or QOTD.
5. As a learner, I can disable streak without disabling daily-goal progress or QOTD.
6. As a learner, I can disable QOTD without disabling the other mechanics.
7. As a learner, a day extends my streak only when the daily-minute goal is met.
8. As a learner, a protected-commitment date is treated as a neutral rest day and does not break my streak.
9. As a learner, I earn one streak freeze after each seven goal-met days, with at most two freezes held.
10. As a learner, an unprotected missed day consumes one held freeze before the streak can break.
11. As a learner, I can see the current streak length and freezes held without exaggerated gamification.
12. As a learner, I receive one stable QOTD selection per exam for the same local date.
13. As a learner, QOTD never selects unpublished or quarantined question versions.
14. As a learner, starting QOTD uses the existing active-session takeover behavior.
15. As a learner, the QOTD session does not expose an answer key or rationale before I answer.
16. As a learner, community answer distribution is hidden until I answer the QOTD.
17. As a learner, after answering I can see a community split derived from real attempts rather than synthetic percentages.
18. As a learner, I can set the local time at which QOTD becomes due for reminder/notification infrastructure.
19. As a learner, the product does not claim an external push was delivered when no APNs/FCM dispatcher exists.

## Implementation Decisions

- Keep the server authoritative for all engagement state. The client renders returned state and never owns a streak counter.
- Reuse `learner_goal_versions`, `practice_sessions`, `session_items`, `attempts`, `question_versions`, `question_reports`, and `notification_preferences` instead of adding parallel learning tables.
- Daily-goal progress counts elapsed time only for submitted sessions and only for the learner-local date derived from the saved notification timezone.
- Phase 1 persists minute goals only; do not invent a question-count goal representation in this slice.
- Add a minimal `engagement_preferences` row keyed by user with independent booleans for daily goal, streak, and QOTD plus optional `qotd_time`.
- QOTD selection is deterministic from the learner, exam, and local date over eligible published/non-quarantined questions. A separate assignment table is unnecessary while the deterministic contract is sufficient.
- Start QOTD by reusing the existing practice-session insertion path with exactly the selected question and the same takeover semantics as normal practice.
- Community split is computed from real attempts for the selected question and is returned only after the current learner has answered it.
- Reuse notification timezone as the learner-local day boundary. `qotd_time` is stored and exposed as due-state input for the existing notification foundation; external APNs/FCM delivery is not claimed because the repository has no dispatcher.
- Protected commitments from the latest goal version are the first honest Phase 1 source for neutral rest dates.
- The master plan says freezes are earned but does not define an earning threshold. Phase 1 therefore uses an explicit implementation rule: earn one freeze per seven goal-met days, hold at most two; on an unprotected miss consume one held freeze before breaking the streak. This is an implementation decision, not a quoted product requirement.
- Keep the Today UI compact and reuse its existing design tokens and session-opening logic. No new theme, dependency, or gamification dashboard is introduced.
- Deployment is out of scope.

## Public Seams

- `GET /v1/me/engagement` returns preferences, learner-local date, daily-goal progress, streak/freeze state, and per-exam QOTD summary.
- `PUT /v1/me/engagement/preferences` updates the three independent mechanic toggles and QOTD time.
- `POST /v1/me/engagement/qotd/{exam_id}/session` starts the selected QOTD through normal practice-session semantics, including explicit takeover.
- The existing answer/session endpoints remain authoritative for answering and answer-key secrecy.
- The real Today page renders compact engagement state, independent toggles, QOTD start, and post-answer community split.

## Testing Decisions

- API integration tests are the primary RED/green seam for preferences, local-day progress, streak/freeze derivation, QOTD selection, answer secrecy, community split, and session takeover.
- Tests manipulate persisted session timestamps and goal history only through test setup, while all product behavior assertions go through HTTP endpoints.
- Heavy build/browser acceptance belongs in GitHub Actions. Local verification is limited to formatting/static/diff checks and other lightweight commands.

## Out of Scope

- Building or claiming an APNs/FCM delivery worker.
- Marketing/retention experiments, streak-anxiety optimization, leaderboards, badges, or penalty mechanics.
- A separate QOTD answer/quiz engine.
- Question-count goal persistence.
- Deployment, merge, tag, release, or workflow activation.

