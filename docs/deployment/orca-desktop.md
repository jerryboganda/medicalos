# Deploying Medical OS locally on Orca Desktop (A–Z)

Local staging on this Windows PC, managed by [Orca Desktop](https://github.com/edvin/orca)
(the open-source Docker Desktop alternative installed at `D:\Orca Desktop`).
Every image is **prebuilt by GitHub Actions** — nothing compiles locally
(AGENTS.md compute policy). Production on the VPS
(`docs/deployment/production.md`, `docs/deployment/vps-medicalos.md`) is a
separate flow and is untouched by everything below.

## Topology

```
Orca Desktop (GUI + daemon) ──► Docker engine (WSL2)
  compose project "medicalos-local"   (infra/local/docker-compose.yml)
  ├─ postgres        postgres:17-alpine                        internal only
  ├─ medicalos-api   ghcr.io/jerryboganda/medicalos-api:<tag>  127.0.0.1:18080
  ├─ medicalos-web   ghcr.io/jerryboganda/medicalos-web:<tag>  127.0.0.1:8081
  └─ medicalos-site  ghcr.io/jerryboganda/medicalos-site:<tag> 127.0.0.1:8082  (profile `full`)
```

Resource ceilings: 0.5/0.5/0.25/0.1 CPU and 512/512/128/64 MB — under
1.5 GB RAM and 1.4 cores even at 100% (normal idle is well below 1 GB).
The API applies all migrations itself on first boot; seeding is one command.

---

## A. Prerequisites (one-time, host)

1. **Orca Desktop** is installed (`D:\Orca Desktop`: `orca-gui.exe`,
   `orca-daemon.exe`, `orca.exe`). Launch `orca-gui.exe`.
2. **WSL2** is already present on this machine (`D:\wsl` holds the `oet-ci`
   CI-runner distro). Leave that distro alone — see the engine step below.
3. **Git Bash** (already used for this repo) provides `curl` and `openssl`
   for the scripts.

## B. Container engine — two options

- **Option A (default): let Orca own its engine.** On first run Orca's setup
  wizard enables WSL2 and installs its own Ubuntu + Docker Engine distro.
  Costs ~1–2 GB disk; keeps your `oet-ci` CI-runner distro 100% untouched.
- **Option B (leaner, riskier): reuse the `oet-ci` dockerd.** Inside
  `wsl -d oet-ci` (as root) add a localhost-only TCP listener:
  `systemctl edit docker` →
  `[Service]\nExecStart=/usr/bin/dockerd -H fd:// -H tcp://127.0.0.1:2375`,
  then `systemctl restart docker`; Orca's engine endpoint becomes
  `tcp://127.0.0.1:2375` (WSL2 forwards localhost to Windows). Saves the
  extra distro but touches a distro another project's CI depends on — do
  this only if disk/RAM is tight. `oet-ci` has `automount enabled=false`,
  so files must be piped in (e.g. `wsl -d oet-ci -u root tee ...`).

## C. WSL memory budget

Check `%UserProfile%\.wslconfig` (the `D:\wsl\swap.vhdx` suggests one exists).
If it has no `[wsl2]` caps, add `memory=4GB`, `processors=4`,
`autoMemoryReclaim=gradual`, then `wsl --shutdown` once. If caps already
exist (tuned for `oet-ci` CI builds), leave them — the compose ceilings above
bound this stack regardless. The budget is shared with `oet-ci`.

## D. GHCR access (required — packages are private)

The repo is private, so its GHCR packages are private too.

1. Create a GitHub personal access token with **`read:packages`** scope.
2. Give it to Docker: either Orca → **Registries → ghcr.io** (+ your token),
   or in the engine's shell: `echo <TOKEN> | docker login ghcr.io -u <github-username> --password-stdin`.
3. Prove it: `docker manifest inspect ghcr.io/jerryboganda/medicalos-api:latest`
   (401 without the login).

## E. Configure and deploy

1. `cp infra/local/.env.example infra/local/.env`, then edit:
   - `POSTGRES_PASSWORD` — any strong string.
   - `PACK_SIGNING_KEY` — generate: `openssl rand -base64 48` (must be
     ≥ 32 non-whitespace bytes or the API refuses to boot).
   - `IMAGE_TAG` — pin to the SHA of the deploy you want (default `latest`
     tracks the last green `main` deploy; both tags exist on GHCR).
2. Deploy — pick one:
   - **Orca GUI:** Stacks → New → pick `infra/local/docker-compose.yml` as
     the compose file and `infra/local/.env` as its env file → Deploy. Orca
     runs `docker compose up -d` and gives you per-container CPU/RAM charts.
   - **CLI:** `cd "/d/Projects/Medical OS/infra/local" && docker compose up -d`
     (first pull is ~400 MB of images, once).
3. First boot order: postgres healthy → api applies the 57 migrations (watch
   `docker compose logs -f medicalos-api` once) → web up.
4. Seed demo content (as you chose): `docker compose exec medicalos-api api --seed`.

## F. Verify

```bash
bash infra/local/verify.sh
```

Checks `/healthz` = ok, the SvelteKit shell, the static
`/api/version.json` provenance file, and the `/api/` proxy — then opens:

| URL | What |
| --- | --- |
| http://localhost:8081 | The app (web SPA + same-origin API proxy) |
| http://127.0.0.1:18080 | API directly (debugging; 18080 because 8080 belongs to another project on the shared engine) |
| http://localhost:8082 | Marketing site (after step G) |

**Admin console.** Admin access in this app is a logged-in user account plus
the shared `ADMIN_TOKEN` (there is no separate admin role): register an
account in the app, then set `localStorage.setItem('mlos_admin', '<ADMIN_TOKEN
from infra/local/.env>')` in the browser and reload — the client sends it as
the `x-admin-token` header, and `/api/v1/admin/*` requires both the session
and the token (session-only → 401, token-only → 401, both → 200).

## G. Marketing site (profile `full`)

`medicalos-site` needs its GHCR image, produced by the `site-image` job in
`deploy.yml` — which waits for the next owner-mandated **repo PUBLIC →
build/verify → PRIVATE** CI window (private-repo Actions billing limit).
When the image exists:

```bash
docker compose --profile full up -d   # from infra/local
```

Sooner fallback (explicit, justified exception to the compute policy — an
Astro build is seconds-light): `npm install && npm run build -w @medical-os/site`,
then point any static server at `apps/site/dist`. Prefer the CI image.

## H. Update & rollback

Deploys are SHA-pinned — `:latest` drift is never involved:

```bash
# update: set IMAGE_TAG=<new sha> in .env, then
docker compose pull && docker compose up -d
# rollback: set IMAGE_TAG=<previous sha>, same two commands
```

Then `docker image prune -f` to drop the superseded layers. In Orca, use the
stack's Pull + Up; the version you're on is always visible at
`http://localhost:8081/api/version.json` and via `verify.sh`.

## I. Backups

```bash
bash infra/local/backup.sh    # pg_dump -> .backups/medicalos-<ts>.sql.gz, keeps 7
```

Nightly via Task Scheduler (admin prompt):

```bat
schtasks /create /tn "medicalos-local-backup" /sc daily /st 03:30 /tr "\"C:\Program Files\Git\bin\bash.exe\" -lc 'cd \"/d/Projects/Medical OS\" && bash infra/local/backup.sh'"
```

`.backups/` is gitignored. Restoring: `gunzip -c <file> | docker compose exec -T postgres psql -U medicalos -d medicalos`.

## J. Day-to-day & teardown

- **Idle:** stop the stack in Orca (or `docker compose stop`) — the WSL VM
  then idles to near-zero; `autoMemoryReclaim` returns borrowed RAM.
- **Fresh start:** `docker compose down -v` wipes the volume (run
  `backup.sh` first); next `up` re-applies migrations from scratch.
- **Uninstall:** `docker compose down -v`, then remove the stack in Orca and
  (Option A only) the Orca engine distro via `wsl --unregister <its-distro>`.
  `D:\wsl\oet-ci` is never touched by any step here.

## Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| Image pulls 401/403 | GHCR login missing (step D). Packages are private. |
| api restarts, logs "PACK_SIGNING_KEY" | Key < 32 non-whitespace bytes — regenerate. |
| api can't reach postgres | Deploy without `.env` (interpolation error names it) or password mismatch — both come from the same file. |
| Port already in use | This engine is shared (Freebuff2API holds 8080, Orca's gateway 80/443 — our mapping already dodges them); for a new clash, change the left side of the `ports:` mapping in `docker-compose.yml`. |
| Everything OOM-killed | WSL budget too small (step C) — the stack needs ≤ ~1.5 GB. |
| Migrations run twice | They don't — schema applies at API startup against the persisted volume; only a `down -v` resets it. |
