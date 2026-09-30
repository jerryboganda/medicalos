"""Mirror the gated Zitadel phase into deploy.yml's inline script (CRLF-safe)."""
import io

path = ".github/workflows/deploy.yml"
with io.open(path, encoding="utf-8", newline="") as f:
    s = f.read()
nl = "\r\n" if "\r\n" in s else "\n"

old_env = (
    "          ZITADEL_IDP_APPLE: ${{ secrets.VPS_ZITADEL_IDP_APPLE }}" + nl
    + "        with:" + nl
)
new_env = (
    "          ZITADEL_IDP_APPLE: ${{ secrets.VPS_ZITADEL_IDP_APPLE }}" + nl
    + "          VPS_ZITADEL_DB_PASSWORD: ${{ secrets.VPS_ZITADEL_DB_PASSWORD }}" + nl
    + "          VPS_ZITADEL_MASTERKEY: ${{ secrets.VPS_ZITADEL_MASTERKEY }}" + nl
    + "          VPS_ZITADEL_ADMIN_PASSWORD: ${{ secrets.VPS_ZITADEL_ADMIN_PASSWORD }}" + nl
    + "          VPS_ZITADEL_EXTERNAL_URL: ${{ secrets.VPS_ZITADEL_EXTERNAL_URL }}" + nl
    + "        with:" + nl
)
assert s.count(old_env) == 1, "env anchor"
s = s.replace(old_env, new_env)

old_envs = "          envs: OIDC_CREDENTIAL_KEY,PACK_SIGNING_KEY,ZITADEL_ISSUER,ZITADEL_CLIENT_ID,ZITADEL_CLIENT_SECRET,ZITADEL_PROJECT_ID,ZITADEL_IDP_GOOGLE,ZITADEL_IDP_APPLE"
new_envs = "          envs: OIDC_CREDENTIAL_KEY,PACK_SIGNING_KEY,ZITADEL_ISSUER,ZITADEL_CLIENT_ID,ZITADEL_CLIENT_SECRET,ZITADEL_PROJECT_ID,ZITADEL_IDP_GOOGLE,ZITADEL_IDP_APPLE,VPS_ZITADEL_DB_PASSWORD,VPS_ZITADEL_MASTERKEY,VPS_ZITADEL_ADMIN_PASSWORD,VPS_ZITADEL_EXTERNAL_URL"
assert s.count(old_envs) == 1, "envs anchor"
s = s.replace(old_envs, new_envs)

body = [
    "# ---- Platform identity provider (Zitadel), owner decision 2026-09-30 ----",
    "# Runs only when all four VPS_ZITADEL_* inputs exist; otherwise fully inert",
    "# and platform sign-in stays disabled (password + institution SSO only).",
    "# One-time owner setup: the zitadel DB + user on platform-postgres (DBA",
    "# step), the four GitHub secrets, and DNS + NPM host for the external URL.",
    'ZDB_PASSWORD="${VPS_ZITADEL_DB_PASSWORD:-}"',
    'ZMASTERKEY="${VPS_ZITADEL_MASTERKEY:-}"',
    'ZADMIN_PASSWORD="${VPS_ZITADEL_ADMIN_PASSWORD:-}"',
    'ZEXTERNAL="${VPS_ZITADEL_EXTERNAL_URL:-}"',
    'if [ -n "$ZDB_PASSWORD" ] && [ -n "$ZMASTERKEY" ] && [ -n "$ZADMIN_PASSWORD" ] && [ -n "$ZEXTERNAL" ]; then',
    '  ZIMAGE="ghcr.io/zitadel/zitadel:v4.16.0"',
    '  echo "[medicalos] pulling $ZIMAGE (identity provider)"',
    '  docker pull "$ZIMAGE"',
    "  docker rm -f medicalos-zitadel 2>/dev/null || true",
    "  docker run -d --name medicalos-zitadel --restart unless-stopped \\",
    '    --cpus "${VPS_ZITADEL_CPUS:-0.2}" --memory "${VPS_ZITADEL_MEMORY:-256m}" \\',
    "    --network platform \\",
    '    -e ZITADEL_DATABASE_POSTGRES_HOST="platform-postgres" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_PORT="5432" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_DATABASE="medicalos_zitadel" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_USER_USERNAME="medicalos_zitadel" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_USER_PASSWORD="$ZDB_PASSWORD" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_ADMIN_USERNAME="medicalos_zitadel" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_ADMIN_PASSWORD="$ZDB_PASSWORD" \\',
    '    -e ZITADEL_EXTERNALPORT="443" \\',
    '    -e ZITADEL_EXTERNALSECURE="true" \\',
    '    -e ZITADEL_EXTERNALURL="$ZEXTERNAL" \\',
    '    -e ZITADEL_MASTERKEY="$ZMASTERKEY" \\',
    '    -e ZITADEL_FIRSTINSTANCE_ORG_HUMAN_PASSWORD="$ZADMIN_PASSWORD" \\',
    '    "$ZIMAGE" start-from-init --masterkey "$ZMASTERKEY"',
    "  echo \"[medicalos] zitadel container: $(docker inspect medicalos-zitadel --format '{{.Config.Image}}')\"",
    '  echo "[medicalos] NEXT (docs/deployment/vps-medicalos.md): run"',
    '  echo "[medicalos] infra/zitadel/provision.sh against $ZEXTERNAL, then set"',
    '  echo "[medicalos] the VPS_ZITADEL_ISSUER/CLIENT_ID/CLIENT_SECRET secrets."',
    "else",
    '  echo "[medicalos] zitadel: not configured (VPS_ZITADEL_* inputs missing) — platform sign-in stays disabled"',
    "fi",
]
indented = ["            " + b if b else "" for b in body]

anchor = '            echo "[medicalos] live on medicalos.polytronx.com @ $SHA"'
assert s.count(anchor) == 1, "script anchor"
block = anchor + nl + nl.join(indented) + nl
s = s.replace(anchor, block, 1)

with io.open(path, "w", encoding="utf-8", newline="") as f:
    f.write(s)
print("deploy.yml mirrored")
