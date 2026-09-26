# EX-08 — Capture browser-observable integrity signals

Status: in-progress
Requirement IDs: EX-08
Triage label: ready-for-agent
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §11.3; `.scratch/ex08-browser-integrity-capture/spec.md`

## Acceptance

- Open timed browser sessions record window blur, backgrounding, fullscreen exit, and significant device-clock changes through the existing authenticated API; configured mock sessions use the same lifecycle signals for their server-enforced policy.
- Signal collection is best-effort and never blocks answers or the countdown; only the server-configured mock policy can produce a warning or auto-submit action.
- Browser E2E verifies the real API accepts all four event types for the owning session.
- Signal listeners are cleaned up when the session page unmounts.
- Browser E2E verifies real warning and worker auto-submit receipt behavior without replacing the API response.

## Comments

- 2026-09-26: The session page already captures the four browser-observable signals and existing Playwright coverage verifies real authenticated API responses; this ticket was stale after commit `7c96af7`.
- 2026-09-26: Follow-up E2E coverage is being added for listener cleanup and real mock warning/auto-submit policy responses; CI evidence pending.
- 2026-09-26: Browser-only capture cannot reliably establish screenshot, recording, split-screen, or PiP activity; these are not inferred from unrelated lifecycle events. Native attestation and platform-specific signals remain separate work.
