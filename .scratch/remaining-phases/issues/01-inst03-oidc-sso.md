Status: ready-for-human
Requirements: INST-03, CORE-01, CORE-07
Implementation state: API/browser CI verified in run 36159484978; provider certification and production secret provisioning remain external.

# Institution OIDC sign-in and scoped enrollment

## Problem

Institutions can pre-bind an external subject to an existing learner account, but learners cannot authenticate through their institution. Email matching would risk linking the wrong account, and a plain provider assertion must not be trusted without verifying the OIDC protocol.

## User stories

1. As an institution learner, I want to authenticate through my institution's OpenID Connect provider so that I can use my existing institutional identity.
2. As an institution operator, I want to configure an issuer and client for an institution so that its login uses the correct identity provider.
3. As an operator, I want to choose whether the provider is enabled and rotate its client credentials so that misconfiguration can be disabled or repaired.
4. As an operator, I want to bind a verified issuer and subject to an existing learner account so that authentication never creates an account or matches by email.
5. As a learner, I want a successful SSO login to create the same revocable application session as password login.
6. As a learner, I want single-active-session policy to apply to SSO sessions as well as password sessions.
7. As an institution, I want issuer and subject mappings isolated by institution so that another tenant cannot reuse a mapping.
8. As an operator, I want failed, expired, mismatched, replayed, and unprovisioned logins to fail closed without exposing provider tokens or account existence.
9. As an operator, I want provider configuration changes and SSO enrollment records audited without recording client secrets, authorization codes, or raw login tokens.
10. As a learner, I want the browser to exchange a short-lived, one-use handoff ticket for the normal session token so that bearer credentials never appear in the callback query string.

## Implementation decisions

- Use the existing Axum API and SvelteKit login page as the only public seams.
- Implement OIDC Authorization Code with PKCE, `openid` scope, random state and nonce, strict issuer/audience/nonce verification, and a no-redirect async HTTP client with a short timeout.
- Require HTTPS issuer and callback URLs, allowing HTTP only for literal loopback hosts used in local development.
- Configure providers only through the existing global admin-token boundary; derive callback URLs from operator-configured public API and app URLs and never accept a return URL from the browser.
- Encrypt optional client secrets at rest with an application-provided OIDC credential key. Never return or audit the secret.
- Store short-lived login state and one-use handoff tickets as hashes where possible; consume each atomically.
- Resolve identity only by `(institution_id, exact issuer, verified sub)` against a pre-existing external identity mapping. Never auto-create users or link by email.
- Issue the existing application session through shared auth logic, including account deletion and single-active-session checks.
- Keep provider certification, tenant-specific registration with third-party IdPs, and production secret provisioning outside code acceptance.

## Testing decisions

- Use the already accepted HTTP integration seam in `apps/api/tests/integration.rs` for admin authorization, secret redaction, PKCE/state creation, callback failure/replay behavior, exact scoped mapping, deleted/unmapped user refusal, and one-use session completion.
- Use the existing Playwright browser seam for the login entry and callback handoff behavior.
- Use a local loopback OIDC fixture with a generated test signing key for protocol-level callback verification; assert externally visible redirects and normal session behavior.
- Do not run builds, tests, or browser screenshots until the remaining feature implementation is complete, as requested.

## Out of scope

- SAML, magic links, email-based account linking, automatic learner provisioning, provider-side tenant application registration, and third-party certification.
- Deploying provider secrets or activating SSO in production.

## Acceptance

- A configured provider redirects with Authorization Code + PKCE and persists expiring one-use state.
- Callback rejects absent, expired, mismatched, replayed, or invalidly signed assertions and only accepts the exact configured issuer, audience, nonce, and a pre-bound subject.
- A valid mapped learner receives the same session behavior as password login through a short-lived one-use ticket.
- Staff, users from other institutions, deleted users, and unconfigured providers cannot authenticate through the route.
- Admin configuration is tenant-addressed, secret-safe, audited, and disabled by default.
