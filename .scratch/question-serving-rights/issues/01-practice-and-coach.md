# Enforce current display rights in practice and Coach

Status: complete (full exact-source CI run 36878420626 on 86723609f363f7844b35d25171170e38a5db9c3d passed)

## Outcome

Practice and Coach stop serving question content once the question's current
display grant becomes unavailable, even when the question was published or a
session/Coach response was previously created.

## Acceptance criteria

- New tutor, timed, blueprint, revision, retry, and incorrect-question pools
  include only questions that currently pass the shared display-rights rule.
- Session detail, hints, answer feedback, answer idempotency replay, Coach
  turns, Coach history, and Coach idempotency replay refuse unavailable
  question content with a stable `rights_unavailable` error.
- Offline answer synchronization uses the same check and cannot replay
  feedback for a question whose grant is no longer active.
- Rights checks happen before returning stored/idempotent responses.
- A rights-denied response contains no question text, options, hint, answer,
  rationale, key-learning point, or Coach answer.
- A started session can still be submitted for aggregate learner evidence;
  persisted attempts/results are not deleted by rights revocation.
- Missing, revoked, expired, wrong-audience, unsupported seat-limited, and
  out-of-scope grants fail closed; active learner grants continue to work.
- Integration coverage exercises public routes, not only the SQL helper.
- Rust, offline SQLx, and full browser acceptance run in GitHub Actions on the
  exact committed source.

## Spec

See `../spec.md`.

## Comments
- 2026-10-01 — Public-route rights regressions for pools, sessions, answer replay, sync, Coach, and preserved aggregate results passed in the full exact-source GitHub Actions run above.
