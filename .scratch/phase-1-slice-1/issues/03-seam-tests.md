# 03 — Seam tests

Status: ready-for-agent
Requirement IDs: §30.1 core release gates (subset), TRUST-01

tests/integration.rs: register/login, full loop with revision + undo + receipt, idempotent answer replay (no duplicate attempt), revision pool = wrong+skipped, honest low-evidence learner state, migration up→down→up.
