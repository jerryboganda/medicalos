# 01: Tenant role and audit foundation

Status: resolved — tested foundation; CI run 35425807184 passed Rust, client, site, and browser E2E gates.

Requirement IDs: CORE-04 (foundation), prepares CORE-01, AI-14, ADMIN-01, ADMIN-04, TRUST-03.

Implement the vertical slice in `../spec.md` through the authenticated HTTP seam.

- [x] Personal context remains distinct while one identity can list multiple tenant memberships.
- [x] Tenant roles and platform roles are separate, shared, stable domain vocabulary.
- [x] Tenant routes require explicit `X-Tenant-Id` and reject non-members across tenants.
- [x] Platform owner can atomically bootstrap a tenant plus initial institution administrator without becoming a tenant member.
- [x] Support/non-owner users cannot use platform-owner tenant bootstrap.
- [x] Institution administrator can add one valid tenant role for an existing user.
- [x] Non-admin tenant members cannot manage memberships or escalate roles.
- [x] Repeating the same role assignment is a semantic no-op with no duplicate audit event.
- [x] Privileged mutations create append-only audit events with the required common event fields.
- [x] Tenant administrators can retrieve a known audit receipt only within the matching tenant.
- [x] Migration remains reversible under the repository up/down/up gate.
- [x] GitHub Actions proves the slice; no heavy local/VPS compute and no deployment.

## Comments

- Acceptance evidence: GitHub Actions run 35425807184 passed formatting, schema application, Clippy, dependency/license audit, workspace tests, SQLx offline cache generation, wasm shared-core build, client/site builds, and browser E2E.
- Ask Matt standards/spec review: no requirement gap or standards violation found for this slice.
- Ponytail review: `Lean already. Ship.`
- Boundary: this closes the CORE-04 authorization/audit foundation only. PostgreSQL RLS, tenant-aware learner data/AI memory, support impersonation, institution hierarchy, and deployment remain out of scope.
