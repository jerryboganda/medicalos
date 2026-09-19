# QB-13 Phase 1 - Session tools baseline

## Problem Statement

The connected practice workspace already supports tutor/timed sessions, server-issued deadlines, answer feedback, marks, and submission results, but QB-13 session tools are incomplete. Learners still need an in-session calculator, converter, four-step text sizing, tutor-only hints that count as assisted evidence, clear deadline warnings, and a complete submission summary without creating a second quiz engine or duplicating medical formulas.

## Solution

Extend the existing session workspace and practice API. Reuse the shared Rust `calc-engine` through one authenticated tools API, keep unit conversion and text-size preferences client-local, expose hints only through a tutor-session action, and persist hint use on the session item so the following attempt is authoritatively stored as assisted. Deadline warnings derive from the existing server deadline and submission summary extends the existing result payload.

## User Stories

1. As a learner, I can open a responsive tool tray without leaving my question.
2. As a learner, I can run the existing BMI, BSA, MAP, GCS, Cockcroft-Gault, CKD-EPI 2021, anion-gap, and corrected-calcium calculators from that tray.
3. As a learner, calculator results are explicitly labelled for exam practice and not clinical use.
4. As a learner, invalid calculator inputs return a clear validation message instead of a fabricated result.
5. As a learner, I can convert common length, mass, temperature, and glucose units without a network dependency.
6. As a learner, I can choose exactly four text sizes and the preference survives navigation in the browser.
7. As a tutor-session learner, I can request an optional hint for the current question before answering.
8. As a timed-session learner, I cannot request a hint.
9. As a learner, a hint guides reasoning without exposing the correct option or rationales.
10. As a learner, using a hint causes the eventual attempt for that item to be stored as assisted evidence.
11. As a timed-session learner, I see an explicit warning as the existing countdown approaches auto-submit.
12. As a learner, the warning never creates or extends a second timer; the server-issued deadline remains authoritative.
13. As a learner, after submission I see total, correct, incorrect, skipped, assisted, score, and time taken from server-derived data.

## Implementation Decisions

- Keep the existing practice-session HTTP and browser workspace as the public seams.
- Add an optional hint to immutable question versions and synthetic fixtures.
- Add a durable `hint_used` flag to session items; requesting a tutor hint sets it and the answer route copies that state into `attempts.assisted`.
- Add one authenticated calculator endpoint backed directly by the shared Rust calculator crate. No formula is reimplemented in TypeScript.
- Keep unit conversion in the client because it is deterministic presentation utility state, not learning evidence. Initial coverage is cm/in, kg/lb, C/F, and glucose mg/dL/mmol/L.
- Keep text-size preference in versioned browser local storage with exactly four values.
- Derive warning text from the existing skew-corrected `remainingMs`; do not add another clock.
- Reuse the existing Medical OS tokens, controls, typography, and app shell. The tool tray is a bottom sheet on narrow screens and a side panel on wider screens.
- No new third-party dependency.

## Testing Decisions

- API integration contracts cover tutor hint access, timed hint rejection, authoritative assisted persistence, calculator success, and calculator validation.
- Browser E2E covers tool-tray visibility, four text sizes, converter/calculator interaction, tutor hint behavior, and the existing short timed-session auto-submit warning.
- Tests stay at the public HTTP/browser seams and do not assert private implementation details.
- Heavy Rust integration/build/Playwright acceptance runs in GitHub Actions. Local verification is limited to formatting/static/diff checks.

## Out of Scope

- Clinical decision support, dosing advice, diagnostic recommendations, or additional calculator formulas.
- Server-synced UI text-size preferences.
- Notes/highlighting/theme expansion beyond the QB-13 baseline.
- Offline pack synchronization beyond existing session behavior.
- Deployment, merge, tag, or release.

## Further Notes

The server deadline and existing auto-submit path remain the single timing authority. Hints never reveal answer keys before the existing feedback boundary.
