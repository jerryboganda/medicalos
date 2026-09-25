# AI-18 — Pre-generated one-tap tutoring

Status: ready-for-agent
Requirement IDs: AI-18, OFF-01, EX-06
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§9.5, 19.4, 22
Implementation state: API/browser/pack CI acceptance passed in run 36159484978; production signing-secret setup and clinical reviewer sampling remain external.

## Domain rule

One-tap tutoring cards are deterministic, cached derivatives of a specific
published question version. They do not call a live model. A later question
version gets its own cards; historical cards remain tied to their source
version. Tutoring is available only where the question and session permit AI
assistance. Timed assessments and reserved questions with `ai_allowed = false`
must not expose cards.

## Acceptance

- Publishing an approved version ensures the five standard card types exist
  once for that version; repeated generation returns the existing cache.
- Existing published versions can be filled idempotently when a tutor session
  or offline pack first requests them.
- A tutor session returns cards only for eligible questions. The session UI
  offers one-tap display after the learner answers and retains the cards in
  its local session cache so the same help remains available offline.
- The v2 signed pack manifest requires a paid, active, device-bound lease and
  covers exact cached tutoring-card content in its signature. It includes cards
  only after that learner has answered the matching question in tutor mode;
  timed-only and reserved questions that prohibit AI do not include cards.
- The v1 metadata-only manifest contract remains compatible and never returns
  tutoring cards.
- API startup rejects a missing or weak `PACK_SIGNING_KEY`; production deploy
  passes a private value from `VPS_PACK_SIGNING_KEY` without logging it.
- Generated cards are grounded in reviewed question fields. Medical reviewer
  sampling is recorded as an external acceptance requirement rather than
  claimed from code or fixtures.

## Verification boundary

Author API coverage for publication, idempotent caching, ineligible content,
and signed manifest contents, plus browser coverage for one-tap and cached
offline display. Do not run local builds, tests, screenshots, or generated
artifacts; final GitHub Actions is the acceptance gate. Medical reviewer
sampling remains pending until real reviewer evidence is recorded.

## Implementation record

- Cache creation is deterministic and idempotent, runs in the publish
  transaction, and repairs older published versions when tutoring or a pack
  first needs them.
- Answered tutor feedback and session reloads return source-linked cards;
  timed sessions and AI-restricted reserved questions do not expose them.
- The signed pack manifest contains the cards and hashes their exact content.
- The session UI offers the five one-tap actions and keeps received cards in
  the local session draft. API/browser/pack coverage passed in GitHub Actions
  run 36159484978; medical reviewer sampling remains an external gate.
- The v1 metadata-only contract is preserved. The v2 manifest is lease-,
  device-, and answered-tutor-scoped, and its regression test recomputes the
  exact item hash and HMAC.
- Runtime signing now fails closed without a non-whitespace key of at least 32
  bytes. Production CI requires the owner-configured `VPS_PACK_SIGNING_KEY`;
  no deployment has been run in this continuation.
