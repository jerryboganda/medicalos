#!/usr/bin/env bash
# Run in CI with controlled command adapters; never contacts a runtime stack.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/bin"
cat > "$WORK/bin/docker" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
echo docker >> "$MOCK_LOG"
[[ "$MOCK_CASE" != dump-failed ]] || exit 1
printf '%s\n' 'CREATE TABLE synthetic_student_answers (receipt_id text);'
MOCK
cat > "$WORK/bin/rclone" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'rclone %s\n' "$*" >> "$MOCK_LOG"
case "$1" in
  listremotes)
    [[ "$MOCK_CASE" != unconfigured ]] || exit 0
    echo 'medicalos-offsite:' ;;
  lsd|mkdir) exit 0 ;;
  copy)
    [[ "$3" == medicalos-offsite:medicalos-db ]]
    [[ "$MOCK_CASE" != upload-failed ]] || exit 1
    cp "$2" "$MOCK_COPY" ;;
  cat)
    [[ "$2" == medicalos-offsite:medicalos-db/medicalos-*.sql.gz ]]
    if [[ "$MOCK_CASE" == corrupt ]]; then printf 'corrupt'; else cat "$MOCK_COPY"; fi ;;
  delete) exit 0 ;;
  *) echo "unexpected command" >&2; exit 1 ;;
esac
MOCK
chmod +x "$WORK/bin/docker" "$WORK/bin/rclone"
for scenario in success unconfigured upload-failed corrupt dump-failed invalid-retention; do
  mkdir -p "$WORK/$scenario"
  export MOCK_CASE="$scenario" MOCK_LOG="$WORK/$scenario/commands" MOCK_COPY="$WORK/$scenario/copy"
  : > "$MOCK_LOG"
  days=30
  [[ "$scenario" != invalid-retention ]] || days=0
  if PATH="$WORK/bin:$PATH" BACKUP_DIR="$WORK/$scenario/backups" KEEP_DAYS="$days" \
    bash "$ROOT/infra/vps/backup-offsite.sh" > "$WORK/$scenario/output" 2>&1; then
    [[ "$scenario" == success ]] || { cat "$WORK/$scenario/output"; exit 1; }
    grep -q '^rclone delete --min-age 30d medicalos-offsite:medicalos-db$' "$MOCK_LOG"
  else
    [[ "$scenario" != success ]] || { cat "$WORK/$scenario/output"; exit 1; }
    ! grep -q '^rclone delete ' "$MOCK_LOG"
    if [[ "$scenario" == dump-failed ]]; then
      [[ -z "$(find "$WORK/$scenario/backups" -type f -name 'medicalos-*' -print -quit)" ]]
    fi
  fi
  echo "PASS backup $scenario"
done
