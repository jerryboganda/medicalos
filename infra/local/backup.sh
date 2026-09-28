#!/usr/bin/env bash
# Local DB backup: pg_dump -> .backups/, keep the newest 7.
# Run from anywhere: bash infra/local/backup.sh   (stack must be up)
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p ../.backups
out="../.backups/medicalos-$(date +%Y%m%d-%H%M).sql.gz"
docker compose exec -T postgres pg_dump -U medicalos medicalos | gzip > "$out"
# Retention: newest 7 (filenames sort by timestamp — no spaces, ls is safe).
ls -1t ../.backups/medicalos-*.sql.gz | tail -n +8 | xargs -r rm --
echo "wrote $out ($(du -h "$out" | cut -f1))"
