# ADMIN-01 — Owner dashboard UI

Status: tested (GitHub Actions run 36245924351; ADMIN-01 API, binding, route, and E2E files unchanged through 63f6509)
Requirement IDs: ADMIN-01, ARCH-02

## Problem Statement

The API already reports platform-wide administrative aggregates, but owners have no browser surface to inspect them. The existing `/admin` page is a long editorial console, so operational totals are difficult to find.

## Solution

Add a focused `/admin/dashboard` page that displays the seven existing aggregate counts, links to the editorial console, and uses the existing session and admin-token gates. Generate the dashboard response type from its Rust DTO so the UI does not duplicate the wire contract.

## User Stories

1. As a platform owner, I want to see the current institution and account totals so that I can quickly understand platform scale.
2. As a platform owner, I want published-question, article, and rights-record totals so that I can monitor the content catalogue without opening each workflow.
3. As a platform owner, I want unresolved incident and 30-day Coach-turn totals so that I can spot operational and usage changes.
4. As an operator, I want the dashboard to show aggregate counts only so that the page does not expose learner- or institution-level records.
5. As an administrator, I want clear loading, access, and retry states so that a slow or unavailable API does not look like zero activity.
6. As an administrator, I want the dashboard to fit mobile and desktop screens so that I can use it on a narrow device.
7. As an administrator, I want a direct link to the editorial console so that I can move from summary to existing operations.

## Implementation Decisions

- Reuse the existing `GET /v1/admin/dashboard` query and its seven current fields: institutions, users, published questions, articles, rights records, open incidents, and Coach turns in the last 30 days.
- Replace the anonymous Rust response with a serializable Rust DTO exported through `ts-rs`; keep the JSON field names and values unchanged.
- Add the typed API method and a new `/admin/dashboard` SvelteKit route. Keep the current bearer-session plus admin-token authorization contract.
- Add a discoverable dashboard link to the existing admin console. Use the owner-locked app tokens, app shell, and motion-cut behavior.
- Display zero as a valid count. Do not show trends, rankings, institution rows, or any metric that the API does not provide.
- Add no database migration and no new dependency.

## Testing Decisions

- Extend the existing authenticated API integration flow to assert the exact seven response keys and non-negative integer values.
- Add Playwright coverage for token entry, dashboard data rendering, access failure/retry, and responsive layout at mobile and desktop widths.
- Keep tests at the HTTP and browser seams. Author them now; defer execution until the remaining implementation slices are complete, as directed by the user.

## Out of Scope

- Changes to aggregate definitions or admin authorization.
- Per-tenant or per-learner analytics, time-series charts, or exports.
- Support tickets, model-routing controls, pricing, and licensing workflows.
- Production deployment or owner acceptance of business metrics.
