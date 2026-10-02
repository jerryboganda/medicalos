# Browser handoff and deep-link picker

Status: ready-for-agent

## Outcome

Complete the owner-approved LTI browser handoff and published-article deep-link picker described in `../spec.md`.

## Acceptance

- A browser form-post to `/v1/lti/launch` restores the verified app session without putting the token in the URL; non-browser callers continue to receive JSON.
- Resource-link launches open the existing article reader for a valid `medicalos_article_slug`, with a safe fallback when the resource is unknown.
- The picker searches published editorial articles only, enforces the platform's content type and selection limits, and returns the platform-signed response to its server-parked return URL.
- HTTPS LMS framing works only for LTI handoff/picker paths; app-wide frame denial remains in force elsewhere.
- Playwright covers learner continuation, picker search/selection, and the LMS return form; focused backend tests cover response negotiation and script-safe handoff output.
- INST-06 traceability is updated with the final CI evidence and external certification boundary.

## Comments

- Owner approval recorded in `.scratch/owner-inputs-requested.md` on 2026-09-30: LTI launch handoff and deep-link picker UI, design-system only.
