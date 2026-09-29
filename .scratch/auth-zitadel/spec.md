# Zitadel identity and central RBAC

Status: in-progress (phase 1)
Plan: phases 0–6 in the approved plan (`~/.claude/plans/search-the-ebst-rbac-radiant-teapot.md`)
Requirement IDs: §6.3 (accounts and sign-in), §18.1 (roles and tenancy), §19.3 (separation of duties), §25 (MFA for privileged roles), CORE-07, ADMIN-06 ("role-aware admin permissions remain")

## Problem

- **Admin access:** one shared `ADMIN_TOKEN` gates every admin route. It isn't tied to a person and has no MFA.
- **Roles:** they are free-text columns. There's no permission model to check against.
- **Sign-in:** Google, Apple, email verification, password reset and MFA are required by the plan, but none exist.

## Decision

- **Identity provider:** self-hosted Zitadel, the owner's choice. Keycloak was never part of this system.
- **The API owns the OIDC flow:** it is Zitadel's only OIDC client, so the webview never holds IdP tokens.
- **Sessions stay the same:** opaque sessions, now carrying a platform-role snapshot and an MFA flag.
- **Where roles live:**
  - platform roles are Zitadel project roles;
  - institution roles stay in `institution_members`;
  - relationship rules stay with their data.
- **One `authz.rs`** maps roles to permissions.

## Phase 1 (this slice): additive, nothing removed

- **Migration `0061_zitadel_identity`:**
  - `users.idp_subject`, and `password_hash` becomes nullable;
  - `auth_sessions` and `oidc_login_tickets` gain `roles` and `mfa`;
  - new table `platform_login_states`;
  - `institution_members.role` gets a CHECK constraint (NOT VALID).
- **`authz.rs`:**
  - a `Permission` enum;
  - `platform_grants` and `institution_grants`;
  - `AuthUser::require`, where privileged permissions need MFA;
  - `require_in(institution)`.
- **`routes/platform_auth.rs`:**
  - `GET /v1/auth/start?idp=google|apple` returns the authorization URL: PKCE, nonce, the Zitadel roles scope, the project audience and the IdP hint.
  - `GET /v1/auth/oidc/callback` verifies the ID token, then either links the user by `sub` or by imported UUID, or creates them from a verified email. It never links by matching email.
  - Then comes the existing one-use ticket, and `POST /v1/auth/oidc/complete` issues the session carrying roles and MFA.
  - `GET /v1/me` returns `{user_id, email, mfa, roles, permissions, institutions[]}`.
- **Config:** `ZITADEL_ISSUER`, `ZITADEL_CLIENT_ID` and `ZITADEL_CLIENT_SECRET` must all be set for platform sign-in to switch on. `ZITADEL_PROJECT_ID`, `ZITADEL_IDP_GOOGLE` and `ZITADEL_IDP_APPLE` are optional.

## Acceptance (phase 1)

- **Unit tests:**
  - roles grant only their permissions;
  - privileged permissions without MFA return `mfa_required`;
  - the role claim is parsed correctly.
- **Integration test with the stub IdP:**
  - the scopes are correct;
  - an unconfigured IdP returns 404;
  - first sign-in creates the account, and `/v1/me` shows `platform_owner` with MFA;
  - a returning sign-in keeps the same account and drops roles and MFA;
  - an email collision is refused;
  - an unverified email is refused;
  - password sessions carry no roles.
- **Unchanged:** every existing test, `ADMIN_TOKEN` handlers, password login and institution SSO.

## Phase 2: production identity plumbing + admin access off the shared token

Status: in-progress (2026-09-28, ZCode)

Requirement IDs: §6.3, §18.1, §25, CORE-01, CORE-07, ADMIN-06, OPS-02.

### Problem

Phase 1 shipped the code but production cannot use it:

- **No IdP env reaches production.** `deploy.yml` and `infra/vps/deploy.sh`
  pass no `ZITADEL_*`, so `/v1/auth/providers` serves `false/false/false` and
  platform sign-in 404s — expected until the owner decides where production
  Zitadel runs (owner gate, below), but the plumbing must exist first.
- **Admin routes need a person.** `require_admin` accepts only the shared
  `ADMIN_TOKEN`. Production passes none, so every admin route is
  `admin_disabled` in the live deployment: editorial work is impossible.

### 2a — deploy plumbing (inert until the owner sets secrets)

- Pass `ZITADEL_ISSUER`, `ZITADEL_CLIENT_ID`, `ZITADEL_CLIENT_SECRET` (plus
  optional `ZITADEL_PROJECT_ID`, `ZITADEL_IDP_GOOGLE`, `ZITADEL_IDP_APPLE`)
  from new repo secrets `VPS_ZITADEL_*` through the `deploy-vps` env and the
  API container, mirroring the `OIDC_CREDENTIAL_KEY` pattern.
- Fail the deploy loudly on a partial trio (any of the three set without all
  three) — the API would otherwise silently disable platform sign-in.
- Re-sync `infra/vps/deploy.sh` with the inline script (the 90×2s health wait
  landed only in the workflow copy) — the files claim to be kept in sync.
- **Owner gate:** where production Zitadel runs (Zitadel Cloud / on-VPS
  container / dedicated host). Until decided and the secrets set, the
  providers endpoint staying all-false is the designed state.

### 2b — admin access: role sessions accepted at the same seam

- `require_admin(user, provided)` accepts EITHER a session holding
  `Permission::PlatformOps` with MFA (via the existing `AuthUser::require`),
  OR the legacy `ADMIN_TOKEN` when configured — CI/local e2e keep the token
  path unchanged; the signature change makes all ~70 call sites compile-check.
- A privileged role without MFA falls through to the token path; if both
  paths fail, the honest error wins (`mfa_required` over `admin_required`
  when the session holds the permission but not the factor).
- Full per-route permission mapping (authors → `ContentAuthor` routes,
  reviewers → `ClinicalApprove` routes) stays a later slice; today the whole
  console gates on `PlatformOps`, which only `platform_owner` grants.
- Client: no functional change needed — admin pages already send the bearer
  session, and the server now accepts it; the `mlos_admin` localStorage
  entry stays as the local-dev escape hatch until cutover.

### Acceptance (phase 2)

- Integration tests: `platform_owner` + MFA session reaches an admin route
  with no `x-admin-token`; the same role without MFA gets `mfa_required`
  (and passes with the legacy token as break-glass); a plain session with a
  wrong token still gets `admin_required`; with `ADMIN_TOKEN` unset the
  `admin_disabled` behavior is unchanged.
- Deploy workflow green; providers endpoint still all-false (no secrets set).
- No production behavior change until the owner's IdP decision.

## Phase 3: role-aware admin routes (ADMIN-06 tail)

Status: in-progress (2026-09-29, ZCode)

`require_permission(user, provided, permission)` replaces the single
`PlatformOps` gate: each admin route names its §18.1 permission; the legacy
token path is unchanged (the operator token stays full-authority). A role
without the route's permission gets `permission_required`; every privileged
permission still demands MFA. Ambiguous routes stay `PlatformOps`.

| Permission | Routes |
|---|---|
| ContentAuthor | hierarchy create/update, question create/search, JSON/file import + rollback, content rights CRUD, concepts (all), extraction create/get/list, article admin + create, image case/annotation create + concept mapping, source-change lifecycle, pregen generation, variant create, scenario create/version |
| ClinicalApprove | assessment workflow transitions, extraction report review, image annotation review, psychometric screening queue |
| ExamConfigure | exam spec create, assessment form create, mock create/list, QTI export |
| ExamAssess | scenario assessment get/record/pending, scenario appeal list/get/review |
| ReportTriage | report queue/resolve, incidents create/list/update, prize review |
| PlatformOps (unchanged) | dashboard, audit log, ai-admin, settings, flags, recovery drills, coach regression, OIDC provider config |

Refusals keep the honest codes: `permission_required` (role lacks the
permission), `mfa_required` (role has it, no second factor).
