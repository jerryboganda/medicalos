# Practice competition entry

Status: in-progress
Type: task
Requirement IDs: COMP-01, COMP-02, COMMUNITY-03
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §17.1

## Problem

Competition APIs have no learner-facing Practice flow. Bulk submissions also
trust client answer times, allowing speed-bonus manipulation.

## Acceptance

- Practice lists events with upcoming/open/ended state, one-attempt status,
  and an opt-in path for learners without a community handle.
- Learners start one event attempt and can resume the same active attempt after
  a reload without resetting the server timer.
- The server randomizes question and option order, sends one question at a
  time, and never includes answer keys or rationales in the response.
- Answer requests accept only the active question and an option in the current
  displayed order. Elapsed time is measured by the server.
- Persisted responses and attempt progress are transactional and replay-safe.
- Final results feed the existing ranked leaderboard, including accuracy,
  attempt count, average response time, own-row pinning, and closed/reviewed
  prize eligibility.
- API integration and Practice browser coverage passed in GitHub Actions run
  [36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978).

## Implementation notes

Use the existing Practice route and owner-locked app tokens. Keep auth,
question disclosure, and server scoring at existing API seams. No new runtime
dependency or deployment action is needed.
