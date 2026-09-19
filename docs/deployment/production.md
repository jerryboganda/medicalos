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

GitHub repo-level secrets: `VPS_HOST`, `VPS_USER`,
`VPS_SSH_KEY`, `VPS_DATABASE_URL`.

## Verification after any deploy

```bash
curl -s https://medicalos.polytronx.com/api/version.json   # sha == HEAD
curl -s https://medicalos.polytronx.com/ -o /dev/null -w '%{http_code}\n'  # 200
curl -s https://medicalos.polytronx.com/api/healthz        # ok
```
