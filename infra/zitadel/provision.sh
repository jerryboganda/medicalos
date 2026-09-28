#!/usr/bin/env bash
# Idempotent Zitadel setup for Medical OS (.scratch/auth-zitadel/spec.md).
# Creates — only when missing — the `medical-os` project, its platform roles,
# the API's confidential OIDC app, and (optionally) the owner's
# platform_owner grant. Safe to re-run: existing objects are reported
# "exists" and left alone. Every change prints a line starting "created".
#
# Env:
#   ZITADEL_URL        issuer, e.g. http://auth.medicalos.localhost:8083
#   ZITADEL_PAT        provisioner machine-user token (IAM owner), or
#   ZITADEL_PAT_FILE   file holding it (first-instance PatPath)
#   API_CALLBACK_URL   e.g. http://localhost:8081/api/v1/auth/oidc/callback
#   APP_LOGOUT_URL     e.g. http://localhost:8081/login (optional)
#   OWNER_USERNAME     human user to grant platform_owner (optional)
#   OUT_ENV            where to write ZITADEL_* for the API (default
#                      ./.env.zitadel — gitignored by the `.env.*` rule)
#   CONNECT_TO         optional curl --connect-to (host:port:ip:port) for
#                      hosts whose resolver doesn't map *.localhost
set -euo pipefail

: "${ZITADEL_URL:?set ZITADEL_URL}"
: "${API_CALLBACK_URL:?set API_CALLBACK_URL}"
PAT="${ZITADEL_PAT:-}"
if [ -z "$PAT" ] && [ -n "${ZITADEL_PAT_FILE:-}" ]; then PAT="$(tr -d '\r\n' < "$ZITADEL_PAT_FILE")"; fi
: "${PAT:?set ZITADEL_PAT or ZITADEL_PAT_FILE}"
OUT_ENV="${OUT_ENV:-./.env.zitadel}"
PROJECT_NAME="medical-os"
APP_NAME="medical-os-api"
# key:display name, one per line (display names contain spaces).
ROLES="platform_owner:Platform owner
support:Support
billing_admin:Billing administrator
author:Author
medical_reviewer:Medical reviewer
examiner:Examiner"

command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }
# Zitadel answers 503 for a few seconds after /debug/ready while its
# projections catch up; curl retries 503 (a transient error) with backoff.
CURL=(curl -sS --fail-with-body --retry 20 --retry-delay 3 -H "Authorization: Bearer $PAT" -H "Content-Type: application/json")
[ -n "${CONNECT_TO:-}" ] && CURL+=(--connect-to "$CONNECT_TO")

api() { # method path [json]
  "${CURL[@]}" -X "$1" "$ZITADEL_URL$2" ${3:+--data "$3"}
}
eq() { printf '{"%s":{"%s":%s,"method":"TEXT_QUERY_METHOD_EQUALS"}}' "$1" "$2" "$(jq -Rn --arg v "$3" '$v')"; }

# ---- project ---------------------------------------------------------------
PROJECT_ID="$(api POST /management/v1/projects/_search "{\"queries\":[$(eq nameQuery name "$PROJECT_NAME")]}" | jq -r '.result[0].id // empty')"
if [ -z "$PROJECT_ID" ]; then
  # Roles ride in tokens (role assertion); no project-level access check so
  # learners without any grant can still sign in.
  PROJECT_ID="$(api POST /management/v1/projects "{\"name\":\"$PROJECT_NAME\",\"projectRoleAssertion\":true,\"projectRoleCheck\":false,\"hasProjectCheck\":false}" | jq -r '.id')"
  echo "created project $PROJECT_NAME ($PROJECT_ID)"
else
  echo "exists  project $PROJECT_NAME ($PROJECT_ID)"
fi

# ---- roles (authz.rs platform_grants) ---------------------------------------
EXISTING_ROLES="$(api POST "/management/v1/projects/$PROJECT_ID/roles/_search" '{}' | jq -r '.result[]?.key')"
while IFS=: read -r key name; do
  if grep -qx "$key" <<<"$EXISTING_ROLES"; then
    echo "exists  role $key"
  else
    api POST "/management/v1/projects/$PROJECT_ID/roles" \
      "$(jq -n --arg k "$key" --arg n "$name" '{roleKey:$k,displayName:$n}')" >/dev/null
    echo "created role $key"
  fi
done <<<"$ROLES"

# ---- the API's confidential OIDC app ------------------------------------------
APP="$(api POST "/management/v1/projects/$PROJECT_ID/apps/_search" "{\"queries\":[$(eq nameQuery name "$APP_NAME")]}" | jq -c '.result[0] // empty')"
CLIENT_SECRET=""
if [ -z "$APP" ]; then
  DEV_MODE=false
  case "$API_CALLBACK_URL" in http://*) DEV_MODE=true ;; esac # http redirect = local only
  BODY="$(jq -n --arg name "$APP_NAME" --arg cb "$API_CALLBACK_URL" --arg lo "${APP_LOGOUT_URL:-}" --argjson dev "$DEV_MODE" '{
    name: $name,
    redirectUris: [$cb],
    postLogoutRedirectUris: ([$lo] | map(select(. != ""))),
    responseTypes: ["OIDC_RESPONSE_TYPE_CODE"],
    grantTypes: ["OIDC_GRANT_TYPE_AUTHORIZATION_CODE", "OIDC_GRANT_TYPE_REFRESH_TOKEN"],
    appType: "OIDC_APP_TYPE_WEB",
    authMethodType: "OIDC_AUTH_METHOD_TYPE_BASIC",
    version: "OIDC_VERSION_1_0",
    accessTokenType: "OIDC_TOKEN_TYPE_BEARER",
    idTokenRoleAssertion: true,
    idTokenUserinfoAssertion: true,
    devMode: $dev
  }')"
  CREATED="$(api POST "/management/v1/projects/$PROJECT_ID/apps/oidc" "$BODY")"
  CLIENT_ID="$(jq -r '.clientId' <<<"$CREATED")"
  CLIENT_SECRET="$(jq -r '.clientSecret' <<<"$CREATED")"
  echo "created app $APP_NAME ($CLIENT_ID)"
else
  CLIENT_ID="$(jq -r '.oidcConfig.clientId' <<<"$APP")"
  echo "exists  app $APP_NAME ($CLIENT_ID)"
fi

# ---- owner grant (optional) ------------------------------------------------------
if [ -n "${OWNER_USERNAME:-}" ]; then
  # Zitadel may store the username with its org domain (owner@org.domain).
  USERS="$(api POST /management/v1/users/_search "{\"queries\":[{\"userNameQuery\":{\"userName\":$(jq -Rn --arg v "$OWNER_USERNAME" '$v'),\"method\":\"TEXT_QUERY_METHOD_STARTS_WITH\"}}]}")"
  OWNER_ID="$(jq -r --arg u "$OWNER_USERNAME" '[.result[]? | select(.userName == $u or (.userName | startswith($u + "@")))][0].id // empty' <<<"$USERS")"
  [ -n "$OWNER_ID" ] || { echo "owner user $OWNER_USERNAME not found; users seen: $(jq -c '[.result[]?.userName]' <<<"$USERS")" >&2; exit 1; }
  GRANTED="$(api POST /management/v1/users/grants/_search "{\"queries\":[{\"userIdQuery\":{\"userId\":\"$OWNER_ID\"}},{\"projectIdQuery\":{\"projectId\":\"$PROJECT_ID\"}}]}" | jq -r '.result[0].id // empty')"
  if [ -z "$GRANTED" ]; then
    api POST "/management/v1/users/$OWNER_ID/grants" "{\"projectId\":\"$PROJECT_ID\",\"roleKeys\":[\"platform_owner\"]}" >/dev/null
    echo "created grant platform_owner -> $OWNER_USERNAME"
  else
    echo "exists  grant for $OWNER_USERNAME"
  fi
fi

# ---- hand the client settings to the API ------------------------------------------
# The secret is only returned when the app is created; later runs keep the
# file that holds it. A lost file needs a deliberate secret rotation in the
# Zitadel console — never a silent one here.
if [ -n "$CLIENT_SECRET" ]; then
  umask 077
  cat > "$OUT_ENV" <<EOF
ZITADEL_ISSUER=$ZITADEL_URL
ZITADEL_CLIENT_ID=$CLIENT_ID
ZITADEL_CLIENT_SECRET=$CLIENT_SECRET
ZITADEL_PROJECT_ID=$PROJECT_ID
EOF
  echo "created $OUT_ENV (client secret; keep it private)"
elif [ ! -f "$OUT_ENV" ]; then
  echo "WARNING: $OUT_ENV missing and the app already exists — rotate the client secret in the Zitadel console" >&2
fi
echo "done: project $PROJECT_ID, client $CLIENT_ID"
