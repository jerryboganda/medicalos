# Institution OIDC sign-in

The API supports institution-scoped OpenID Connect Authorization Code login with PKCE. Provider configuration is disabled by default. Sign-in only succeeds when the verified `(issuer, sub)` is already mapped to an active Medical OS account for that institution; email is never used to link or create accounts.

## Production configuration

Register this callback URL with the institution's OIDC provider, replacing the UUID with the institution ID:

```text
https://medicalos.polytronx.com/api/v1/auth/oidc/callback/<institution-id>
```

The API derives callback and browser handoff URLs from `PUBLIC_API_BASE_URL` and `PUBLIC_APP_URL`. Production deploys set them to `https://medicalos.polytronx.com/api` and `https://medicalos.polytronx.com`. Local development defaults use loopback HTTP; non-loopback HTTP URLs are rejected.

For confidential clients, set the GitHub Actions repository secret `VPS_OIDC_CREDENTIAL_KEY` before saving a client secret. Use at least 32 bytes of randomly generated material and keep it stable across deployments. The API encrypts client secrets before storing them. Without the key, public clients can use PKCE, but client secrets cannot be saved. Changing the key makes stored secrets unreadable; re-enter each affected provider secret after changing it.

## Configure a provider

1. In the admin console, load settings using the institution UUID.
2. Enter the exact issuer URL, client ID, and optional secret. Keep the provider disabled while checking the redirect URL and client registration.
3. Save the provider and enable it when the provider is ready.
4. Share the generated learner URL: `/login?institution=<institution-id>`.
5. Before learner sign-in, bind each exact provider subject to an existing account through `POST /v1/institutions/{institution_id}/external-enrollments`. Use the configured issuer as `provider`, the verified OIDC `sub` as `subject`, and an existing learner's `user_id`; an optional cohort must belong to the same institution. This endpoint requires an authenticated institution admin or instructor.

Configuration changes are audit logged. The API never returns or writes client secrets to audit records. Sign-in state is short-lived and one-use; the browser receives a short-lived one-use handoff ticket, which it exchanges for the normal application session.

## Disable or rotate credentials

Disable the provider in its settings to stop new sign-ins. To rotate a secret, enter the new secret and save. To remove it, clear the stored secret and save; a provider that requires a secret will then reject the token exchange. If the encryption key changes, re-enter the secret so it is encrypted with the new key.

Provider registration and certification remain institution-specific operational steps. This implementation does not provide SAML, roster-file import, automatic account creation, or production activation.
