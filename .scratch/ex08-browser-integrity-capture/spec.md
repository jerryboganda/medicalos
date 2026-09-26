# EX-08 Browser-Observable Integrity Signal Capture

Status: ready-for-agent
Requirement: EX-08

## Outcome

Timed browser sessions best-effort record integrity signals that the browser can observe through standard page lifecycle events. The existing authenticated `/v1/integrity-events` endpoint remains the source of persistence and server timestamps.

## Design choice

Use the existing session page and API helper. A shared cross-app monitoring layer would add a second abstraction for one consumer. Inferring signals such as screenshots or split-screen from ordinary browser events would report evidence the browser cannot reliably observe. The session page will therefore report only backgrounding, window blur, fullscreen exit, and significant device wall-clock changes.

Signals are sent only for open timed sessions. Requests are best-effort and must never block answering, submission, or the monotonic countdown. They include the session ID and client timestamp; the server remains authoritative for ownership and recorded time. Existing integrity records are observational and never auto-punish.

## Acceptance

- A timed session sends `window_blur` when the page window loses focus.
- It sends `background` when the document becomes hidden.
- It sends `fullscreen_exit` only after fullscreen had been entered and is then exited.
- It sends `clock_change` when `Date.now()` drifts by at least ten seconds from a visible-session monotonic baseline. A sampling gap over one minute resets the baseline to avoid treating long suspension as a clock edit.
- Signal failures do not interrupt session behavior, and listeners are removed on unmount.
- Browser E2E verifies real authenticated API responses for all four captured signal types.

## Deliberate limits

Ordinary web pages cannot reliably detect screenshots, screen recording, split-screen, or picture-in-picture across browsers and operating systems. Native attestation and those platform-specific signals remain separate work. Grace windows and per-test enforcement remain unimplemented; the master plan labels proposed grace durations as proposals, and the current server contract explicitly keeps integrity records non-punitive.

## Verification

Run the existing GitHub Actions workflow for the resulting commit. No local builds or test suites are part of this project’s compute policy.
