## Problem Statement

The browser registers its device before loading study features, but the API accepts a valid bearer directly even when its session has no registered device. A caller can therefore bypass the client gate and use protected study endpoints.

## Solution

Enforce active device registration in the shared authenticated-session extractor. Keep only the established account recovery controls available to a valid unbound session, so a learner can register a device, free a device slot, change session policy, export data, or close the account after reaching the device limit.

## User Stories

1. As a learner signing in on a new browser, I want protected study requests refused until device registration succeeds, so direct API calls obey the same rule as the browser.
2. As a learner at my device limit, I want to list and revoke a device from a fresh session, so I can recover access without support.
3. As a learner at my device limit, I want to change my single-session policy, export my data, or close my account, so device registration does not trap me inside my account.
4. As a learner, I want a session bound to a revoked or missing device to be refused, so stale device records cannot keep a bearer active.
5. As a learner, I want successful registration to unlock protected routes, so the recovery step has a clear result.
6. As an API client, I want the `/v1` and `/api/v1` route trees to apply the same device-binding rule, so path choice cannot bypass it.

## Implementation Decisions

- Enforce registration in the shared `AuthUser` HTTP extractor, not in individual study handlers or only in the browser.
- Add a distinct account-recovery extractor that permits an unbound session only for device registration/list/revocation, account deletion/export, and session-policy read/write handlers.
- Require any already-bound session to reference an active device row owned by the same user. A revoked or missing device remains unauthorized, including on recovery handlers.
- Keep admin password reset and all study, course, institutional, and platform handlers on the strict default extractor.
- Preserve both public route prefixes and the existing account page behavior and visual patterns. This slice changes no UI.
- Do not claim device attestation: the existing device key is a client-provided identifier.

## Testing Decisions

- Test through the public authenticated HTTP routes: unbound study access is rejected, the documented recovery routes remain usable, device registration unlocks study access, and both route prefixes behave alike.
- Adapt shared integration helpers to bind ordinary test sessions, while retaining explicit unbound login helpers for registration and recovery scenarios.
- Run Rust, SQLx offline-cache, integration, and browser checks in GitHub Actions; do not run heavy tests or builds on the production stack or local workstation.

## Out of Scope

- Hardware-backed device identity, browser fingerprinting, mobile attestation, device-count policy changes, new migrations, and visual redesign.
- Production deployment or live-student acceptance.

## Further Notes

The API already limits recovery behavior in the client to the device-management, account export/deletion, and session-policy routes after a `devices_exhausted` response. The server must enforce the matching exception explicitly rather than infer it from URL strings inside the general extractor.
