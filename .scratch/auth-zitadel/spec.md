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

- **Migration `0060_zitadel_identity`:**
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
