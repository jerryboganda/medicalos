# 01: Notification policy and in-app inbox foundation

Status: in-progress (foundation implemented; GitHub Actions acceptance billing-blocked; real push delivery and producers pending)

Requirement IDs: CORE-08 (partial until real remote push transport, delivery policy enforcement, campaign analytics, and real product-event producers exist).

Implement the vertical slice in `../spec.md` through the authenticated HTTP and Account-page seams.

- [x] Durable personal-context notification preferences cover every master-plan category group.
- [x] Learner time zone and paired optional quiet hours are persisted and validated.
- [x] Mobile push-token registration is idempotent per learner/device/platform and does not expose provider tokens through a read endpoint.
- [x] In-app inbox is learner-isolated, newest-first, and exposes read state plus exact-action deep-link fields.
- [x] Read acknowledgement is idempotent and cannot mutate another learner's item.
- [x] Account UI exposes notification preferences and inbox without creating a sixth primary destination.
- [x] UI reuses existing Medical OS tokens and remains accessible/responsive under Hallmark constraints by static review; browser acceptance remains externally blocked.
- [x] No fake remote push transport, delivery receipt, campaign analytics, or notification producer is introduced.
- [x] Local verification remains lightweight; heavy API/client/browser acceptance runs only in GitHub Actions.
- [x] Matt two-axis review and Ponytail over-engineering review are completed before closeout.
- [x] Traceability records the truthful partial/blocked boundary.

## Comments

- Testing seams were inherited from the accepted project workflow: authenticated HTTP integration tests for backend behavior and the existing browser E2E seam for Account UI.
- QB-08 report resolution remains a deliberate Phase 2 editorial-console boundary; CORE-08 will not bypass that authorization model to manufacture a producer.
- TDD red commit `15282cc` was pushed before implementation, but run `35430685017` executed no runner steps because GitHub reported failed account payments or an insufficient spending limit; it is not functional red evidence.
- Implementation adds the notification preference/inbox schema, authenticated HTTP routes, Account controls, and browser coverage including fail-closed load states. Matt Standards/Spec review and Ponytail review found no remaining implementation finding after the load-state correction.
- CORE-08 stays partial until real APNs/FCM transport, delivery-time quiet-hour/daily-cap enforcement, campaign opt-out analytics, and at least one real product-event producer exist and are acceptance-tested.
