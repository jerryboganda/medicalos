# OPS-03 — Reversible migration verification in CI

Status: in-progress (rollback/reapply CI gate authored; CI acceptance pending)
Requirement IDs: OPS-03
Source: `.scratch/ops03-reversible-migrations/spec.md`; `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§20.2, 28.1, 30.1; `docs/requirements/traceability.md`

## Problem

`ci.yml` previously only applied `.up.sql` files against the ephemeral CI PostgreSQL database. Although 57 pairs of `.up.sql` and `.down.sql` migrations exist in `apps/api/migrations`, the `.down.sql` rollback files were not executed or tested in CI, allowing potential rollback syntax or dependency errors to slip through.

## Acceptance

- Bash script `scripts/verify-migrations.sh` checks:
  1. It runs only in GitHub Actions; `DATABASE_URL` is configured and `apps/api/migrations` exists.
  2. Every `.up.sql` has a matching `.down.sql` and vice versa (parity check).
  3. Applies all `.up.sql` in ascending lexical/numeric order.
  4. Rolls back all `.down.sql` in descending reverse order.
  5. Reapplies all `.up.sql` in ascending order so downstream tests and query preparation have the complete schema.
  6. Fails fast on any error (`set -euo pipefail` and `psql -v ON_ERROR_STOP=1`).
- `.github/workflows/ci.yml` replaces the raw `for f in *.up.sql` loop with `bash scripts/verify-migrations.sh`.
- The schema execution guarantee proves DDL reversibility on a clean database; it does not claim to validate data-preserving rollbacks on production tables.
- Signed releases remain a separate pending external gate for OPS-03.
