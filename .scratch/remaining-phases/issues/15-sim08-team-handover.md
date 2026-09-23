# SIM-08 — Team simulation roles and handover

Status: ready-for-human
Requirement IDs: SIM-08, SIM-02, TRUST-01
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§14.2–14.4

## Problem Statement

Scenario runs are private to the learner who starts them. Learners cannot assign team roles, collaborate on the same authored case, or record a handover between participants.

## Solution

Let the run owner invite a small team into an active fictional case. Record the participant role with each authored action. Participants can hand over the case using a structured situation/background/assessment/recommendation note, and the recipient can acknowledge it. Keep the run transcript authoritative and the handover private to the invited team.

## User Stories

1. As a team lead, I want to invite participants with explicit roles so that each person knows their task in the case.
2. As an invitee, I want a single-use invitation code so that joining a team does not require exposing another learner’s account details.
3. As a team member, I want to see the same pinned scenario, current state, transcript, and debrief so that the team shares one case history.
4. As a team member, I want actions attributed to my assigned role so that the transcript shows who contributed without disclosing account identifiers.
5. As an observer, I want to follow the case without advancing it so that observation remains distinct from participation.
6. As a team member, I want to hand over the case in a structured format so that the next role receives a concise, accountable summary.
7. As the handover recipient, I want to acknowledge receipt so that the team can distinguish an offered handover from one accepted by its recipient.
8. As an operator, I want participant access bounded to one invited run and its version so that collaboration does not expose unrelated learner records.
9. As a learner, I want simulation pages to state that the case is fictional and formative so that team activity is not mistaken for clinical authorization.

## Implementation Decisions

- Use the existing authenticated scenario-run and SvelteKit interfaces.
- The run owner is the team lead. Allow up to five additional members with `history_taker`, `scribe`, or `observer` roles; only an observer is barred from advancing the case.
- Store team membership and one-use invitation records in additive tables. Invitations contain a random code whose digest is stored, expire after 24 hours, and cannot be used after the case finishes.
- Scope run reads and counterfactual replay to the owner or an accepted team member. Keep appeal submission restricted to the run owner.
- Include the acting member’s role in new transcript events and in the normalized timeline; do not expose user IDs or email addresses.
- Store SBAR handovers and recipient acknowledgements as append-only rows. A handover can target only a different member of the same active team.
- Bound every role, invitation, handover text field, list response, and team size. Audit identifiers and role/action codes only; omit handover prose and invitation codes.
- Keep the same pinned scenario state machine, deterministic transitions, immutable examiner evidence, and independent appeal rules.
- Expose team/invite/join/handover actions inside the existing scenario run flow and a small invite-code join route.

## Testing Decisions

- Use the authenticated HTTP integration seam to cover owner/team access, single-use invitations, role restrictions, attributed actions, handover recipient scope, acknowledgements, finished-run refusal, and private narratives in audit output.
- Use Playwright to cover the invite/join path, shared run state, observer behavior, one structured handover, and narrow viewport overflow.
- Follow the existing scenario integration and Playwright flows. Keep build and test execution deferred to the final GitHub Actions pass.

## Out of Scope

- Video/voice conferencing, real-patient data, institutional grading, clinical sign-off, patient-voice models, and clinical validation.
- Team membership discovery by email, public links, cross-run access, team chat, and mutable handover editing.

## Further Notes

The transcript and handovers are simulation learning records. They do not establish clinical competence or authorize patient care.

## Implementation Record

- Added run-scoped team membership, single-use hashed invitations, four explicit roles, role-attributed transcript events, and shared owner/member access to the pinned case and debrief.
- Added structured SBAR handovers with recipient-only one-time acknowledgement. Team memberships, handovers, and acknowledgements are immutable; invite acceptance is a one-time database transition.
- Added the team panel and invite-code join page with observer action limits, narrow-screen coverage, and clear simulation-only language.
- Added API integration and Playwright coverage for invitation, access, role, action, handover, acknowledgement, and audit-privacy behavior. No builds or tests have been run; final GitHub Actions acceptance remains pending.
