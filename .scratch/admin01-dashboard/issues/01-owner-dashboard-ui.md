# ADMIN-01 — Owner dashboard UI

Status: ready-for-agent
Requirement IDs: ADMIN-01, ARCH-02
Implementation state: source implementation and static review complete; generated ARCH-02 binding and GitHub Actions acceptance pending.
Source: `.scratch/admin01-dashboard/spec.md`; `docs/requirements/traceability.md`

## Acceptance

- `/admin/dashboard` uses the existing bearer session and admin token gate.
- The page displays the seven existing aggregate fields with accurate labels and keeps learner- and institution-level rows private.
- Loading, invalid access, API failure, and retry behavior are understandable and do not render missing data as zero.
- The owner can navigate from the editorial console to the dashboard and back.
- Layout has no horizontal overflow at mobile and desktop viewport widths and uses the locked app tokens.
- Rust owns the response DTO; checked-in generated TypeScript matches it after the ARCH-02 bootstrap.
- API integration and Playwright E2E jobs pass in GitHub Actions after implementation is complete.

## Scope

- Existing dashboard API handler and its exact seven-field response.
- Typed API client method and new dashboard route.
- One discoverable link from the existing admin console.
- Exact response-shape integration assertions and browser-flow coverage.

## Verification boundary

The test cases are authored as part of the slice. Build, unit, integration, and E2E execution are deferred until all implementation slices are complete; final acceptance requires fresh GitHub Actions evidence.
