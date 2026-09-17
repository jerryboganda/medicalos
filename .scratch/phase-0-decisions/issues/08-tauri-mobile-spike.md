# 08 — Tauri mobile go/no-go spike (§20.1, §28.1)

Status: in-progress (plan + template done; device runs pending issues/15 + issues/18)
Requirement IDs: ARCH-01, ARCH-03, UX-03; gate evidence for Phase 0

## Task

Spike on a reference low-end Android (2–3 GB RAM) and a recent iPhone:

1. Owned native plugins (§20.1 risk): remote push (APNs/FCM), purchases (StoreKit 2 / Play Billing), secure-window flag, capture detection, Play Integrity / App Attest, background sync scheduling, keep-awake, keychain.
2. Encrypted SQLite via the Rust core; cold start; scroll + tap latency in the system WebView.
3. Shared Rust core running natively in the Tauri shell and as WebAssembly (same crate).

Measured against §30.2 client budgets (cold start ≤ 2.5 s Android reference; next question ≤ 100 ms; 60 fps scroll/tap; ≤ 40 MB base; stable memory through a 200-question mock; ≤ 8%/h battery).

Outcome: GO keeps Tauri mobile; NO-GO triggers the §20.1 fallback — same SvelteKit UI + Rust backend, Capacitor mobile shell only.

Report: `spike-report-template.md` in this folder. Device access and the CI-artifacts-on-devices ruling are issues/15 and issues/18.

## Acceptance evidence

Completed spike report with measured numbers vs budgets, and the go/no-go call recorded in an ADR addendum.
