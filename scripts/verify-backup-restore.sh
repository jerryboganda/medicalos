#!/usr/bin/env bash
# Real pg_dump/rclone/psql drill against the disposable CI database only.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true ]] || { echo 'CI-only restore drill'; exit 1; }
: "${DATABASE_CONTAINER:?CI postgres service container required}"
ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=$(mktemp -d)
TARGET="medos_restore_${GITHUB_RUN_ID}_${GITHUB_RUN_ATTEMPT}"
cleanup() {
  docker exec "$DATABASE_CONTAINER" dropdb -U postgres --if-exists "$TARGET" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT
docker exec -i "$DATABASE_CONTAINER" psql -U postgres -d medos_ci -v ON_ERROR_STOP=1 <<'SQL'
CREATE SCHEMA medicalos_restore_fixture;
CREATE TABLE medicalos_restore_fixture.learners (id integer PRIMARY KEY, email text NOT NULL);
CREATE TABLE medicalos_restore_fixture.answers (receipt text PRIMARY KEY, learner_id integer REFERENCES medicalos_restore_fixture.learners, chosen_index integer NOT NULL);
CREATE TABLE medicalos_restore_fixture.notes (id integer PRIMARY KEY, learner_id integer REFERENCES medicalos_restore_fixture.learners, body text NOT NULL);
INSERT INTO medicalos_restore_fixture.learners VALUES (1, 'restore-a@example.test'), (2, 'restore-b@example.test');
INSERT INTO medicalos_restore_fixture.answers VALUES ('receipt-a', 1, 0), ('receipt-b', 1, 2), ('receipt-c', 2, 1);
INSERT INTO medicalos_restore_fixture.notes VALUES (1, 1, 'Synthetic reviewed note A'), (2, 2, 'Synthetic reviewed note B');
SQL
printf '[medicalos-offsite]\ntype = local\n' > "$WORK/rclone.conf"
export RCLONE_CONFIG="$WORK/rclone.conf"
DATABASE_USER=postgres DATABASE_NAME=medos_ci BACKUP_DIR="$WORK/backups" \
  REMOTE_PATH="$WORK/remote" bash "$ROOT/infra/vps/backup-offsite.sh"
docker exec "$DATABASE_CONTAINER" createdb -U postgres "$TARGET"
gzip -dc "$WORK"/remote/medicalos-*.sql.gz | \
  docker exec -i "$DATABASE_CONTAINER" psql -U postgres -d "$TARGET" -v ON_ERROR_STOP=1 > "$WORK/restore.log"
QUERY="SELECT (SELECT count(*) FROM medicalos_restore_fixture.learners)::text || ':' || (SELECT count(*) FROM medicalos_restore_fixture.answers)::text || ':' || (SELECT count(*) FROM medicalos_restore_fixture.notes)::text || ':' || (SELECT sum(chosen_index) FROM medicalos_restore_fixture.answers)::text;"
RESTORED=$(docker exec "$DATABASE_CONTAINER" psql -U postgres -d "$TARGET" -At -c "$QUERY")
[[ "$RESTORED" == '2:3:2:3' ]] || { echo 'restored student data mismatch'; exit 1; }
for table in users attempts notes coach_turns; do
  SOURCE_COUNT=$(docker exec "$DATABASE_CONTAINER" psql -U postgres -d medos_ci -At -c "SELECT count(*) FROM $table")
  TARGET_COUNT=$(docker exec "$DATABASE_CONTAINER" psql -U postgres -d "$TARGET" -At -c "SELECT count(*) FROM $table")
  [[ "$SOURCE_COUNT" == "$TARGET_COUNT" ]] || { echo "restored $table count mismatch"; exit 1; }
done
echo 'PASS full schema backup/restore, synthetic learner relationships, answer receipts and notes'
