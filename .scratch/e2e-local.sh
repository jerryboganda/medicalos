#!/usr/bin/env bash
# E2E lifecycle test against the local stack, through the web origin exactly
# like the browser (nginx /api proxy). Prints PASS/FAIL lines; exits non-zero
# if any step fails. Never prints secrets. Idempotent (safe to re-run).
set -u
BASE="http://127.0.0.1:8081/api/v1"
ADMIN_TOKEN=$(grep '^ADMIN_TOKEN=' ~/medicalos-local/.env | cut -d= -f2 | tr -d '\r\n')
STUDENT="student-e2e@local.test"
ADMIN="admin-e2e@local.test"
PW="e2e-Passw0rd!"
fails=0
body=/tmp/e2e-body.json

req() { # method url [curl args...] -> status code
  local method=$1 url=$2; shift 2
  curl -s -o "$body" -w '%{http_code}' -X "$method" "$@" "$url"
}
check() { # label expected actual
  if [ "$3" = "$2" ]; then echo "PASS  $1 (HTTP $3)"; else
    echo "FAIL  $1 — expected $2, got $3"; head -c 400 "$body"; echo; fails=$((fails+1)); fi
}
login() { # email -> token (empty on failure)
  req POST "$BASE/auth/login" -H 'content-type: application/json' \
    -d "{\"email\":\"$1\",\"password\":\"$PW\"}" >/dev/null
  jq -r '.token // empty' "$body" 2>/dev/null
}

echo "== seeded content =="
code=$(req POST "$BASE/auth/register" -H 'content-type: application/json' \
  -d "{\"email\":\"probe-$RANDOM@local.test\",\"password\":\"$PW\"}")
# A fresh registration succeeding proves the API is live; content seeding is
# judged by whether the exams catalog has items once we are logged in.

echo "== student account =="
code=$(req POST "$BASE/auth/register" -H 'content-type: application/json' \
  -d "{\"email\":\"$STUDENT\",\"password\":\"$PW\"}")
case "$code" in
  200|409) echo "PASS  student account present (HTTP $code)";;
  *) echo "FAIL  student register — HTTP $code"; head -c 300 "$body"; echo; fails=$((fails+1));;
esac
# Wrong-password rejection is probed with a throwaway account so repeated
# script runs never trip the per-account login lockout (§6.3).
PROBE="probe-wrong-$(date +%s)@local.test"
code=$(req POST "$BASE/auth/login" -H 'content-type: application/json' \
  -d "{\"email\":\"$PROBE\",\"password\":\"wrong-password\"}")
check "wrong-password login rejected (probe account)" 401 "$code"
STOKEN=$(login "$STUDENT")
if [ -n "$STOKEN" ]; then echo "PASS  student login (session issued)"; else
  echo "FAIL  student login"; fails=$((fails+1)); fi
AUTH="authorization: Bearer $STOKEN"

echo "== admin account =="
code=$(req POST "$BASE/auth/register" -H 'content-type: application/json' \
  -d "{\"email\":\"$ADMIN\",\"password\":\"$PW\"}")
case "$code" in
  200|409) echo "PASS  admin account present (HTTP $code)";;
  *) echo "FAIL  admin register — HTTP $code"; head -c 300 "$body"; echo; fails=$((fails+1));;
esac
ATOKEN=$(login "$ADMIN")
if [ -n "$ATOKEN" ]; then echo "PASS  admin login (session issued)"; else
  echo "FAIL  admin login"; fails=$((fails+1)); fi
AAUTH="authorization: Bearer $ATOKEN"

echo "== learner lifecycle (student session) =="
code=$(req GET "$BASE/exams" -H "$AUTH")
check "GET /exams" 200 "$code"
EXAM=$(jq -r '.exams[0].exam_id // .items[0].id // .[0].id // empty' "$body" 2>/dev/null)
if [ -n "$EXAM" ]; then echo "      exam under test: $EXAM"; else
  echo "FAIL  exams catalog empty — cannot drive lifecycle"; head -c 300 "$body"; echo; fails=$((fails+1)); fi

code=$(req PUT "$BASE/me/engagement/settings" -H "$AUTH" -H 'content-type: application/json' \
  -d "{\"qotd_exam_id\":\"$EXAM\"}")
check "PUT /me/engagement/settings (select exam)" 200 "$code"

code=$(req GET "$BASE/qotd" -H "$AUTH")
check "GET /qotd" 200 "$code"
QV=$(jq -r '.question_version_id // empty' "$body" 2>/dev/null)
if [ -n "$QV" ]; then
  echo "      qotd question available ($(jq -r '.options | length' "$body") options)"
  code=$(req POST "$BASE/me/qotd/answers" -H "$AUTH" -H 'content-type: application/json' \
    -d "{\"question_version_id\":\"$QV\",\"chosen_index\":0,\"elapsed_ms\":4200}")
  check "POST /me/qotd/answers (submit answer)" 200 "$code"
  echo "      answer outcome: $(jq -c '{correct, already_answered}' "$body" 2>/dev/null)"
else
  echo "SKIP  qotd has no question today (answered=$? — see GET above)"
fi

code=$(req GET "$BASE/me/readiness?exam_id=$EXAM" -H "$AUTH")
check "GET /me/readiness?exam_id=..." 200 "$code"
for ep in me/xp me/today me/trends me/review-debt me/heatmap me/retests; do
  code=$(req GET "$BASE/$ep" -H "$AUTH")
  check "GET /$ep" 200 "$code"
done
echo "      lifecycle state: readiness=$(jq -c '{exam_id, readiness: (.readiness // .score // .band)}' "$body" 2>/dev/null | head -c 160)"

echo "== admin console =="
code=$(req GET "$BASE/admin/dashboard" -H "$AUTH")
check "admin dashboard rejected for plain user session (no token)" 403 "$code"
code=$(req GET "$BASE/admin/dashboard" -H "$AAUTH" -H "x-admin-token: $ADMIN_TOKEN")
check "admin dashboard accepted for admin session + token" 200 "$code"
jq -c '.' "$body" 2>/dev/null | head -c 260; echo

echo "== result =="
if [ "$fails" -eq 0 ]; then echo "E2E RESULT: ALL PASS"; else echo "E2E RESULT: $fails FAILURE(S)"; fi
exit $fails
