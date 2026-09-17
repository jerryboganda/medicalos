# Tauri Mobile Go/No-Go Spike Report — TEMPLATE

Fill with **measured** numbers from the Phase 0 spike (issue 08). No estimated or hoped-for values. Devices: reference low-end Android (2–3 GB RAM) + recent iPhone. Builds come from GitHub Actions (issues/18 ruling applies to running them).

## 1. Environment

| Item | Android device | iOS device |
|---|---|---|
| Model / RAM / OS version | | |
| WebView / browser kernel version | | |
| Build provenance (CI run URL) | | |

## 2. Client budgets (§30.2) — measured

| Metric | Budget | Android measured | iPhone measured | Pass? |
|---|---|---|---|---|
| Cold start to interactive | ≤ 2.5 s Android ref; ≤ 1.5 s iPhone | | | |
| Next question from local data | ≤ 100 ms | | | |
| Scroll/tap | 60 fps; no visible tap delay | | | |
| Base app size (before packs) | ≤ 40 MB | | | |
| Memory through 200-question mock | stable on 2–3 GB device | | | |
| Battery, 1 h active study | ≤ 8% | | | |
| Crash-free sessions (spike period) | ≥ 99.8% target | | | |

## 3. Owned native plugins — working / not working

For each: remote push (APNs/FCM), purchases (StoreKit 2 / Play Billing), secure-window flag, capture detection + obscure, Play Integrity / App Attest, background sync scheduling, keep-awake, keychain/keystore, encrypted SQLite via Rust core. Note: owned Swift/Kotlin code with tests (§31.1), not copied snippets.

| Capability | Android | iOS | Notes |
|---|---|---|---|
| | | | |

## 4. Shared core parity

- `crates/domain-contracts` behavior identical native (Tauri) vs WebAssembly (web build): same fixture outputs? yes/no + evidence.

## 5. Verdict

- GO / NO-GO for Tauri mobile, per §20.1. NO-GO triggers the fallback: same SvelteKit UI + Rust backend, Capacitor mobile shell only.
- Decision recorded in: docs/architecture-decisions/0006-tauri-mobile-gonogo.md
- Residual risks carried into Phase 1:
