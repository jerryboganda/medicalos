#!/usr/bin/env bash
# VPS deploy for medicalos.polytronx.com — runs ON the VPS via SSH from the
# GitHub Actions `deploy-vps` job. Pulls GHCR images (never builds), recreates
# the two containers on the shared networks, health-checks, reports versions.
#
# Usage: deploy.sh <sha>   (DATABASE_URL exported in the environment)
set -euo pipefail

SHA="${1:?usage: deploy.sh <sha>}"
REGISTRY="${REGISTRY:-ghcr.io/jerryboganda}"
API_IMAGE="$REGISTRY/medicalos-api:$SHA"
WEB_IMAGE="$REGISTRY/medicalos-web:$SHA"
: "${DATABASE_URL:?DATABASE_URL must be exported (VPS_DATABASE_URL secret)}"

echo "[medicalos] pulling $API_IMAGE $WEB_IMAGE"
docker pull "$API_IMAGE"
docker pull "$WEB_IMAGE"

for net in platform nginx-proxy-manager_default; do
  docker network inspect "$net" >/dev/null 2>&1 \
    || { echo "missing docker network $net" >&2; exit 1; }
done

echo "[medicalos] recreating medicalos-api"
docker rm -f medicalos-api 2>/dev/null || true
docker run -d --name medicalos-api --restart unless-stopped \
  --cpus "1.0" --memory "1g" \
  --network platform \
  -e DATABASE_URL="$DATABASE_URL" \
  -e MIN_TIME_LIMIT_SECONDS=30 \
  -e FREE_DAILY_QUESTIONS=10 \
  "$API_IMAGE"

echo "[medicalos] recreating medicalos-web"
docker rm -f medicalos-web 2>/dev/null || true
docker run -d --name medicalos-web --restart unless-stopped \
  --cpus "0.5" --memory "256m" \
  --network platform \
  "$WEB_IMAGE"
docker network connect nginx-proxy-manager_default medicalos-web 2>/dev/null || true

echo "[medicalos] health-check (API + web through the container network)"
API_IP=$(docker inspect medicalos-api --format '{{.NetworkSettings.Networks.platform.IPAddress}}')
WEB_IP=$(docker inspect medicalos-web --format '{{.NetworkSettings.Networks.platform.IPAddress}}')
for i in $(seq 1 30); do
  if docker exec medicalos-api wget -q -O - http://127.0.0.1:8080/healthz 2>/dev/null | grep -q ok \
    && wget -q -O - "http://$WEB_IP/" 2>/dev/null | grep -q 'data-sveltekit-preload-data'; then
    echo "[medicalos] api ($API_IP) + web ($WEB_IP) healthy"
    break
  fi
  [ "$i" -eq 30 ] && { echo "stack never became healthy" >&2; exit 1; }
  sleep 2
done

echo "[medicalos] versions:"
docker inspect medicalos-api --format '  api: {{.Config.Image}} {{.Image}}' | head -c 200; echo
docker inspect medicalos-web --format '  web: {{.Config.Image}} {{.Image}}' | head -c 200; echo
echo "[medicalos] live on medicalos.polytronx.com @ $SHA"
