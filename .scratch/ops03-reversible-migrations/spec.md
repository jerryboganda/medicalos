# OPS-03 — Reversible database migrations CI verification gate

Status: in-progress (rollback/reapply CI gate authored; CI acceptance pending)
Requirement IDs: OPS-03
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§20.2, 28.1, 30.1; `docs/requirements/traceability.md`

## Problem Statement

The repository maintains paired `.up.sql` and `.down.sql` migration scripts for all database schema changes (57 pairs), but the continuous integration workflow (`.github/workflows/ci.yml`) only executed `.up.sql` files during the test setup. Consequently, syntax defects, foreign key constraint dropping order bugs, missing cascade rules, or incomplete teardown in `.down.sql` files could pass undetected in pull requests and master commits.

## Solution

Implement an automated schema verification gate executed against the ephemeral PostgreSQL service container in the GitHub Actions `rust` CI job:
1. Verify migration parity: require that every `*.up.sql` has an exact corresponding `*.down.sql` pair, and vice versa.
2. Forward migration: apply all `.up.sql` files in ascending numeric order against the clean test database.
3. Reverse rollback: execute all `.down.sql` files in reverse order (highest to lowest), verifying complete schema teardown without foreign key dependency violations.
4. Clean reapply: re-execute all `.up.sql` files in ascending order to return the database to the target schema state.
5. Proceed to SQLx compile-time query verification, tests, and contract exports with the clean, reapplied schema.

## Schema Execution Guarantee & Deliberate Limits

- **Guarantee:** Proves that all DDL scripts execute cleanly without syntax errors, missing type/table references, or constraint ordering deadlocks when applied forward, rolled back in reverse order, and reapplied on an ephemeral PostgreSQL 17 database. The script refuses to run outside GitHub Actions to reduce the chance of targeting a persistent database accidentally.
- **Deliberate Limit:** This gate tests schema execution mechanics on clean/ephemeral schemas; it does not claim to guarantee data-preserving rollbacks for live production data (destructive drops such as `DROP TABLE` or `DROP COLUMN` in `.down.sql` by definition discard table state). Production zero-downtime migrations with expand/contract data migrations remain an operational runtime practice.
- **Pending External Gate:** OPS-03 also spans cryptographically signed release packages. Release signing (Cosign/GPG/minisign) in the release workflow is an infrastructure gate requiring owner key provisioning and is tracked separately from this migration reversibility slice.

## Acceptance Criteria

- `scripts/verify-migrations.sh` exists as a Bash script and runs with `set -euo pipefail`.
- Fails fast outside GitHub Actions, if `DATABASE_URL` is unset, or if migration directory `apps/api/migrations` is missing.
- Fails fast if any `.up.sql` is missing a `.down.sql`, or if any `.down.sql` is missing an `.up.sql`.
- Applies migrations in forward order, rolls back in exact reverse order, and reapplies in forward order using `psql "$DATABASE_URL" -v ON_ERROR_STOP=1`.
- `.github/workflows/ci.yml` invokes `scripts/verify-migrations.sh` during the `rust` job before Clippy, SQLx query checks, and workspace tests.
- SQLx metadata generation and tests continue to run after the final forward reapply.
