# Competition learner flow

Requirement IDs: COMP-01, COMP-02, COMMUNITY-03
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §17.1

## Problem statement

Learners can opt in to a community handle and competitions can be created, but
the Practice page does not list competitions or provide an entry flow. The
current API accepts all answers at once and trusts client-reported answer time,
so a modified client can claim an instant response and gain the speed bonus.

## Solution

Add a competition section to Practice. Learners can inspect event windows,
start or resume one attempt, answer a single randomized question at a time, and
see the final score. The server snapshots the attempt order, measures each
answer interval, validates each response, and scores it from the stored
question version. The page reuses the locked Midnight app tokens and routes
learners without a community handle to the existing opt-in flow.

## User stories

1. As a learner, I want to see available competition windows in Practice, so
   that I can choose an event before it closes.
2. As a learner, I want open, upcoming, and ended states to use the event's
   timestamps, so that stale scheduler status does not mislead me.
3. As a learner, I want a clear opt-in path when I have no community handle,
   so that competition participation remains voluntary.
4. As a learner, I want to start one attempt per event, so that my result is
   not replaced by repeated submissions.
5. As a learner, I want an in-progress attempt to resume at its current
   question after a reload, so that a lost response does not restart the event.
6. As a learner, I want one question at a time with no answer key or rationale,
   so that future content stays unreleased while the event is open.
7. As a learner, I want question and option order randomized for my attempt,
   so that participants do not all receive an identical sequence.
8. As a learner, I want the server to measure response time, so that a modified
   client cannot claim an instant speed bonus.
9. As a learner, I want the next question to appear only after I answer the
   current question, so that the entry is one continuous sitting.
10. As a learner, I want a clear result after the last response, so that I can
    understand whether my entry was recorded.
11. As a learner, I want a retried answer request to return the same result,
    so that a network retry cannot skip a question or create a second entry.
12. As a learner, I want leaderboard metrics and my own rank after submitting,
    so that I can review my performance without exposing my email or profile.

## Implementation decisions

- Use the existing authenticated competition list and leaderboard routes.
- Add a start/resume endpoint and a per-question answer endpoint. A start
  captures the learner's current community handle and creates one unique
  attempt for the learner and competition.
- Snapshot randomized question order and the displayed-option-to-version
  mapping on the attempt. Expose only question text, lead-in, and option text.
- Measure elapsed time from the server's stored question start timestamp. The
  answer contract rejects unrecognized fields, including client time claims.
- Record the answer, idempotency key, next state, and final leaderboard entry
  transactionally. A start after submission returns the existing one-attempt
  conflict; an answer retry with the same key returns its stored response.
- The Practice section distinguishes opt-in, upcoming, open, in-progress,
  submitted, and closed states and uses the existing Community route for
  handle setup.
- Keep event creation controls in the Editorial Console. Live sockets,
  anomaly detection, league/group filters, and prize administration remain
  separate slices.

## Testing decisions

- Exercise public HTTP seams for event states, opt-in gating, one-attempt
  enforcement, randomized question delivery without answer leakage, invalid
  sequence/choice rejection, server-timed scoring, request replay, and
  leaderboard output.
- Exercise the Practice page through mocked HTTP responses for listing,
  opt-in navigation, one-question progression, completion, and visible errors.
- Do not run local tests or builds; batch them in GitHub Actions after the
  implementation slices are complete.

## Out of scope

Competition creation, live WebSocket scheduling, anomaly detection, prizes,
leagues, private challenges, and offline competition attempts.
