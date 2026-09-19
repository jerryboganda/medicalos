# 01: ENG-01 daily goal, streak freezes, and QOTD

Status: ready-for-agent

Requirement ID: ENG-01.

Implement `../spec.md` through the existing goals, notification, practice, and Today seams.

- [ ] RED API contract exists before implementation.
- [ ] Preferences are isolated per learner and all three mechanics can be disabled independently.
- [ ] Daily minutes derive only from real submitted-session elapsed time in the learner-local day.
- [ ] Protected commitments are neutral rest days.
- [ ] Streak extends only on goal-met days.
- [ ] One freeze is earned per seven goal-met days, with at most two held.
- [ ] An unprotected miss consumes a held freeze before breaking the streak.
- [ ] QOTD is stable for learner + exam + local date and excludes unpublished/quarantined questions.
- [ ] QOTD starts as an existing one-question study session and preserves active-session takeover behavior.
- [ ] Answer keys/rationales remain hidden before answer.
- [ ] Community split is absent before the learner answers and truthful afterward.
- [ ] QOTD local reminder time is persisted/exposed without claiming nonexistent external push delivery.
- [ ] Today UI remains compact and uses the existing design system.
- [ ] No new dependency is introduced.
- [ ] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [ ] Traceability is updated truthfully.
- [ ] Heavy verification remains delegated to GitHub Actions.

## Comments

- Deployment is explicitly skipped.
- Freeze earning threshold is an explicit Phase 1 implementation decision because §17.2 does not define a threshold: one earned freeze per seven goal-met days, maximum two held.
