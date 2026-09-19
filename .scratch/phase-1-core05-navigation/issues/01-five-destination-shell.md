# 01: Five-destination learner shell

Status: claimed

Requirement IDs: CORE-05; supports CORE-06, PLAN-01, AI-07, SR-01/02, QB-03.

Implement the vertical slice in `../spec.md` through the browser E2E seam.

- [ ] Authenticated shell exposes exactly Today, Practice, Learn, Coach, and Progress as primary destinations.
- [ ] Account and Sign out remain secondary controls outside the primary destination set.
- [ ] Current destination is identifiable and navigation remains keyboard/focus accessible.
- [ ] Practice starts real tutor and timed sessions through the existing API.
- [ ] Practice preserves explicit active-session takeover behavior.
- [ ] Learn surfaces the real review queue and links into the existing Review workflow.
- [ ] Coach shows only persisted plan revisions/explanations and has an honest no-activity state.
- [ ] Progress shows persisted learner chapter evidence and preserves low-evidence disclosure.
- [ ] All four new destinations have truthful loading, error/retry, and empty states.
- [ ] New routes are deep-linkable and reuse existing auth/API/design-system patterns.
- [ ] Playwright proves the five-destination shell and real capability paths against the real client/API/database stack.
- [ ] GitHub Actions passes; no heavy local/VPS compute and no deployment.

## Comments

- Spec synthesized from the Phase 1 master plan and current repository capabilities. The existing Playwright client/API/database path is the acceptance seam.
