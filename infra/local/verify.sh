#!/usr/bin/env bash
# Local staging verifier — host-side mirror of deploy.yml production-verify.
# Usage: bash infra/local/verify.sh   (stack must be up)
set -u
API=${API:-http://127.0.0.1:18080}
WEB=${WEB:-http://127.0.0.1:8081}
SITE=${SITE:-http://127.0.0.1:8082}
fail=0

check() { # label url pattern
  if curl -sf "$2" | grep -q "$3"; then
    echo "PASS  $1"
  else
    echo "FAIL  $1  ($2)"
    fail=1
  fi
}

check "api  /healthz (direct)"        "$API/healthz" '^ok'
check "api  /readyz (database)"      "$API/readyz" '"status":"ok"'
check "web  app shell"                "$WEB/" 'data-sveltekit-preload-data'
check "web  /api/version.json static" "$WEB/api/version.json" '"sha"'
check "web  /api/healthz via proxy"   "$WEB/api/healthz" 'ok'
check "web  /api/readyz via proxy"    "$WEB/api/readyz" '"status":"ok"'

echo
echo "deployed build: $(curl -sf "$WEB/api/version.json" || echo '(version.json unreachable)')"

# Site is behind the `full` profile — only assert when it answers.
if curl -sf --max-time 2 "$SITE/" | grep -q '<'; then
  echo "PASS  site /"
else
  echo "SKIP  site (not deployed — enable with: docker compose --profile full up -d)"
fi

exit $fail
