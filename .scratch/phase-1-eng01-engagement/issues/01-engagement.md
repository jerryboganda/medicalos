# 01: ENG-01 daily goal, streak freezes, and QOTD

Status: implemented — heavy acceptance pending CI

Requirement ID: ENG-01.

Implement `../spec.md` through the existing goals, notification, practice, and Today seams.

- [x] RED API contract exists before implementation.
- [x] Preferences are isolated per learner and all three mechanics can be disabled independently.
- [x] Daily minutes derive only from real submitted-session elapsed time in the learner-local day.
- [x] Protected commitments are neutral rest days.
- [x] Streak extends only on goal-met days.
- [x] One freeze is earned per seven goal-met days, with at most two held.
- [x] An unprotected miss consumes a held freeze before breaking the streak.
- [x] QOTD is stable for learner + exam + local date and excludes unpublished/quarantined questions.
- [x] QOTD starts as an existing one-question study session and preserves active-session takeover behavior.
- [x] Answer keys/rationales remain hidden before answer.
- [x] Community split is absent before the learner answers and truthful afterward.
- [x] QOTD local reminder time is persisted/exposed without claiming nonexistent external push delivery.
- [x] Today UI remains compact and uses the existing design system.
- [x] No new dependency is introduced.
- [x] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [x] Traceability is updated truthfully.
- [x] Heavy verification remains delegated to GitHub Actions.

## Comments

- Deployment is explicitly skipped.
- Freeze earning threshold is an explicit Phase 1 implementation decision because §17.2 does not define a threshold: one earned freeze per seven goal-met days, maximum two held.
- RED contract checkpoint: `1d2cebb`.
- Matt Standards review: no substantive standards defect found. The slice preserves authenticated learner ownership, existing practice/session semantics, existing answer-secrecy behavior, repository error conventions, and the existing Today design system; no new dependency or parallel quiz engine was introduced.
- Matt Spec review: the ENG-01 contract is implemented across independent preferences, learner-local submitted-session minutes, non-punitive streak/freezes, deterministic eligible per-exam QOTD, takeover reuse, pre-answer secrecy, post-answer real community split, and reminder due state. Real APNs/FCM delivery remains outside this slice exactly as specified.
- Ponytail review: Lean already. The implementation reuses existing goals, practice sessions, attempts, question reports, notification timezone, native time input, and current UI tokens. No speculative service, analytics table, assignment table, scheduler, or dependency was added.
- Hallmark closeout: the Today addition stays a compact operational card inside the existing shell/tokens, uses native labelled controls, retains existing button hierarchy, and introduces no new theme or decorative gamification surface.
- Accessibility static review: settings exposes `aria-expanded`; switches and time input use native labelled controls; save feedback uses a polite live region; load/save errors use alert semantics.
- Product analytics boundary: the taxonomy names `daily_goal_met`, `streak_extended`, and `streak_lost`, but the repository has no real product-analytics event sink yet. ENG-01 does not fabricate telemetry; emission remains OPS-05 work.
- Local verification is intentionally lightweight under the compute policy: `cargo fmt --all -- --check` and `git diff --check` pass. Heavy Rust/PostgreSQL/browser acceptance remains delegated to GitHub Actions.
- Post-implementation CI run `35439962380` for commit `ec1a6da5c6668e6e4ac0ad5cdf5ed8662038d473` could not execute functional checks. Client, site, and e2e jobs failed before any steps with GitHub's annotation: `The job was not started because recent account payments have failed or your spending limit needs to be increased.` The Rust job was still queued at the observation point, so no acceptance claim is made from this run and it will not be retry-looped.
