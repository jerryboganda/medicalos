# Auth hardening (learner-facing security tails)

Status: in-progress (slice 1: login throttling)
Requirement IDs: §6.3 (accounts and sign-in), CORE-07 tails; named gap in the
2026-09-28 external readiness assessment ("no login throttling").

## Problem

`POST /v1/auth/login` performs an unthrottled password verify. An attacker can
guess passwords at network speed against every account. The external readiness
assessment lists login throttling as a required beta gate.

## Slice 1 — per-account login throttling (this slice)

- **Migration `0062_login_throttling`:** `login_throttle (email TEXT PRIMARY
  KEY, consecutive_failures INT NOT NULL DEFAULT 0, locked_until TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now())`. Idempotent up/down.
- **Login flow:** read the row before verifying. Locked → `423 login_locked`
  with `details.retry_after_seconds` (honest to the legitimate user). Failed
  password increments the counter; at 10 consecutive failures the account
  locks with exponential backoff `min(2^(n-10), 15)` minutes. Successful
  login clears the row.
- **Known limitation (documented, deliberate):** per-account counting lets a
  third party extend a victim's lockout; the 15-minute cap bounds that
  annoyance. Per-IP/per-source throttling arrives with the edge/proxy work
  (CSP/HSTS slice notes the same dependency).

## Acceptance (slice 1)

- 10 wrong passwords → the 11th attempt returns `423 login_locked` even with
  the correct password, carrying `retry_after_seconds`.
- A successful login resets the count: 5 wrong + 1 correct + 5 wrong stays
  unlocked.
- Existing login behavior (200/401 shapes) unchanged; e2e unaffected.

## Later slices (queued, not started)

- Password reset + email verification: needs an email-delivery seam (adapter
  pattern like the Coach's OpenAI adapter); the provider choice is an owner
  input. Token infrastructure can be built against the seam.
- CSP (report-only first), HSTS: app-level CSP in `apps/client/nginx.conf`;
  HSTS belongs to the TLS edge (nginx-proxy-manager) — runbook change.
- Off-site backup copy + restore drill: destination is an owner input.
