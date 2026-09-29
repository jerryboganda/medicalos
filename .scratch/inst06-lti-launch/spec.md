# INST-06 — LTI 1.3 launch (tool side)

Status: in-progress (slice 1: OIDC login initiation, resource-link launch,
tool JWKS, API-level deep linking)
Requirement IDs: INST-06, INST-01 (institution membership), §18.1.

## Problem

The interop lane shipped QTI 2.1 export; the actual LTI 1.3 launch a campus
LMS performs was not built. Institutions cannot place medicalos content in
Canvas/Moodle-style platforms.

## Scope of this slice (API-only, no client/UI changes)

- **Platform registration** (institution-scoped): `POST
  /v1/institutions/{id}/interop/lti-platforms` gated `InstitutionAdmin`
  through `authz::require_in`; registers issuer, client_id, deployment_id,
  the platform's OIDC auth login URL and JWKS key-set URL.
- **OIDC third-party login initiation**: `GET|POST /v1/lti/login` — the
  platform redirects here; the tool validates the registration, stores a
  single-use `state` + `nonce` (10 min TTL), and 303-redirects the browser
  to the platform's auth login URL with the LTI OIDC parameters.
- **Resource-link launch**: `POST /v1/lti/launch` (form_post `id_token` +
  `state`) — consumes the one-use state, fetches the platform's JWKS,
  verifies RS256 signature, iss/aud/exp and the echoed nonce, then checks
  message type, `1.3.0` version and deployment id. The LMS user is linked
  once per (platform, subject): first launch links a **verified-email**
  match that is also a member of the platform's institution — never a bare
  email match, and never account creation. Issues the shared app session.
- **Tool JWKS**: `GET /v1/lti/jwks.json` — the RSA public key half of
  `LTI_TOOL_PRIVATE_KEY` (PKCS#8 PEM, off when unset, like the Zitadel
  config), kid = short SHA-256 of the modulus.
- **Deep linking (API-level)**: a `LtiDeepLinkingRequest` launch stores the
  platform-provided settings server-side against the launched staff user
  (never trusted from the client later). `POST /v1/lti/deep-links` then
  signs the `LtiDeepLinkingResponse` JWT (RS256, tool kid, aud/iss = the
  tool's client_id at the platform, 5-minute expiry, `data` echoed) over
  the submitted content items and returns the platform's return URL with
  the JWT for the browser handoff.
- **Handoff honesty**: the launch returns the session in JSON. A browser
  handoff page would be new UI surface — deliberately deferred behind the
  standing no-UI mandate.

## Out of scope / gated

- Real-platform certification (needs a campus LMS — external).
- Browser launch handoff page and content-selection picker (UI-gated).
- Names-and-provisioning / assignment-grades services (later slices).
