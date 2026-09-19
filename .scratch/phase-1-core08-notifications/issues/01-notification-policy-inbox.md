# 01: Notification policy and in-app inbox foundation

Status: ready-for-agent

Requirement IDs: CORE-08 (partial until real remote push transport, delivery policy enforcement, campaign analytics, and real product-event producers exist).

Implement the vertical slice in `../spec.md` through the authenticated HTTP and Account-page seams.

- [ ] Durable personal-context notification preferences cover every master-plan category group.
- [ ] Learner time zone and paired optional quiet hours are persisted and validated.
- [ ] Mobile push-token registration is idempotent per learner/device/platform and does not expose provider tokens through a read endpoint.
- [ ] In-app inbox is learner-isolated, newest-first, and exposes read state plus exact-action deep-link fields.
- [ ] Read acknowledgement is idempotent and cannot mutate another learner's item.
- [ ] Account UI exposes notification preferences and inbox without creating a sixth primary destination.
- [ ] UI reuses existing Medical OS tokens and remains accessible/responsive under Hallmark constraints.
- [ ] No fake remote push transport, delivery receipt, campaign analytics, or notification producer is introduced.
- [ ] Local verification remains lightweight; heavy API/client/browser acceptance runs only in GitHub Actions.
- [ ] Matt two-axis review and Ponytail over-engineering review are completed before closeout.
- [ ] Traceability records the truthful partial/blocked boundary.

## Comments

- Testing seams were inherited from the accepted project workflow: authenticated HTTP integration tests for backend behavior and the existing browser E2E seam for Account UI.
- QB-08 report resolution remains a deliberate Phase 2 editorial-console boundary; CORE-08 will not bypass that authorization model to manufacture a producer.
