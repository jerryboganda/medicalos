# CORE-07 — Personal account security and session control

## Problem Statement

Medical OS currently has a minimal email/password login with one long-lived bearer token. It has no email-verification lifecycle, password recovery, device/session inventory, server-side logout, device cap, account-deletion initiation, or single-active-study-session enforcement. The current client also signs out only by deleting its local token.

The Phase 1 account requirement needs a secure personal-account foundation without collapsing account sessions into institution tenancy or learner study sessions. It must also avoid pretending that Google, Apple, or email delivery are integrated when this repository has no configured identity/email adapter or credentials.

## Solution

Replace the single long-lived login token with short-lived access tokens backed by rotating refresh tokens and durable device sessions. Add email-verification and password-reset challenge lifecycles, authenticated session inventory/revocation, a two-device limit, in-app account-deletion initiation, and explicit takeover for the one-active-study-session rule.

The browser client will use a stable local device identifier, refresh-capable auth storage, server-backed sign-out, and a compact Account page built from the existing Medical OS design system.

Provider sign-in and real email delivery remain explicitly incomplete until real adapters and credentials exist; no fake provider buttons or secret-echo production behavior will be shipped.

## User Stories

1. As a learner, I want registration to require email verification before login so that ownership of the address is confirmed.
2. As a learner, I want verification challenges to expire and be single-use so that old links cannot be replayed indefinitely.
3. As a learner, I want to request a password reset without revealing whether an email is registered so that account discovery is not leaked.
4. As a learner, I want a reset challenge to expire and be single-use so that a captured old reset link cannot be reused.
5. As a learner, I want a successful reset to invalidate existing sessions so that a credential reset also closes potentially compromised access.
6. As a learner, I want short-lived access tokens so that a stolen bearer credential has a bounded lifetime.
7. As a learner, I want refresh tokens to rotate when used so that a replayed old refresh token is rejected.
8. As a learner, I want my device session to have a stable identifier and human-readable name so that I can recognize it.
9. As a learner, I want at most two active devices so that routine account sharing is constrained.
10. As a learner, I want a third device login refused without silently evicting an existing device so that I remain in control of active devices.
11. As a learner, I want to list my active sessions and see which one is current so that device access is transparent.
12. As a learner, I want to sign out the current device on the server so that clearing local storage is not the security boundary.
13. As a learner, I want to sign out other devices so that I can recover control of my account.
14. As a learner, I want an existing device to log in again without consuming another device slot so that token renewal does not create phantom devices.
15. As a learner, I want account deletion to be startable from the app so that I do not need support to initiate it.
16. As a learner, I want deletion initiation to revoke active sessions and block new login while deletion is pending so that the request has immediate account-security effect.
17. As a learner, I want only one open study session at a time so that two devices cannot silently create competing active attempts.
18. As a learner, I want the API to tell me which study session is already active so that the client can offer an explicit takeover choice.
19. As a learner, I want explicit takeover to close the prior open study session and create the requested one so that the transition is deliberate.
20. As an institution member, I want all of these account controls to remain personal-account scoped so that tenant membership does not change identity-session semantics.

## Implementation Decisions

- Keep authentication personal-account scoped; tenant memberships and platform roles remain separate authorization concepts.
- Add an additive account migration rather than rewriting the initial migration.
- A durable auth session owns a stable session identifier, device identifier/name, hashed access token, hashed refresh token, access expiry, refresh expiry, last-seen timestamp, and optional revocation timestamp.
- Access tokens are opaque random values, stored only as SHA-256 hashes server-side, and expire after 15 minutes.
- Refresh tokens are opaque random values, stored only as SHA-256 hashes server-side, expire after 30 days, and rotate atomically on every refresh.
- Login accepts a client-generated stable device identifier and a bounded display name. Re-login from the same device replaces that device's existing session rather than consuming another slot.
- A personal account may have at most two non-revoked, refresh-valid device sessions. A third distinct device receives a conflict response; the server does not silently evict another device.
- Registration creates a hashed, expiring email-verification challenge. Login is denied until verification completes.
- Password-reset request is enumeration-safe and creates a hashed, expiring reset challenge only for an eligible account.
- Production responses never expose raw verification or reset secrets. The integration-test AppState may expose them so the existing HTTP seam can exercise the lifecycle end-to-end; the production binary hard-codes that capability off.
- Password reset updates the Argon2 password hash and revokes all active sessions.
- Account-deletion initiation records a pending deletion timestamp, revokes active sessions, and blocks subsequent login. Actual purge/retention propagation belongs to TRUST-02 and is not claimed by CORE-07.
- `GET /v1/me/sessions` returns active device sessions and marks the caller's current session.
- Current logout revokes only the caller's durable session. Sign-out-other-devices revokes every other active session for the same user.
- `AuthUser` carries both user and auth-session identifiers so authenticated account routes can target the current session without accepting it from the client.
- Practice-session creation rejects a second open study session with a conflict that identifies the active session. A `takeover: true` request atomically marks prior open sessions abandoned before creating the new one.
- `open`, `submitted`, and `abandoned` are the study-session lifecycle states used by this slice.
- The account UI reuses existing tokens/classes. No new UI framework, state library, auth SDK, or design-system abstraction is introduced.
- Google and Apple sign-in are not represented as working until a real provider adapter plus credentials exist. Their absence keeps CORE-07 overall status partial even when the local account foundation is accepted.

## Testing Decisions

- The primary acceptance seam is the existing HTTP API integration suite.
- Account lifecycle coverage uses only HTTP responses for behavior. Test-only secret echo is enabled through AppState solely to simulate receipt of an external verification/reset message without adding a fake production mail provider.
- Cover: unverified-login rejection, verification, two-device login, third-device refusal, session listing, refresh rotation/replay rejection, sign-out-other-devices, current logout, password reset with session invalidation, and deletion initiation.
- Cover the study-session rule through `POST /v1/practice/sessions`: second-open conflict and explicit takeover.
- Existing practice tests that intentionally create another open session must opt into takeover or submit the first session; the production default remains fail-closed.
- Migration up/down/up remains the schema rollback gate.
- Client behavior is verified through the existing browser E2E seam after the backend contract is green.
- Heavy compilation, database tests, browser tests, and builds run only in GitHub Actions. Local work is limited to formatting, diff, and static inspection.

## Out of Scope

- Google sign-in until a real Google identity configuration/adaptor is supplied.
- Sign in with Apple until a real Apple identity configuration/adaptor is supplied.
- Phone OTP.
- Institutional SSO/SCIM.
- Biometric app unlock and native secure-keychain storage.
- Privileged-role MFA, which is a broader section-25 security slice.
- Final account purge, retention scheduling, export, search-index deletion, and queued-job propagation; those belong to TRUST-02.
- Device attestation, encrypted offline packs, watermarking, and anti-scraping limits.
- Deployment.

## Further Notes

- This slice closes the server/client foundation of CORE-07 but must remain `in-progress` in traceability until real required Google and Apple sign-in adapters are implemented and verified.
- The two-device account limit and one-active-study-session rule are intentionally separate mechanisms with separate identifiers and lifecycles.
- Email challenge delivery is an adapter boundary, not an excuse to weaken the token lifecycle. The backend stores only challenge hashes and never exposes raw challenge secrets in the production AppState.
