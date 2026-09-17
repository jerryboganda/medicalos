# 03 — Seam tests

Status: resolved — tests/integration.rs, 4/4 green (run 35280682748). The tests caught two real defects (see issue 01); that is the loop working.
Requirement IDs: §30.1 core release gates (subset), TRUST-01

tests/integration.rs: register/login, full loop with revision + undo + receipt, idempotent answer replay (no duplicate attempt), revision pool = wrong+skipped, honest low-evidence learner state, migration up→down→up.
