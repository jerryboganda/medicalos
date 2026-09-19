#!/usr/bin/env bash
# VPS deploy for medicalos.polytronx.com — runs ON the VPS via SSH from the
# GitHub Actions `deploy-vps` job. Pulls GHCR images (never builds), recreates
# the two containers on the shared networks, health-checks, reports versions.
#
# Usage: deploy.sh <sha>   (VPS_DATABASE_URL_AS_ARG exported by CI; the
# appleboy action passes `envs` through, DATABASE_URL is set from it below
# so the secret never appears in the SSH command line.)
set -euo pipefail

SHA="${1:?usage: deploy.sh <sha>}"
DATABASE_URL="${DATABASE_URL:-${VPS_DATABASE_URL_AS_ARG:?VPS_DATABASE_URL secret missing}}"
REGISTRY="${REGISTRY:-ghcr.io/jerryboganda}"
# The GHCR packages for this repo are public: pulls are anonymous, no
# registry login is wired through CI by design. (A failed `docker login`
# here once masked a no-op deploy as success — the script must fail loudly
# if pulls fail, and every step below echoes what it did.)
API_IMAGE="$REGISTRY/medicalos-api:$SHA"
WEB_IMAGE="$REGISTRY/medicalos-web:$SHA"
: "${DATABASE_URL:?DATABASE_URL must be exported (VPS_DATABASE_URL secret)}"

echo "[medicalos] pulling $API_IMAGE $WEB_IMAGE"
docker pull "$API_IMAGE"
docker pull "$WEB_IMAGE"
echo "[medicalos] pulled:"
docker images --format '{{.Repository}}:{{.Tag}} {{.ID}}' \
  | grep -E "medicalos-(api|web):$SHA" || {
  echo "pulled images do not include SHA $SHA" >&2
  exit 1
}

for net in platform nginx-proxy-manager_default; do
  docker network inspect "$net" >/dev/null 2>&1 \
    || { echo "missing docker network $net" >&2; exit 1; }
done

echo "[medicalos] recreating medicalos-api (image: $API_IMAGE)"
docker rm -f medicalos-api 2>/dev/null || true
docker run -d --name medicalos-api --restart unless-stopped \
  --cpus "1.0" --memory "1g" \
  --network platform \
  -e DATABASE_URL="$DATABASE_URL" \
  -e MIN_TIME_LIMIT_SECONDS=30 \
  -e FREE_DAILY_QUESTIONS=10 \
  "$API_IMAGE"
echo "[medicalos] api container: $(docker inspect medicalos-api --format '{{.Config.Image}}')"

echo "[medicalos] recreating medicalos-web (image: $WEB_IMAGE)"
docker rm -f medicalos-web 2>/dev/null || true
docker run -d --name medicalos-web --restart unless-stopped \
  --cpus "0.5" --memory "256m" \
  --network platform \
  "$WEB_IMAGE"
docker network connect nginx-proxy-manager_default medicalos-web 2>/dev/null || true
echo "[medicalos] web container: $(docker inspect medicalos-web --format '{{.Config.Image}}')"

echo "[medicalos] health-check (API + web through the container network)"
for i in $(seq 1 30); do
  # The debian-slim API image has no wget/curl: probe from a throwaway
  # curl container on the same network instead.
  if docker run --rm --network platform curlimages/curl:8.5.0 -sf http://medicalos-api:8080/healthz 2>/dev/null | grep -q ok \
    && docker run --rm --network platform curlimages/curl:8.5.0 -sf "http://medicalos-web/" 2>/dev/null | grep -q 'data-sveltekit-preload-data'; then
    echo "[medicalos] api + web healthy"
    break
  fi
  [ "$i" -eq 30 ] && { echo "stack never became healthy" >&2; exit 1; }
  sleep 2
done

echo "[medicalos] versions:"
docker inspect medicalos-api --format '  api: {{.Config.Image}} {{.Image}}' | head -c 200; echo
docker inspect medicalos-web --format '  web: {{.Config.Image}} {{.Image}}' | head -c 200; echo
echo "[medicalos] live on medicalos.polytronx.com @ $SHA"
