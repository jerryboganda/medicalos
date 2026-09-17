# ADR 0003 — SvelteKit SPA as the single UI layer

Status: Accepted (owner decision 11, 17 September 2026) · Date: 2026-09-18

## Context

Decision 10 reaffirms that the same UI ships as responsive web + PWA alongside the Tauri shells (5C). SvelteKit must therefore run in static/SPA mode: server-only SvelteKit behavior cannot be embedded in the Tauri webview (§20.1, S32/S34).

## Decision

One `apps/client` SvelteKit + TypeScript SPA in static/SPA mode serves web, PWA, and both Tauri shells. Design tokens and Svelte components live in `packages/design-system`, shared with the Astro site. TypeScript types and the API client are generated from Rust/OpenAPI — never hand-edited (§31.1, ARCH-02).

## Consequences

No SSR-only features. Offline commitments differ by surface (§22: full in Tauri via encrypted SQLite; best-effort in browsers) and the UI must state that honestly. All UI screens are built through the hallmark skill; business logic never enters Svelte components (§31.1).
