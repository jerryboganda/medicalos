# Community post reporting

Requirement: COMMUNITY-01
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §17 / Community-01 traceability gap

## Problem

Group members can post and moderators can remove posts, but members have no
way to report a post for moderator review. The feed currently shows a Remove
button to every member even though the API correctly restricts removal to
moderators.

## User stories

1. As a group member, I want to report a visible post with a reason and an
   optional private note so moderators can review concerns without exposing
   my report to other members.
2. As a reporter, I want to see whether my report is open, dismissed, or tied
   to a removed post so I know what happened without seeing other reports.
3. As a group moderator, I want a private queue of open reports for my group
   so I can decide each case with the relevant post context.
4. As a group moderator, I want to dismiss a report or remove its post so the
   decision is explicit and auditable.
5. As a group member, I want removed content to appear as a tombstone and
   moderation controls to match my role so group actions are understandable.

## Implementation decisions

- A group member can report a visible post once with a reason (`spam`,
  `harassment`, `medical_misinformation`, or `other`) and an optional note of
  at most 500 characters.
- Reports are private to their reporter and the moderators of that group.
  Acknowledgement returns the report ID, status, and creation time; reporters
  can view only their own report status.
- Group moderators get an open-report queue scoped to their group. They can
  dismiss a report or remove its post. Removing a post keeps the existing
  tombstone behavior and closes every open report for that post as
  `post_removed`.
- Every moderator decision is recorded in `audit_events`. Reports never
  automatically hide content; human moderators make every decision.
- The feed exposes moderator capability so the UI only renders removal and
  queue controls for moderators.
- Durable reports use an additive table with one report per member and post.
- Report creation and moderation serialize on the post row; removal and
  resolution update the tombstone and affected open reports in one transaction.
- Both API prefixes retain the same authenticated-user and group-role rules.

## Testing decisions

- Exercise membership, privacy, idempotency, authorization, resolution, and
  audit behavior through authenticated HTTP integration tests.
- Exercise member reporting and moderator review actions through the existing
  Playwright Community page seam.
- Author the test coverage now; defer local and GitHub Actions execution until
  the final implementation batch.

## Acceptance

- Members can report visible posts, but cannot report posts in groups they
  have not joined or reports for removed/missing posts.
- Repeated reports by the same member for the same post fail without replacing
  the original reason or note.
- Non-moderators cannot read a group's report queue or resolve reports.
- Dismissal leaves the post visible. Removal records the tombstone and closes
  the post's remaining open reports atomically.
- Report notes and other members' report status never appear in the public
  feed or a reporter's own report list.
- The Community page lets members report and moderators review, dismiss, or
  remove. Existing non-moderator controls no longer expose removal actions.
- API integration and browser coverage are authored; final Actions verification
  remains deferred until all implementation slices are complete.

## Out of scope

Automated moderation, report-count thresholds, platform-wide staff review,
notifications, and clinical adjudication of reported medical content.

## Further notes

Only a group's moderators can see report notes. Public feeds and reporters'
own status lists do not include notes or other reporters' identities.
