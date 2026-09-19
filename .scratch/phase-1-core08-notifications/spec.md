# CORE-08 — Notification preferences, push registration, and in-app inbox

## Problem Statement

Medical OS has no learner-controlled notification policy, no durable push-token registration, and no in-app inbox. The Phase 1 product plan requires mobile push mirrored by an in-app inbox, per-category controls, quiet hours in the learner's time zone, push-frequency limits, exact-action deep links, campaign opt-out measurement, and non-shaming reminder copy.

The repository does not yet contain a native APNs/FCM delivery adapter, a scheduler/worker that can honor quiet hours and daily push caps, or Phase 2/4 producers for most notification categories. The current QB-08 report-resolution route also deliberately remains unavailable until the editorial-console boundary exists. CORE-08 must therefore build the truthful server/client foundation without pretending that remote push delivery or future event producers already work.

## Solution

Add a personal-context notification foundation through the authenticated HTTP API: durable learner preferences, registered mobile push tokens, and an in-app inbox with per-item read state. The Account surface will expose notification settings and the inbox without adding a sixth primary learner destination.

Preferences cover every category named by the master plan, learner time zone, and optional quiet hours. Push-token registration is idempotent per learner/device/platform and stores provider tokens server-side without exposing them back through a list endpoint. Inbox reads are learner-isolated and read acknowledgement is idempotent.

Remote APNs/FCM delivery, delivery-time enforcement of the three-push/day and one-promotional/day caps, campaign analytics, and event producers that do not yet exist remain explicit adapter/phase boundaries. No fake notification, fake delivery receipt, or fake opt-out metric is introduced.

## User Stories

1. As a learner, I want one notification-preferences record in my personal context so that my choices follow my account rather than a tenant membership.
2. As a learner, I want plan and review reminders independently controllable so that study reminders match my preference.
3. As a learner, I want new mock or assignment notifications independently controllable so that institutional content does not force unrelated alerts.
4. As a learner, I want competition start/end notifications independently controllable so that future competition alerts are opt-in according to my settings.
5. As a learner, I want duel invitations independently controllable so that future social invitations can be silenced without muting other categories.
6. As a learner, I want report-resolved notifications independently controllable so that editorial outcomes can reach me when that workflow exists.
7. As a learner, I want subscription-event notifications independently controllable so that billing/service notices can follow the notification policy while account/receipt email remains separate.
8. As a learner, I want my time zone stored with notification preferences so that future delivery evaluates quiet hours in my local time.
9. As a learner, I want optional quiet-start and quiet-end times so that notifications can avoid a sleeping or protected period.
10. As a learner, I want to clear quiet hours entirely so that I can return to unrestricted delivery windows.
11. As a learner, I want a native mobile client to register its push token for the current personal account so that remote push can be added without changing the public API later.
12. As a learner, I want re-registering the same device/platform to update its token instead of creating duplicates so that provider token rotation is safe.
13. As a learner, I want a push token from another account never exposed to me so that notification routing remains private.
14. As a learner, I want an in-app inbox that lists only my notifications newest first so that I can recover alerts I missed.
15. As a learner, I want each inbox item to carry its category, title, body, creation time, read state, and exact-action deep link so that the client can take me to the intended action.
16. As a learner, I want marking a notification read to be idempotent so that retries do not cause errors or duplicate effects.
17. As a learner, I want an honest empty inbox when no producer has emitted notifications so that the product never fabricates engagement.
18. As a learner, I want notification controls under Account rather than as a new primary navigation destination so that the five-destination learner shell stays intact.
19. As a learner, I want accessible labels, focus behavior, and touch-sized controls so that notification settings work across supported screen sizes and input methods.
20. As a learner, I want reminder wording to remain non-shaming when producers are added so that notification copy does not create streak anxiety.
21. As an operator, I want remote push delivery to remain behind an explicit native/provider adapter boundary so that a registered token is never misrepresented as a delivered push.
22. As an operator, I want future delivery to enforce at most three pushes per learner-local day and at most one promotional push in that day so that the master-plan cap is authoritative.
23. As an operator, I want future campaign delivery to record opt-outs against real campaigns so that no synthetic analytics are reported.

## Implementation Decisions

- Notification settings are personal-context data keyed by user identity; they are not tenant-membership settings.
- Persist one preference row per learner with time zone, optional quiet start/end, and booleans for the six master-plan category groups.
- Default category toggles are enabled, time zone defaults to `UTC`, and quiet hours default to unset. The client may propose the browser/device IANA time zone, but the learner explicitly saves it.
- Time-zone text is bounded and stored without adding a time-zone dependency in this foundation slice. Delivery-time IANA resolution belongs to the scheduler/push adapter that actually evaluates local-day and quiet-hour rules.
- `GET /v1/me/notification-preferences` returns the learner's effective defaults and lazily persists them when absent.
- `PUT /v1/notification-preferences` replaces the learner-controlled preference values after validating bounded time-zone text and paired quiet-hour values.
- `POST /v1/push-tokens` registers or rotates a token for the authenticated learner's `(device_id, platform)` pair. Supported foundation platforms are `ios` and `android`.
- Push tokens are write-only through this public slice. There is no token-list endpoint and no fake send endpoint.
- Inbox rows are user-scoped and carry category, title, body, exact-action relative deep link, optional campaign key, promotional flag, created time, and nullable read time.
- `GET /v1/notifications` returns only the authenticated learner's inbox, newest first.
- `POST /v1/notifications/{id}/read` marks only the learner's own item read and is idempotent.
- No public notification-create endpoint is added. Product events must create notifications from trusted server-side producers when those event workflows exist.
- The current QB-08 resolve stub remains unchanged; CORE-08 does not smuggle Phase 2 editorial authorization into Phase 1.
- Remote APNs/FCM delivery is not faked. Delivery-time quiet-hour enforcement, the three-push/day cap, the one-promotional/day cap, and campaign opt-out analytics remain blocked on the real native/provider delivery boundary.
- The Account UI reuses existing Medical OS tokens, cards, buttons, and fields. No UI framework, notification SDK, state library, or speculative abstraction is introduced.

## Testing Decisions

- The primary backend seam is the existing authenticated HTTP integration suite.
- Test preferences through GET/PUT behavior, including defaults, persistence, paired quiet-hour validation, and account isolation.
- Test push registration through repeated POST requests for the same device/platform to prove idempotent rotation without exposing tokens through a read API.
- Test the inbox's honest empty state and not-found behavior for read acknowledgement until a real product event producer exists. Do not seed fake product notifications merely to make an acceptance test pass.
- The client seam is the existing browser E2E suite: Account loads notification preferences, saves them, and renders an honest empty inbox without altering the five-destination primary navigation.
- Heavy compilation, database integration tests, browser tests, and builds run only in GitHub Actions. Local verification is limited to formatting/static inspection and diff checks.

## Out of Scope

- APNs/FCM transport implementation and provider credentials.
- Native/community Tauri push plugin selection and store entitlement configuration.
- Scheduled delivery workers and delivery receipts.
- Enforcement evidence for three pushes per learner-local day and one promotional push per day before a real delivery transport exists.
- Campaign opt-out analytics before real campaigns/deliveries exist.
- Email account/receipt delivery, which remains separate from push/inbox policy.
- Phase 2 mock/assignment and editorial-console producers that do not yet exist.
- Phase 4 competition/duel producers that do not yet exist.
- Subscription billing workflows that do not yet exist.
- Deployment.

## Further Notes

- CORE-08 remains partial until a real remote-push adapter plus at least one real notification-producing product event can exercise delivery policy end to end.
- Empty inboxes are valid product state, not missing demo data.
- The delivery adapter must consume these stored preferences rather than duplicating notification policy in native clients.
