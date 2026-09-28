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
DEST="$REMOTE/medicalos-db"
KEEP_DAYS="${KEEP_DAYS:-30}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="/var/backups/medicalos/medicalos-$STAMP.sql.gz"

rclone listremotes 2>/dev/null | grep -qx "$REMOTE/" || {
  echo "rclone remote '$REMOTE' is not configured — refusing to fake an off-site backup" >&2
  exit 1
}
rclone lsd "$DEST" >/dev/null 2>&1 || rclone mkdir "$DEST"

mkdir -p "$(dirname "$OUT")"
echo "[medicalos-backup] dumping medicalos database"
docker exec platform-postgres pg_dump -U medicalos medicalos | gzip > "$OUT"
[ -s "$OUT" ] || { echo "pg_dump produced an empty file — refusing to upload" >&2; exit 1; }

echo "[medicalos-backup] copying $OUT to $DEST"
rclone copy "$OUT" "$DEST"

# Verify the copy landed with the same size before pruning anything.
LOCAL_SIZE=$(stat -c%s "$OUT")
REMOTE_SIZE=$(rclone ls "$DEST" | grep "$(basename "$OUT")" | awk '{print $1}')
[ "$LOCAL_SIZE" = "$REMOTE_SIZE" ] || {
  echo "remote copy size mismatch (local=$LOCAL_SIZE remote=${REMOTE_SIZE:-missing})" >&2
  exit 1
}
echo "[medicalos-backup] verified: $DEST/$(basename "$OUT") ($LOCAL_SIZE bytes)"

echo "[medicalos-backup] pruning remote copies older than $KEEP_DAYS days"
rclone delete --min-age "${KEEP_DAYS}d" "$DEST" || true

# Keep the two most recent local dumps for quick restores; drop older.
ls -1t "$(dirname "$OUT")"/medicalos-*.sql.gz 2>/dev/null | tail -n +3 | xargs -r rm --
echo "[medicalos-backup] done at $STAMP"
