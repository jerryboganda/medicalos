#!/usr/bin/env bash
# One-time VPS bootstrap for medicalos.polytronx.com.
# Run over SSH (root@vps). Idempotent: safe to re-run.
set -euo pipefail
DB=/opt/docker/nginx-proxy-manager/data/database.sqlite

echo "[1/5] platform credentials present"
test -f /opt/platform/projects/medicalos.env
# shellcheck disable=SC1091
. /opt/platform/projects/medicalos.env
: "${PLATFORM_PG_URL:?missing PLATFORM_PG_URL}"

echo "[2/5] NPM proxy host 47 -> medicalos-web:80 (plain HTTP until LE cert)"
python3 - "$DB" <<'PY'
import sqlite3, sys
db = sys.argv[1]
c = sqlite3.connect(db)
c.execute("UPDATE proxy_host SET meta=?, modified_on=datetime('now') WHERE id=47",
          ('{"nginx_online":false,"nginx_err":null}',))
c.commit()
print("row47 reset:", c.execute("SELECT substr(meta,1,60) FROM proxy_host WHERE id=47").fetchone()[0])
PY

echo "[3/5] restart NPM, wait for 47.conf"
docker restart nginx-proxy-manager-app-1 >/dev/null
for i in $(seq 1 12); do
  if [ -f /opt/docker/nginx-proxy-manager/data/nginx/proxy_host/47.conf ]; then
    echo "47.conf generated after ${i}0s"
    break
  fi
  sleep 10
done
ls /opt/docker/nginx-proxy-manager/data/nginx/proxy_host/47.conf

echo "[4/5] direct-origin check (bypass Cloudflare)"
curl -sI --max-time 15 -H "Host: medicalos.polytronx.com" http://127.0.0.1/ | head -5 || true

echo "[5/5] public check (through Cloudflare)"
curl -sI --max-time 20 http://medicalos.polytronx.com | head -5 || true
echo "BOOTSTRAP DONE — next: request LE cert in NPM UI for host 47, then ssl_forced=1"
