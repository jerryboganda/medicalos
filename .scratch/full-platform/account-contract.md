Status: implemented; CI acceptance pending
Scope: learner account controls UI
External gate: authoritative privacy, retention, and erasure policy details remain pending.

## Problem Statement

Learners need a clear place to review registered devices, download the account data the current export supports, and disable access to their account. The controls must describe the current API behavior truthfully, including the partial export and soft deletion.

## Solution

Add an Account destination to the existing You navigation group and render the controls as a stack of existing design-system cards. The page lists registered devices, identifies this browser by its existing device key, revokes another device or signs out this device, downloads the current JSON export, and offers account deletion behind a reversible review step and explicit acknowledgement.

## User Stories

1. As a learner, I want to see my registered devices and their recent activity so that I can recognize sessions on my account.
2. As a learner, I want to revoke another device so that its access is disabled and the list reflects the result.
3. As a learner, I want to revoke the device I am using so that this browser is signed out immediately.
4. As a learner, I want loading, error, retry, and empty states for device loading so that an unavailable response is clear and recoverable.
5. As a learner, I want to download the supported account export as JSON so that I can keep a copy of the data currently included.
6. As a learner, I want the export scope described as partial, including account, attempts, notes, card reviews, and portfolio, so that I do not mistake it for a complete archive.
7. As a learner, I want to review and cancel account deletion before confirming it so that I can stop before the request is sent.
8. As a learner, I want deletion to say that access is disabled while existing records are retained so that I am not told the records were erased.
9. As a learner, I want successful account deletion to clear this browser's sign-in so that the deleted account is no longer presented as authenticated here.
10. As a learner, I want malformed API responses to produce a clear error instead of a fabricated device list or export.
11. As a learner, I want to see my saved single-session setting without an assumed default.
12. As a learner, I want a plain explanation and explicit confirmation before enabling a setting that signs out every active session, including this browser.
13. As a learner, I want a failed or unavailable session setting to show a retry rather than a value the server never returned.
14. As a learner signing in on a new browser at the device limit, I want to manage my session setting while I free a device slot.

## Implementation Decisions

- Reuse the existing navigation list so Account appears in the You rail and derived command palette. It is not a primary mobile tab; the five existing tabs remain unchanged.
- Use the existing account API façade: `Api.listDevices()`, `Api.revokeDevice(id)`, `Api.exportAccount()`, and `Api.deleteAccount()`.
- Validate the unknown device response as an object containing a `devices` array with the device id/key, label, activity timestamps, and revocation state needed by the UI. Compare `device_key` with the browser's existing device key to identify the current device.
- Accept a device revoke only when the response confirms `revoked: true`. Revoking the current device clears local authentication and shows a sign-in path.
- If registration reports the active-device limit, permit only authenticated account list/revoke/export/closure and session-policy read/write requests. Other registration failures and study requests still fail closed. A real API browser regression fills all five slots, retires one, and registers the new browser on reload; the session-policy UI regression covers safe setting changes while a new browser is blocked.
- Validate export JSON for an account object and arrays named `attempts`, `notes`, `card_reviews`, and `portfolio`. The UI calls `card_reviews` “reviews” in learner-facing copy.
- Describe the export as partial. Other account-data categories remain pending and are not claimed to be included.
- Accept account deletion only when the response confirms `deleted: true`. On success, clear local authentication. Describe the operation as access disablement with records retained; make no erasure or retention-period claim.
- Read the persisted single-active-session policy from its authenticated API endpoint. Never render an assumed default while the stored value is loading or unavailable.
- Explain that enabling single-session protection immediately signs out every active session, including the current browser. Require a separate user action after that explanation; on confirmation, send the existing session-policy update and clear local authentication after success.
- Preserve the existing navigation group, tokens, cards, typography, responsive layouts and five primary mobile tabs.
- Keep response validation and learner-facing state in the page; do not add generated API types or change the device/offline-pack API.

## Testing Decisions

- Test the public page through Playwright with synthetic API responses; do not couple the test to Svelte state or private helpers.
- Cover device listing, revoking another device, signing out by revoking the current device, partial JSON download content, the reversible deletion review, confirmed DELETE, and local sign-in invalidation.
- Cover device loading, retry after an API error, and the empty list through observable UI states.
- Cover authenticated policy read, persisted false/true values, confirmation and cancellation before immediate sign-out, confirmed local sign-out, and retry after a failed policy read.
- Cover a full device limit where session settings remain usable while study requests stay blocked.
- Use the existing device-session, shell-navigation, and browser-download E2E patterns as prior art. The agreed seam is the learner page plus intercepted `/v1/me/*` requests.

## Out of Scope

- A complete account archive, hard erasure, retention scheduling, or new privacy features.
- Claims about which unexported categories exist or how long retained records remain stored.
- New themes, fonts, visual patterns, or mobile primary tabs.

## Further Notes

The current export response uses `card_reviews` and includes `account`, `attempts`, `notes`, and `portfolio`. Account deletion is a soft deletion: it disables access and retains existing records. An authoritative privacy/retention/erasure policy remains an external gate for any more specific disclosure.
