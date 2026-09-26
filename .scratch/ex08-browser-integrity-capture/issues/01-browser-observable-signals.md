# EX-08 — Capture browser-observable integrity signals

Status: ready-for-agent
Requirement IDs: EX-08
Triage label: ready-for-agent
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §11.3; `.scratch/ex08-browser-integrity-capture/spec.md`

## Acceptance

- Open timed browser sessions record window blur, backgrounding, fullscreen exit, and significant device-clock changes through the existing authenticated API.
- Signal requests are best-effort and do not change answer, timer, or submission behavior.
- Browser E2E verifies the real API accepts all four event types for the owning session.
- Signal listeners are cleaned up when the session page unmounts.

## Comments

- 2026-09-26: The API accepted and stored signals, but the browser session had no call to the endpoint.
- 2026-09-26: Browser-only capture cannot reliably establish screenshot, recording, split-screen, or PiP activity; these are not inferred from unrelated lifecycle events.
- 2026-09-26: Grace windows and policy-based warning/auto-submit remain separate EX-08 work because the master plan describes proposed durations and the current integrity route states that records never auto-punish.
