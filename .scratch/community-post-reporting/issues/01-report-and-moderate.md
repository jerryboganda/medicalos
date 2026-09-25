Status: in-progress
Type: task
Requirement IDs: COMMUNITY-01, COMMUNITY-03

# Member reports and group-moderator review

Implement the acceptance contract in `../spec.md` through the existing
community API and Community page. Keep reports private, decisions human, and
removed posts as visible tombstones. Author HTTP integration and browser
coverage; do not run the deferred final verification batch.

- [x] Add migration and both API route prefixes.
- [x] Add private member reporting and own-status endpoints.
- [x] Add group-moderator queue, audited decisions, and atomic tombstones.
- [x] Restrict feed removal controls to moderators; add report and review UI.
- [x] Add API integration and Playwright coverage.
- [ ] Run the final CI verification batch after all implementation slices.

## Implementation record

Added an idempotent report migration, member-only reporting/status routes,
group-moderator queue and transactional dismiss/remove decisions. Feed removal
now tombstones posts and resolves open reports in the same transaction, with
moderator actions recorded in the audit log. The Community page gates feed
actions by membership/role, shows private reporter status, and exposes the
moderator queue. Integration and browser cases cover privacy, authorization,
duplicate/invalid reports, member joins, and both resolution outcomes. Per the
requested batch order, no build, test, or CI run has been performed.
