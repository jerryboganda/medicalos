# Enforce device registration for authenticated API sessions

Status: ready-for-agent

## Outcome

Protected API handlers reject unbound sessions and sessions whose bound device is no longer active. Explicit account-recovery handlers continue to accept valid unbound sessions so learners can recover from the device limit.

## Acceptance criteria

- A direct unbound bearer cannot read study data through either `/v1` or `/api/v1`.
- The explicit recovery handlers remain usable from an unbound bearer.
- Registering the session to an active device enables protected access.
- A bound bearer cannot access protected routes after its device is revoked.
- Password, OIDC, and platform-role test sessions are bound in ordinary integration fixtures; targeted recovery tests retain unbound sessions.
- Rust integration tests and SQLx offline compilation pass in GitHub Actions.

## Spec

See `../spec.md`.

## Comments
