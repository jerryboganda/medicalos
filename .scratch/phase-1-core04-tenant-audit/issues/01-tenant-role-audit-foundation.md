# 01: Tenant role and audit foundation

Status: ready-for-agent

Requirement IDs: CORE-04 (foundation), prepares CORE-01, AI-14, ADMIN-01, ADMIN-04, TRUST-03.

Implement the vertical slice in `../spec.md` through the authenticated HTTP seam.

- [ ] Personal context remains distinct while one identity can list multiple tenant memberships.
- [ ] Tenant roles and platform roles are separate, shared, stable domain vocabulary.
- [ ] Tenant routes require explicit `X-Tenant-Id` and reject non-members across tenants.
- [ ] Platform owner can atomically bootstrap a tenant plus initial institution administrator without becoming a tenant member.
- [ ] Support/non-owner users cannot use platform-owner tenant bootstrap.
- [ ] Institution administrator can add one valid tenant role for an existing user.
- [ ] Non-admin tenant members cannot manage memberships or escalate roles.
- [ ] Repeating the same role assignment is a semantic no-op with no duplicate audit event.
- [ ] Privileged mutations create append-only audit events with the required common event fields.
- [ ] Tenant administrators can retrieve a known audit receipt only within the matching tenant.
- [ ] Migration remains reversible under the repository up/down/up gate.
- [ ] GitHub Actions proves the slice; no heavy local/VPS compute and no deployment.
