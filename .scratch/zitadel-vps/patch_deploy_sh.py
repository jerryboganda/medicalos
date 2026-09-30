"""Append the gated Zitadel phase to infra/vps/deploy.sh (CRLF-safe)."""
import io

path = "infra/vps/deploy.sh"
with io.open(path, encoding="utf-8", newline="") as f:
    s = f.read()
nl = "\r\n" if "\r\n" in s else "\n"

anchor = 'echo "[medicalos] live on medicalos.polytronx.com @ $SHA"'
assert s.count(anchor) == 1

extension_lines = [
    "",
    "# ---- Platform identity provider (Zitadel), owner decision 2026-09-30 ----",
    "# Runs only when all four VPS_ZITADEL_* inputs exist; otherwise fully inert",
    "# and platform sign-in stays disabled (password + institution SSO only).",
    "# One-time owner setup before this activates:",
    "#   1. A dedicated database + user on platform-postgres (DBA step; the",
    "#      deploy credentials are least-privilege and cannot create databases).",
    "#   2. GitHub secrets: VPS_ZITADEL_DB_PASSWORD (the zitadel DB user's password), VPS_ZITADEL_MASTERKEY (exactly",
    "#      32 chars), VPS_ZITADEL_ADMIN_PASSWORD, VPS_ZITADEL_EXTERNAL_URL.",
    "#   3. DNS for the external URL + an NPM proxy host with a certificate",
    "#      (browsers and this VPS both reach Zitadel through that URL).",
    'ZMASTERKEY="${VPS_ZITADEL_MASTERKEY:-}"',
    'ZADMIN_PASSWORD="${VPS_ZITADEL_ADMIN_PASSWORD:-}"',
    'ZEXTERNAL="${VPS_ZITADEL_EXTERNAL_URL:-}"',
    'if [ -n "$ZMASTERKEY" ] && [ -n "$ZADMIN_PASSWORD" ] && [ -n "$ZEXTERNAL" ]; then',
    '  ZIMAGE="ghcr.io/zitadel/zitadel:v4.16.0"',
    '  echo "[medicalos] pulling $ZIMAGE (identity provider)"',
    '  docker pull "$ZIMAGE"',
    '  docker rm -f medicalos-zitadel 2>/dev/null || true',
    "  docker run -d --name medicalos-zitadel --restart unless-stopped \\",
    '    --cpus "${VPS_ZITADEL_CPUS:-0.2}" --memory "${VPS_ZITADEL_MEMORY:-256m}" \\',
    "    --network platform \\",
    '    -e ZITADEL_DATABASE_POSTGRES_HOST="platform-postgres" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_PORT="5432" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_DATABASE="medicalos_zitadel" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_USER_USERNAME="medicalos_zitadel" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_USER_PASSWORD="${VPS_ZITADEL_DB_PASSWORD:-}" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_ADMIN_USERNAME="medicalos_zitadel" \\',
    '    -e ZITADEL_DATABASE_POSTGRES_ADMIN_PASSWORD="${VPS_ZITADEL_DB_PASSWORD:-}" \\',
    '    -e ZITADEL_EXTERNALPORT="443" \\',
    '    -e ZITADEL_EXTERNALSECURE="true" \\',
    '    -e ZITADEL_EXTERNALURL="$ZEXTERNAL" \\',
    '    -e ZITADEL_MASTERKEY="$ZMASTERKEY" \\',
    '    -e ZITADEL_FIRSTINSTANCE_ORG_HUMAN_PASSWORD="$ZADMIN_PASSWORD" \\',
    '    "$ZIMAGE" start-from-init --masterkey "$ZMASTERKEY"',
    "  echo \"[medicalos] zitadel container: $(docker inspect medicalos-zitadel --format '{{.Config.Image}}')\"",
    '  echo "[medicalos] NEXT (documented in docs/deployment/vps-medicalos.md): run"',
    '  echo "[medicalos] infra/zitadel/provision.sh against $ZEXTERNAL, then set"',
    '  echo "[medicalos] the VPS_ZITADEL_ISSUER/CLIENT_ID/CLIENT_SECRET secrets."',
    "else",
    '  echo "[medicalos] zitadel: not configured (VPS_ZITADEL_* inputs missing) — platform sign-in stays disabled"',
    "fi",
]

extension = nl.join(extension_lines) + nl
s = s.replace(anchor, anchor + nl + extension, 1)
with io.open(path, "w", encoding="utf-8", newline="") as f:
    f.write(s)
print("deploy.sh extended")
