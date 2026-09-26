# Production Deployment Runbook

Built by GitHub Actions only (AGENTS.md compute policy — the VPS never builds).

## Live surfaces — medicalos.polytronx.com (shared VPS)

- **URL:** https://medicalos.polytronx.com/ (client SPA + API same-origin)
- Deployed automatically on every push to `main` by `.github/workflows/deploy.yml`:
  builds `medicalos-api:<sha>` + `medicalos-web:<sha>` → GHCR, SSHes to the
  VPS (`environment: production`), runs `infra/vps/deploy.sh <sha>`, then the
  `production-verify` job fails unless the live domain serves the **exact
  commit SHA** (`/api/version.json`) plus the app shell, entry bundle, and
  `/api/healthz == ok`.
- Details: `docs/deployment/vps-medicalos.md`. Provisioning:
  `/opt/platform/bin/provision-project.sh medicalos` (shared Postgres/Redis/
  MinIO/Soketi — never project-local backing services, per
  `/opt/platform/PLATFORM-RULES.md`).
- Note: the repository is flipped back to PRIVATE after each verified
  deployment (owner mandate) per the mandated PUBLIC → deploy → verify →
  PRIVATE sequence.

## Secrets & config

| Env | Default | Purpose |
|---|---|---|
| `DATABASE_URL` | — (required) | Shared platform Postgres (`PLATFORM_PG_URL`) |
| `MIN_TIME_LIMIT_SECONDS` | 30 | Floor for timed sessions |
| `FREE_DAILY_QUESTIONS` | 10 | Free-tier daily allowance |
| `PUBLIC_API_BASE_URL` | `https://medicalos.polytronx.com/api` in production | Public API prefix used to derive OIDC callback URLs |
| `PUBLIC_APP_URL` | `https://medicalos.polytronx.com` in production | Browser origin used for OIDC handoff |
| `OIDC_CREDENTIAL_KEY` | unset | Optional encryption key for institution OIDC client secrets; provision as GitHub secret `VPS_OIDC_CREDENTIAL_KEY` |
| `PACK_SIGNING_KEY` | — (required) | Private secret deriving lease-bound Ed25519 offline-pack signatures (legacy v1 remains HMAC); provision as GitHub secret `VPS_PACK_SIGNING_KEY` with at least 32 non-whitespace bytes |

GitHub repo-level secrets: `VPS_HOST`, `VPS_USER`,
`VPS_SSH_KEY`, `VPS_DATABASE_URL`, and `VPS_PACK_SIGNING_KEY`.

Institution OIDC setup and callback registration: `docs/deployment/institution-oidc.md`.

## Verification after any deploy

```bash
curl -s https://medicalos.polytronx.com/api/version.json   # sha == HEAD
curl -s https://medicalos.polytronx.com/ -o /dev/null -w '%{http_code}\n'  # 200
curl -s https://medicalos.polytronx.com/api/healthz        # ok
```
