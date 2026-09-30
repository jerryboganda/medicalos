#!/usr/bin/env bash
# Off-site database backup (readiness assessment gap: "no off-site copy").
# Dumps the medicalos database out of platform-postgres and copies it to an
# rclone remote. Runs ON the VPS as a nightly timer — a pg_dump of this size
# is minutes of I/O, inside the ops envelope (the AGENTS.md compute policy
# bars heavy builds/processing there, not routine backups).
#
# One-time owner setup, then the script refuses to run without it:
#   rclone config create medicalos-offsite <backend> ...
# (any backend works: S3/B2/rsync-over-ssh — the destination choice is the
# owner input recorded in .scratch/owner-inputs-requested.md).
set -euo pipefail

REMOTE="${REMOTE:-medicalos-offsite}"
REMOTE="${REMOTE%:}"
DEST="$REMOTE:${REMOTE_PATH:-medicalos-db}"
KEEP_DAYS="${KEEP_DAYS:-30}"
BACKUP_DIR="${BACKUP_DIR:-/var/backups/medicalos}"
DATABASE_CONTAINER="${DATABASE_CONTAINER:-platform-postgres}"
DATABASE_USER="${DATABASE_USER:-medicalos}"
DATABASE_NAME="${DATABASE_NAME:-medicalos}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$BACKUP_DIR/medicalos-$STAMP.sql.gz"

[[ "$REMOTE" =~ ^[[:alnum:]_.-]+$ ]] || { echo "invalid rclone remote name" >&2; exit 1; }
[[ "$KEEP_DAYS" =~ ^[1-9][0-9]*$ ]] || { echo "KEEP_DAYS must be a positive integer" >&2; exit 1; }
rclone listremotes 2>/dev/null | grep -Fxq "$REMOTE:" || {
  echo "rclone remote '$REMOTE' is not configured — refusing to fake an off-site backup" >&2
  exit 1
}
rclone lsd "$DEST" >/dev/null 2>&1 || rclone mkdir "$DEST"

mkdir -p "$(dirname "$OUT")"
exec 9>"$BACKUP_DIR/.backup.lock"
flock -n 9 || { echo "another backup is running" >&2; exit 1; }
echo "[medicalos-backup] dumping medicalos database"
docker exec "$DATABASE_CONTAINER" pg_dump -U "$DATABASE_USER" "$DATABASE_NAME" | gzip > "$OUT"
[ -s "$OUT" ] || { echo "pg_dump produced an empty file — refusing to upload" >&2; exit 1; }
gzip -t "$OUT"

echo "[medicalos-backup] copying $OUT to $DEST"
rclone copy "$OUT" "$DEST"

# Download verification works even on backends without checksum support.
LOCAL_HASH=$(sha256sum "$OUT" | cut -d' ' -f1)
REMOTE_HASH=$(rclone cat "$DEST/$(basename "$OUT")" | sha256sum | cut -d' ' -f1)
[ "$LOCAL_HASH" = "$REMOTE_HASH" ] || {
  echo "remote copy checksum mismatch — refusing to prune" >&2
  exit 1
}
echo "[medicalos-backup] verified: $DEST/$(basename "$OUT")"

echo "[medicalos-backup] pruning remote copies older than $KEEP_DAYS days"
rclone delete --min-age "${KEEP_DAYS}d" "$DEST" || true

# Keep the two most recent local dumps for quick restores; drop older.
ls -1t "$(dirname "$OUT")"/medicalos-*.sql.gz 2>/dev/null | tail -n +3 | xargs -r rm --
echo "[medicalos-backup] done at $STAMP"
