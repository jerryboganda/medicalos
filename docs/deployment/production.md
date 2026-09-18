# Production Deployment Runbook

Two production surfaces, both built by GitHub Actions only (AGENTS.md compute policy — the VPS never builds).

## 1. Learner client (web/PWA) — LIVE

- **URL:** https://jerryboganda.github.io/medicalos/
- Deployed automatically on every push to `main` by `.github/workflows/deploy.yml`.
- The workflow's `production-verify` job fails unless the live site serves the
  **exact commit SHA** (`/version.json`) plus the app shell and entry bundle.
- Note: the repository is flipped back to PRIVATE after each verified
  deployment (owner mandate). If Pages stops serving while private (plan
  dependency), redeploy while PUBLIC per the mandated sequence — the flip
  cycle is what keeps it live.

## 2. API — GHCR image, one command on the VPS

Image: `ghcr.io/jerryboganda/medicalos-api:latest` (also tagged per-commit SHA).

On the production VPS (serving only — never builds, per AGENTS.md):

```bash
# 1. PostgreSQL (once; skip if the VPS already runs one)
docker volume create pgdata
docker run -d --name pg --restart unless-stopped \
  -v pgdata:/var/lib/postgresql/data \
  -e POSTGRES_PASSWORD=<secret> -e POSTGRES_DB=medical_os postgres:17

# 2. API (schema is applied by the container at startup)
docker run -d --name medicalos-api --restart unless-stopped \
  -p 127.0.0.1:8080:8080 \
  -e DATABASE_URL=postgres://postgres:<secret>@pg:5432/medical_os \
  ghcr.io/jerryboganda/medicalos-api:latest
# (add --network to share the pg container network, or use the host IP)

# 3. Seed synthetic demo content (optional, once)
docker exec medicalos-api api --seed

# 4. TLS proxy in front (Caddy example) — the only other process
#    caddy: reverse_proxy 127.0.0.1:8080 with your domain + auto-HTTPS
```

Then point the client build at the API host: set `VITE_API_BASE` in
`.github/workflows/deploy.yml` (web-pages job) to the API's public URL and
push — the Pages redeploy picks it up.

## Secrets & config

| Env | Default | Purpose |
|---|---|---|
| `DATABASE_URL` | — (required) | PostgreSQL connection |
| `MIN_TIME_LIMIT_SECONDS` | 30 | Floor for timed sessions |
| `FREE_DAILY_QUESTIONS` | 10 | Free-tier daily allowance |
| `PORT` (implicit) | 8080 | API listen port |

## Verification after any deploy

```bash
curl -s https://jerryboganda.github.io/medicalos/version.json   # sha == HEAD
curl -s https://jerryboganda.github.io/medicalos/ -o /dev/null -w '%{http_code}\n'  # 200
curl -s http://<api-host>:8080/healthz   # ok (once the API container runs)
```
