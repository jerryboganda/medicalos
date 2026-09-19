# CORE-04 — Multi-tenant role and audit foundations

## Problem Statement

Medical OS currently authenticates a user but has no institution authorization context, role model, or durable audit foundation. That makes it impossible to safely add institution administration, faculty tools, owner operations, tenant-private AI memory, or support workflows without risking accidental cross-tenant access.

The platform needs a minimal, explicit authorization foundation now: personal learning remains separate, a user may belong to multiple institutions, every tenant-scoped request must prove membership, privileged membership changes must require an institution administrator, platform-owner powers must remain separate from tenant roles, and privileged changes must leave immutable audit evidence.

## Solution

Add tenant memberships, multi-role tenant assignments, platform roles, explicit tenant-context resolution, platform-owner tenant bootstrap, tenant-admin membership-role assignment, and append-only audit events.

The authenticated HTTP API remains the single testing seam. Tenant-private routes require an explicit tenant identifier in the request and validate that the authenticated user belongs to that tenant. Platform-owner routes use a separate platform-role check and do not silently grant tenant membership.

This slice intentionally establishes the authorization and audit foundation only. It does not yet move learner plans/sessions into institution contexts, add an owner dashboard UI, implement support impersonation, or claim PostgreSQL row-level security complete.

## User Stories

1. As a learner, I want my personal learning context to remain available even when I join institutions so that institutional access does not replace my personal account.
2. As a user, I want to belong to more than one institution so that one identity can serve multiple programs without duplicate accounts.
3. As a user, I want my roles to be scoped to a specific institution so that a role in one tenant grants nothing in another tenant.
4. As a user, I want to see the institution contexts and roles I actually hold so that the client can present truthful role-aware navigation.
5. As a user, I want tenant-scoped requests to require an explicit tenant context so that the server never guesses which institution I meant.
6. As a tenant member, I want access to a tenant context rejected when I am not a member so that another institution's private surface stays inaccessible.
7. As an institution administrator, I want to add an existing user to my institution with an approved tenant role so that staff and learners can be authorized without creating another identity.
8. As an institution administrator, I want repeating the same membership-role assignment to be idempotent so that retries do not create duplicate roles or duplicate audit events.
9. As a non-administrator tenant member, I want membership-management operations denied so that ordinary roles cannot escalate privileges.
10. As a platform owner, I want to create an institution and assign its initial institution administrator so that a tenant can be bootstrapped without granting myself tenant membership.
11. As a support user, I want platform-owner-only tenant creation denied so that support access cannot silently become owner access.
12. As a security reviewer, I want every privileged tenant bootstrap and membership-role addition to create durable audit evidence so that administrative changes are accountable.
13. As a security reviewer, I want audit events to include actor, tenant, subject, aggregate identity/version, occurred/received timestamps, correlation identifier, privacy scope, and payload so that later incident review has a stable evidence contract.
14. As a tenant administrator, I want to fetch a known audit event only inside its tenant scope so that audit evidence itself cannot leak across tenants.
15. As a security reviewer, I want audit history to be append-only so that privileged application code cannot silently rewrite or delete past events.
16. As a developer, I want tenant and platform role names centralized in the shared domain contract so that clients and server do not invent incompatible authorization vocabulary.

## Implementation Decisions

- The personal context is represented separately from tenants; it is not implemented as a hidden personal tenant.
- A tenant is the institution-level private authorization namespace for this slice. Campus/program/cohort hierarchy remains a later institution-domain expansion.
- Tenant membership is many-to-many between users and tenants. Roles are stored separately from membership so one membership can hold multiple tenant roles without array/JSON authorization logic.
- Tenant roles in this foundation are learner, instructor, author, medical reviewer, examiner, program lead, institution administrator, and billing administrator.
- Platform roles are support and platform owner. They are stored separately from tenant roles and do not imply tenant membership.
- Role vocabulary is represented by shared domain types with stable snake-case wire values.
- Tenant-scoped HTTP routes require an explicit `X-Tenant-Id` header. Missing/invalid scope is rejected; a valid tenant identifier without membership is forbidden.
- The authenticated tenant context exposes the authenticated user identifier, tenant identifier, tenant status/name, and the roles held in that tenant.
- `GET /v1/me/contexts` returns the personal context plus all tenant memberships and platform roles visible to the authenticated user.
- `GET /v1/tenant/context` resolves the explicit tenant scope and proves membership without mutating state.
- `POST /v1/platform/tenants` requires the platform-owner role and atomically creates a tenant, an initial membership for an existing user, the institution-administrator role, and an audit event.
- `POST /v1/tenant/memberships` requires an institution-administrator role in the explicit tenant context and atomically adds an existing user plus one tenant role. Repeating an existing assignment is a semantic no-op.
- `GET /v1/tenant/audit/{event_id}` requires institution-administrator authority in the explicit tenant context and returns only an event whose tenant matches that context.
- Audit events are database-enforced append-only records. This slice writes security-relevant administrative events only; it does not attempt to log every read.
- Each privileged write generates a correlation identifier and records actor, tenant, subject, aggregate identity/version, occurred-at, received-at, optional device, privacy scope, action, and structured payload.
- No new dependency is introduced.

## Testing Decisions

- The only acceptance seam is the authenticated HTTP API, matching existing integration-test prior art.
- Tests create normal users through the public auth flow; only test bootstrap data for the first platform-owner role is inserted directly because self-elevation must not exist as a production API.
- A red test first covers: owner tenant bootstrap, one identity in two tenants, truthful context listing, explicit tenant resolution, tenant-admin membership assignment, idempotent retry, role denial, cross-tenant denial, platform-role separation, audit receipt retrieval, and audit tenant isolation.
- Existing migration up/down/up coverage must include the new migration.
- Heavy database/build/test work runs only in GitHub Actions. Local checks are limited to formatting/diff/static inspection.

## Out of Scope

- Tenant-aware learner plans, attempts, goals, notes, AI memory, retrieval, exports, caches, or background jobs; those consumers migrate onto this foundation in their owning requirements.
- PostgreSQL row-level-security policies and adversarial RLS tests; those remain TRUST-03 work and CORE-04 must not claim them complete.
- Campus/program/cohort/course hierarchy and faculty workspace features.
- Owner dashboard UI, billing administration UI, tenant suspension UI, or feature-flag UI.
- Support impersonation or supervised support access. If added later it must be time-limited, purpose-recorded, highly restricted, and separately audited.
- Invitations, email delivery, SSO/SCIM, or automatic user provisioning.
- Content authoring/review/publishing workflows beyond establishing distinct role vocabulary.
- Deployment.

## Further Notes

- Maps directly to CORE-04 and prepares CORE-01 institution contexts, AI-14 private-memory isolation, ADMIN-01/04, and TRUST-03 without claiming those requirements complete.
- The master plan requires explicit tenant/user scopes and treats row-level security as defense in depth; this slice provides the application authorization seam that later RLS policies will reinforce.
- Platform-owner and support roles are deliberately not tenant memberships, preserving the distinction between cross-tenant operations and tenant-private access.
