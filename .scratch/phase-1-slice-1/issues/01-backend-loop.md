# 01 — Connected question loop backend

Status: resolved — all gates green in https://github.com/jerryboganda/medicalos/actions/runs/35280682748 (fmt, clippy -D warnings, cargo-deny, 4/4 seam tests, wasm32 build). Bugs found and fixed by the seam tests: plans.version NOT NULL violation on cold start (500s), revision tasks not stamped with added_by_revision (undo left them behind).
Requirement IDs: CORE-01, CORE-10, QB-01, QB-03, QB-04, QB-05, QB-11, QB-14, EX-01, EX-04, AI-01, AI-02, AI-07, AI-17, PLAN-01, PLAN-02 (all partial as scoped in spec.md)

Implement apps/api: schema + routes + deterministic revision engine per spec. Gate evidence: seam tests green in CI against ephemeral PostgreSQL.
