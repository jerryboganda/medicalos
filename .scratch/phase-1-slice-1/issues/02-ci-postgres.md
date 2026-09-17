# 02 — CI PostgreSQL gates

Status: resolved — postgres:17 service + DATABASE_URL + psql schema bootstrap live in .github/workflows/ci.yml; query! macros check against the live schema.
Requirement IDs: OPS-07 (matrix extension), §27.1 'integration tests against an ephemeral PostgreSQL'

Add postgres:17 service + DATABASE_URL to the rust job; schema bootstrap via psql before compile (query! macros check against live schema); test step exercises the seam tests.
