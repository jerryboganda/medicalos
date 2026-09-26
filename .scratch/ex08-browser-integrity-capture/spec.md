# EX-08 Browser-Observable Integrity Signal Capture

Status: in-progress (E2E verification update underway)
Requirement: EX-08

## Outcome

Open timed and configured mock sessions best-effort record integrity signals that the browser can observe through standard page lifecycle events. Mock sessions use the same signals to apply their explicitly configured server-side warning or auto-submit policy. The existing authenticated `/v1/integrity-events` endpoint remains the source of persistence and server timestamps.

## Design choice

Use the existing session page and API helper. A shared cross-app monitoring layer would add a second abstraction for one consumer. Inferring signals such as screenshots or split-screen from ordinary browser events would report evidence the browser cannot reliably observe. The session page will therefore report backgrounding, foreground/return, window blur, fullscreen exit, and significant device wall-clock changes.

Signals are sent only for open timed or mock sessions. Requests are best-effort and must never block answering, submission, or the monotonic countdown. They include the session ID and client timestamp; the server remains authoritative for ownership and recorded time. Timed-session signals remain observational; only a mock's explicit policy may warn or auto-submit.

## Acceptance

- A timed session sends `window_blur` when the page window loses focus.
- It sends `background` when the document becomes hidden.
- It sends `fullscreen_exit` only after fullscreen had been entered and is then exited.
- It sends `clock_change` when `Date.now()` drifts by at least ten seconds from a visible-session monotonic baseline. A sampling gap over one minute resets the baseline to avoid treating long suspension as a clock edit.
- Signal failures do not interrupt session behavior, and listeners are removed on unmount.
- Browser E2E verifies real authenticated API responses for all four captured signal types.
- Browser E2E verifies that a configured mock warning is server-produced, leaves the session open, and that the mock auto-submit worker returns its persisted receipt through a real foreground signal.
- Browser E2E proves a session's lifecycle listeners stop emitting after client-side navigation unmounts the session page.

## Deliberate limits

Ordinary web pages cannot reliably detect screenshots, screen recording, split-screen, or picture-in-picture across browsers and operating systems. Native attestation and those platform-specific signals remain separate work. Timed non-mock sessions record signals observationally. Only a mock's explicitly configured server policy may warn or auto-submit from server-measured away time; its late-sync grace is a separate server-side answer rule. Neither client timestamps nor browser signals are clinical or disciplinary evidence.

## Verification

Run the existing GitHub Actions workflow for the resulting commit. The E2E API process uses a test-only admin token to create isolated policy fixtures. No local builds or test suites are part of this project’s compute policy.
