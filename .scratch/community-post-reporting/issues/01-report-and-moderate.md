Status: in-progress
Type: task
Requirement IDs: COMMUNITY-01, COMMUNITY-03

# Member reports and group-moderator review

Implement the acceptance contract in `../spec.md` through the existing
community API and Community page. Keep reports private, decisions human, and
removed posts as visible tombstones. HTTP integration and browser coverage is
verified in GitHub Actions run 36159484978.

- [x] Add migration and both API route prefixes.
- [x] Add private member reporting and own-status endpoints.
- [x] Add group-moderator queue, audited decisions, and atomic tombstones.
- [x] Restrict feed removal controls to moderators; add report and review UI.
- [x] Add API integration and Playwright coverage.
- [x] Run the final CI verification batch after all implementation slices (run 36159484978).

## Implementation record

Added an idempotent report migration, member-only reporting/status routes,
group-moderator queue and transactional dismiss/remove decisions. Feed removal
now tombstones posts and resolves open reports in the same transaction, with
moderator actions recorded in the audit log. The Community page gates feed
actions by membership/role, shows private reporter status, and exposes the
moderator queue. Integration and browser cases cover privacy, authorization,
duplicate/invalid reports, member joins, and both resolution outcomes. GitHub
Actions run [36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978)
passed on source commit `a885734c5b28344254a68e6881c13bdee29fce6e`.
