# AI-04 — Time-budgeted next action

Status: ready-for-agent
Requirement IDs: AI-04, AI-02, AI-08, PLAN-02
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§8.4–8.8
Implementation state: current-plan selection, protected work, activity/time preferences, free-tier caps and exam-date states are implemented; content rights, verified offline availability, broad accessibility metadata and CI remain.

## Problem statement

Learners need one useful next step when they have limited time. The current
Today plan has task durations, but it does not select a task that fits the
learner's stated time or explain the evidence behind that choice.

## Solution

Add a read-only recommendation over the learner's current pending plan tasks.
The learner supplies available minutes, an activity preference, and an optional
time adjustment. Only launchable tasks whose adjusted estimate fits are
eligible. Protected tasks rank first, followed by eligible revision tasks,
observed independent chapter accuracy after the ten-attempt evidence floor,
and current plan order when evidence is sparse. Return one recommendation with
its estimate, question count, reason, and evidence count, or a stable no-fit
result. Do not mutate the plan, infer exam success, or create additional work.

## Acceptance

- A learner can request one next action for a 5–480 minute budget; invalid
  budgets return a validation error.
- The recommendation is selected only from that learner's pending current-plan
  tasks whose adjusted `estimated_minutes` fit the supplied budget and whose
  session entitlement is currently available. A missing plan returns
  `no_current_plan` without creating plan rows.
- A task is eligible only when the same published, quarantine and reserved-item
  rules used at launch can supply the exact planned question count. If fitting
  work exists but no task has enough available questions, return
  `content_unavailable`.
- Fitting protected tasks outrank other candidates and report
  `protected_task`; an explicit activity preference filters other candidates.
- The learner may apply a 1–4× time adjustment for reading or interaction
  needs. The adjusted estimate must fit the supplied budget.
- A passed active exam date returns no task with `exam_deadline_passed`; an
  upcoming date is included in the response.
- Free-tier practice uses the configured daily allowance; paid-tier practice
  does not use that free allowance. Revision attempts do not consume it.
- New free-tier attempts are capped atomically across concurrent sessions;
  idempotent replays do not consume another question. A linked task is offered
  and launched only when its full planned question count fits the remaining
  allowance.
- A recent missed-question revision ranks ahead of general practice when both
  fit and no fitting task is protected. With at least ten independent chapter
  attempts, lower observed accuracy is preferred among general-practice
  candidates; below that evidence floor, selection keeps existing plan order.
- The result reports the task, estimate, question count, a neutral reason code,
  and independent evidence count without returning a predicted score or
  ability.
- If no task fits, the API returns no task and a stable reason code. Calling
  the recommendation does not change plan version, tasks, or revision history.
- When the daily question allowance blocks every fitting practice task, the
  response identifies that entitlement reason instead of presenting the task
  as startable.
- Today lets the learner request a recommendation using the available-minutes
  field and clearly presents the action or no-fit state.

## Remaining master-plan hard constraints

The data model cannot establish whether each question's source license allows
this learner's tier to use it: `content_rights` has no resource-version link,
and `question_versions` has no rights reference. Offline leases prove server
entitlement and device binding, but the client has no downloaded-pack receipt
for this picker to verify. `learner_accommodations` has no shared capability
vocabulary, so this slice accepts a per-request time adjustment; format and
modality compatibility remain unverified. These are explicit platform gates,
not inferred eligibility. AI-04 stays in progress until those links and their
API/device acceptance are implemented.

## Verification seam

Use the authenticated API boundary for budget validation, eligibility, ranking,
evidence-floor behavior, empty results, and absence of plan mutation. Use the
existing Today browser flow to verify that the learner's minutes are sent and
the selected action is shown. Do not run local builds, tests, screenshots, or
generated artifacts; final GitHub Actions is the acceptance gate.

## Implementation record

- The read-only endpoint prioritizes protected tasks, then missed-question
  revisions, then the lowest observed chapter accuracy after ten independent
  attempts, then current plan order under sparse evidence. A missing plan is
  not created by a recommendation request.
- Today exposes activity and time-adjustment controls with truthful no-fit,
  allowance, preference, deadline and missing-plan states. The plan is not
  modified and no predictive score is returned.
- Pool counts mirror the linked session launch filters, so quarantined or
  reserved content and revision sources with too few eligible questions are
  not recommended.
- Only free-tier practice uses the free daily cap, and revision attempts are
  excluded from its usage count. Answer inserts serialize on the learner row
  and enforce the cap at the write boundary; next-action and linked task launch
  check that the complete planned count fits. API integration and browser
  regression cases are authored; final GitHub Actions verification is pending.
