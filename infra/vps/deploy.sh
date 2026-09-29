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
PUBLIC_API_BASE_URL="${PUBLIC_API_BASE_URL:-https://medicalos.polytronx.com/api}"
PUBLIC_APP_URL="${PUBLIC_APP_URL:-https://medicalos.polytronx.com}"
OIDC_CREDENTIAL_KEY="${OIDC_CREDENTIAL_KEY:-}"
PACK_SIGNING_KEY="${PACK_SIGNING_KEY:-}"
# Platform sign-in (Zitadel) — empty = disabled at runtime (password login
# and institution SSO keep working). The trio below is all-or-nothing: the
# API silently disables platform sign-in when any is missing, so a partial
# set must fail loudly here instead.
ZITADEL_ISSUER="${ZITADEL_ISSUER:-}"
ZITADEL_CLIENT_ID="${ZITADEL_CLIENT_ID:-}"
ZITADEL_CLIENT_SECRET="${ZITADEL_CLIENT_SECRET:-}"
ZITADEL_PROJECT_ID="${ZITADEL_PROJECT_ID:-}"
ZITADEL_IDP_GOOGLE="${ZITADEL_IDP_GOOGLE:-}"
ZITADEL_IDP_APPLE="${ZITADEL_IDP_APPLE:-}"
if [ -n "${ZITADEL_ISSUER}${ZITADEL_CLIENT_ID}${ZITADEL_CLIENT_SECRET}" ]; then
  for v in ZITADEL_ISSUER ZITADEL_CLIENT_ID ZITADEL_CLIENT_SECRET; do
    [ -n "${!v}" ] || { echo "$v is empty: ZITADEL_ISSUER, ZITADEL_CLIENT_ID and ZITADEL_CLIENT_SECRET are required together" >&2; exit 1; }
  done
fi
# The GHCR packages for this repo are public: pulls are anonymous, no
# registry login is wired through CI by design. (A failed `docker login`
# here once masked a no-op deploy as success — the script must fail loudly
# if pulls fail, and every step below echoes what it did.)
API_IMAGE="$REGISTRY/medicalos-api:$SHA"
WEB_IMAGE="$REGISTRY/medicalos-web:$SHA"
: "${DATABASE_URL:?DATABASE_URL must be exported (VPS_DATABASE_URL secret)}"
if [ "${#PACK_SIGNING_KEY}" -lt 32 ] || [ -z "${PACK_SIGNING_KEY//[[:space:]]/}" ]; then
  echo "VPS_PACK_SIGNING_KEY must contain at least 32 non-whitespace bytes" >&2
  exit 1
fi

echo "[medicalos] pulling $API_IMAGE $WEB_IMAGE"
docker pull "$API_IMAGE"
docker pull "$WEB_IMAGE"
echo "[medicalos] pulled:"
docker images --format '{{.Repository}}:{{.Tag}} {{.ID}}' \
  | grep -E "medicalos-(api|web):$SHA" || {
  echo "pulled images do not include SHA $SHA" >&2
  exit 1
}

# OPS-03: verify the deploy workflow's keyless signatures ON the VPS before
# anything is recreated — an unsigned or re-tagged image aborts the deploy
# with the current containers still serving.
COSIGN_VERSION="${COSIGN_VERSION:-2.4.1}"
if ! command -v cosign >/dev/null 2>&1; then
  echo "[medicalos] installing cosign v$COSIGN_VERSION"
  curl -sfL "https://github.com/sigstore/cosign/releases/download/v${COSIGN_VERSION}/cosign-linux-amd64" \
    -o /usr/local/bin/cosign
  chmod +x /usr/local/bin/cosign
fi
COSIGN_ID_REGEXP='^https://github.com/jerryboganda/medicalos/\.github/workflows/deploy\.yml@refs/heads/main$'
for image in "$API_IMAGE" "$WEB_IMAGE"; do
  cosign verify "$image" \
    --certificate-identity-regexp "$COSIGN_ID_REGEXP" \
    --certificate-oidc-issuer https://token.actions.githubusercontent.com \
    >/dev/null \
    || { echo "signature verification failed for $image — aborting before any container is touched" >&2; exit 1; }
  echo "[medicalos] signature verified: $image"
done

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
  -e PUBLIC_API_BASE_URL="$PUBLIC_API_BASE_URL" \
  -e PUBLIC_APP_URL="$PUBLIC_APP_URL" \
  -e OIDC_CREDENTIAL_KEY="$OIDC_CREDENTIAL_KEY" \
  -e PACK_SIGNING_KEY="$PACK_SIGNING_KEY" \
  -e ZITADEL_ISSUER="$ZITADEL_ISSUER" \
  -e ZITADEL_CLIENT_ID="$ZITADEL_CLIENT_ID" \
  -e ZITADEL_CLIENT_SECRET="$ZITADEL_CLIENT_SECRET" \
  -e ZITADEL_PROJECT_ID="$ZITADEL_PROJECT_ID" \
  -e ZITADEL_IDP_GOOGLE="$ZITADEL_IDP_GOOGLE" \
  -e ZITADEL_IDP_APPLE="$ZITADEL_IDP_APPLE" \
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
# Migrations 0050-0059 (incl. RLS + backfills) can push API startup past
# 60s under load — allow 90 iterations x 2s. (Kept in sync with the inline
# deploy-vps script in .github/workflows/deploy.yml.)
for i in $(seq 1 90); do
  # The debian-slim API image has no wget/curl: probe from a throwaway
  # curl container on the same network instead.
  if docker run --rm --network platform curlimages/curl:8.5.0 -sf http://medicalos-api:8080/healthz 2>/dev/null | grep -q ok \
    && docker run --rm --network platform curlimages/curl:8.5.0 -sf "http://medicalos-web/" 2>/dev/null | grep -q 'data-sveltekit-preload-data'; then
    echo "[medicalos] api + web healthy"
    break
  fi
  [ "$i" -eq 90 ] && { echo "stack never became healthy" >&2; exit 1; }
  sleep 2
done

echo "[medicalos] versions:"
docker inspect medicalos-api --format '  api: {{.Config.Image}} {{.Image}}' | head -c 200; echo
docker inspect medicalos-web --format '  web: {{.Config.Image}} {{.Image}}' | head -c 200; echo
echo "[medicalos] live on medicalos.polytronx.com @ $SHA"
