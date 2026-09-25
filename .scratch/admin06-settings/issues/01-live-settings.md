# ADMIN-06 live settings

Status: ready-for-agent
Type: task
Requirement IDs: ADMIN-06, OPS-06
Spec: `.scratch/admin06-settings/spec.md`

## Acceptance criteria

- [ ] Validate all five supported settings and reject invalid batches atomically.
- [ ] Persist settings and their old/new audit record in one transaction.
- [ ] Resolve stored overrides at runtime for every current consumer.
- [ ] Add an admin settings form to the Editorial Console.
- [ ] Cover API behavior with integration tests and the form with Playwright.
- [ ] Update traceability without claiming remaining §19.5 settings are complete.

## Implementation record

- The five supported settings are validated at the API boundary, with unknown,
  empty, and invalid updates rejected. A transaction applies the batch and
  records the actor plus old/new values in the audit log.
- Runtime consumers now read current overrides for question and Coach limits,
  community sample gates, mastery bands, and re-test intervals. Public config
  exposes only the learner-safe question allowance.
- The Editorial Console has a responsive settings form with server error and
  save states. API integration and Playwright regressions are authored.
- The two-axis review found no gaps in this scoped settings slice. Final GitHub
  Actions verification is deferred; CSV/Excel imports and the other §19.5
  controls remain out of scope.
