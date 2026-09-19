# 01: Five-destination learner shell

Status: claimed

Requirement IDs: CORE-05; supports CORE-06, PLAN-01, AI-07, SR-01/02, QB-03.

Implement the vertical slice in `../spec.md` through the browser E2E seam.

- [x] Authenticated shell exposes exactly Today, Practice, Learn, Coach, and Progress as primary destinations.
- [x] Account and Sign out remain secondary controls outside the primary destination set.
- [x] Current destination is identifiable and navigation remains keyboard/focus accessible.
- [x] Practice starts real tutor and timed sessions through the existing API.
- [x] Practice preserves explicit active-session takeover behavior.
- [x] Learn surfaces the real review queue and links into the existing Review workflow.
- [x] Coach shows only persisted plan revisions/explanations and has an honest no-activity state.
- [x] Progress shows persisted learner chapter evidence and preserves low-evidence disclosure.
- [x] All four new destinations have truthful loading, error/retry, and empty states.
- [x] New routes are deep-linkable and reuse existing auth/API/design-system patterns.
- [ ] Playwright proves the five-destination shell and real capability paths against the real client/API/database stack.
- [ ] GitHub Actions passes; no heavy local/VPS compute and no deployment.

## Comments

- Spec synthesized from the Phase 1 master plan and current repository capabilities. The existing Playwright client/API/database path is the acceptance seam.
- TDD red evidence: run 35429347073 executed and failed at the intended first navigation assertion because the primary navigation did not yet exist.
- Implementation commits `a6ee5cb` and `4f02626` complete the vertical slice, including hiding primary destination navigation inside `/session/...` study workspaces.
- Final browser acceptance is blocked externally. Runs 35429885728 and 35430203458 started no job steps because GitHub reported failed account payments or an insufficient spending limit. Heavy tests were not moved to local development or the production VPS.
