# Competition schedules and weekly leagues

Requirements: COMP-01, COMP-03
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §17.1

## Problem statement

Competition records accept a cadence, but daily, weekly, and monthly records do
not produce later events. Learners also have no weekly league cohorts or
promotion/relegation path, although private duels already exist.

## Solution

Treat a recurring competition as an admin-created series. Materialize its next
UTC event from the existing competition-list request under a bounded,
row-locked transaction. Keep each event independent, same-exam, randomized,
and scored with the series' saved policy.

Add opt-in exam leagues to Practice. A league has Monday-to-Monday UTC weeks,
cohorts of at most 30 learners, and a stable handle roster. New members start
in Division 1. When a learner's next week is materialized, rank the prior
cohort from completed same-exam event scores; promote the top three and
relegate the bottom three when the cohort has at least 10 members. Rank by
points, accuracy, total time, then stable user ID. Division 1 has no lower
division. Learners can leave at any time.

## User stories

1. As an administrator, I want to create daily, weekly, or monthly competition
   series so recurring events do not require manual recreation.
2. As a learner, I want each scheduled competition to be its own timed entry
   so prior answers and attempts do not carry into the next event.
3. As a learner, I want recurring events to use questions from the approved
   series pool, preferring questions not used by the immediately prior event.
4. As a learner, I want the event cadence and UTC window visible in Practice
   so I can distinguish daily, weekly, monthly, live, and one-off events.
5. As a learner, I want to explicitly join a same-exam weekly league so my
   handle is ranked only after I opt in.
6. As a league member, I want to see only my cohort's standings so my
   participation is limited to the group I joined.
7. As a league member, I want promotion and relegation based on recorded
   competition results so division changes reflect completed work.
8. As a learner, I want to leave the league so optional rankings remain under
   my control.

## Implementation decisions

- Reuse the authenticated competition list as the bounded recurring-event
  materialization trigger; lock series rows and persist a unique occurrence
  start to make concurrent requests idempotent. A request processes at most
  ten series and three due occurrences per series.
- Daily and weekly schedules advance by UTC day/week. Monthly schedules advance
  by one calendar month, clamping at the month's last day when needed.
- New occurrences select the configured question count from the series pool
  and copy the difficulty-point snapshot. Choose published same-exam questions
  and avoid the preceding event's items when possible; skip an occurrence if
  fewer than three eligible questions remain.
- Live and one-off events remain single events. Existing series remain
  independent from one-off event creation.
- League participation is a durable per-exam opt-in roster. Weekly cohort
  snapshots retain handle, division, and exam; each cohort holds at most 30.
- Only completed entries from same-exam competitions whose finish and
  submission fall within the UTC league week count. Learners without a score
  stay in their division without a ranking; ties use accuracy, time, then
  stable user ID.
- A cohort needs at least 10 active members before its top/bottom three move.
  Promotions increase the division number; relegations decrease it but never
  pass below Division 1. Division 1 is the entry tier and floor; new opt-ins
  start there.
- Materialize up to 31 missing league-week memberships per request under an
  exam-row lock. Carry a learner's full prior cohort together and apply those
  results exactly once; the requesting opted-in learner is included in the
  batch. Reads by nonmembers do not materialize other learners' records.
- Joining requires the learner's own opt-in community profile. League
  standings are visible only inside the learner's own cohort and show handles,
  not user IDs or account details.

## Testing decisions

- API integration coverage exercises daily occurrence creation and
  idempotency, scoring snapshots, question rotation, league opt-in/leave,
  cohort capacity, promotion/relegation, ranking privacy, and cross-cohort
  access. Monthly calendar clamping has focused unit coverage.
- Playwright coverage exercises cadence labels, league join/standings, and
  leaving from the Practice page.
- Author coverage during implementation; defer builds, tests, and GitHub
  Actions until the final implementation batch.

## Out of scope

Group quizzes and shared decks, country/institution/friends leaderboard
filters, native push delivery, prize fulfillment, and provider or clinical
certification.

## Further notes

Competition series are created through the existing admin-authenticated
competition endpoint. The learner never joins a league implicitly: only a
prior explicit join keeps their roster active in later weeks.
