#!/usr/bin/env bash
# OPS-03: Reversible migration verification gate
# Verifies that every database migration applies, rolls back in reverse order,
# and reapplies cleanly against the target PostgreSQL database.
set -euo pipefail

if [[ "${GITHUB_ACTIONS:-}" != "true" ]]; then
  echo "ERROR: This migration gate is restricted to GitHub Actions." >&2
  exit 1
fi

if [ -z "${DATABASE_URL:-}" ]; then
  echo "ERROR: DATABASE_URL environment variable is not set." >&2
  exit 1
fi

MIGRATIONS_DIR="apps/api/migrations"
if [ ! -d "$MIGRATIONS_DIR" ]; then
  echo "ERROR: Migrations directory '$MIGRATIONS_DIR' does not exist." >&2
  exit 1
fi

echo "==> [OPS-03] Checking migration file pairs in $MIGRATIONS_DIR..."
shopt -s nullglob
UP_FILES=("$MIGRATIONS_DIR"/*.up.sql)
DOWN_FILES=("$MIGRATIONS_DIR"/*.down.sql)
shopt -u nullglob

if [ ${#UP_FILES[@]} -eq 0 ]; then
  echo "ERROR: No .up.sql migration files found in $MIGRATIONS_DIR." >&2
  exit 1
fi

# 1. Parity validation: each .up.sql must have a matching .down.sql
for up in "${UP_FILES[@]}"; do
  down="${up%.up.sql}.down.sql"
  if [ ! -f "$down" ]; then
    echo "ERROR: Missing matching rollback file: expected '$down' for '$up'." >&2
    exit 1
  fi
done

# 2. Parity validation: each .down.sql must have a matching .up.sql
for down in "${DOWN_FILES[@]}"; do
  up="${down%.down.sql}.up.sql"
  if [ ! -f "$up" ]; then
    echo "ERROR: Missing matching forward migration: expected '$up' for '$down'." >&2
    exit 1
  fi
done

echo "==> [OPS-03] Verified parity: ${#UP_FILES[@]} matching (.up.sql / .down.sql) migration pairs."

# 3. Apply all forward migrations in sorted order
echo "==> [OPS-03] Applying ${#UP_FILES[@]} migrations in forward order..."
for up in "${UP_FILES[@]}"; do
  echo "  [apply] $up"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$up"
done

# 4. Roll back all migrations in reverse order
echo "==> [OPS-03] Rolling back ${#DOWN_FILES[@]} migrations in reverse order..."
for (( i=${#DOWN_FILES[@]}-1; i>=0; i-- )); do
  down="${DOWN_FILES[$i]}"
  echo "  [rollback] $down"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$down"
done

# 5. Reapply all forward migrations in sorted order
echo "==> [OPS-03] Reapplying ${#UP_FILES[@]} migrations in forward order..."
for up in "${UP_FILES[@]}"; do
  echo "  [reapply] $up"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$up"
done

echo "==> [OPS-03] Migration reversibility gate passed: apply -> reverse rollback -> reapply completed successfully."
