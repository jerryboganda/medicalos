# Production on the shared VPS — medicalos.polytronx.com

Live surfaces (2026-09-19, owner-directed):

- **Client + API origin:** `https://medicalos.polytronx.com`
  (Cloudflare-proxied → Nginx Proxy Manager on the VPS → containers).
- **API:** `https://medicalos.polytronx.com/api/*` → `medicalos-api:8080`
  (same origin, so no CORS configuration is needed).
- **Client:** SvelteKit static SPA served by `medicalos-web` (nginx) at `/`.
- GitHub Pages is RETIRED for this project (the old
  `jerryboganda.github.io/medicalos` verify target is replaced by the
  domain checks below).

## VPS layout (shared-infrastructure rules, /opt/platform/PLATFORM-RULES.md)

- Postgres: shared `platform-postgres` — database + role `medicalos`
  (provisioned by `/opt/platform/bin/provision-project.sh medicalos`;
  credentials in `/opt/platform/projects/medicalos.env`, chmod 600).
- Containers (both on networks `platform` + `nginx-proxy-manager_default`,
  `cpus` + `mem_limit` mandatory, nothing on 0.0.0.0):
  - `medicalos-api` — `ghcr.io/jerryboganda/medicalos-api:<sha>`
    (built by GitHub Actions, never on the VPS), env `DATABASE_URL`
    from the provisioned `PLATFORM_PG_URL`, `MIN_TIME_LIMIT_SECONDS=30`,
    `FREE_DAILY_QUESTIONS=10`. Schema applies at startup; optional
    `docker exec medicalos-api api --seed` once for demo content.
  - `medicalos-web` — `${REGISTRY:-ghcr.io/jerryboganda/}medicalos-web:<sha>`
    (built by GitHub Actions from `apps/client/Dockerfile`), nginx serves
    the static build + proxies `/api/` to `medicalos-api:8080` (see
    `apps/client/nginx.conf`). Client baked with
    `VITE_API_BASE=''` (same-origin; API reached via the `/api/` prefix).
- NPM proxy host: `medicalos.polytronx.com` → `medicalos-web:80`,
  SSL forced (Let's Encrypt via NPM), HSTS per shared default.
- Backups: the `medicalos` database is inside platform-postgres, so the
  nightly `platform-backup` pg_dump covers it (7 nights on-box).

## Deploy flow (every push to main)

1. GitHub Actions `deploy` workflow builds `medicalos-api:<sha>` and
   `medicalos-web:<sha>`, pushes both to GHCR (VPS pulls, never builds).
2. `deploy-vps` job (Okedeoудовлетворение: `environment: production`,
   concurrency `medicalos-vps`) SSHes in and runs
   `infra/vps/deploy.sh <sha>` — pull, recreate both containers,
   health-check `/healthz` through the API container, report versions.
3. `production-verify` curls the live domain: `/api/version.json` carries
   the deployed SHA, `/` serves the app shell, the JS entry bundle
   returns 200, `/api/healthz` returns `ok`.

## Secrets (GitHub environment `production`)

| Name | Purpose |
|---|---|
| `VPS_HOST` | `185.252.233.186` |
| `VPS_USER` | `root` |
| `VPS_SSH_KEY` | Private key matching the VPS `authorized_keys` |
| `VPS_DATABASE_URL` | `PLATFORM_PG_URL` value from the provisioned env file |
