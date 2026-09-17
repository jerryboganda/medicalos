# ADR 0002 — Technology stack: Rust backend, Tauri shells, Astro marketing site

Status: Accepted (owner decision 9, 17 September 2026) · Date: 2026-09-18

## Context

Master plan v1.0 proposed a TypeScript backend and Capacitor for mobile. Owner decision 9 replaced both. §20.1 records the Tauri mobile risk: remote push and in-app purchases currently depend on community plugins; WebView performance on low-end Android is unproven.

## Decision

- Backend and APIs: Rust modular monolith (Axum on Tokio, SQLx with compile-time-checked queries, generated OpenAPI, tracing + OpenTelemetry — candidates confirmed during Phase 0/1).
- Workers: Rust worker processes on a queue + transactional outbox.
- Desktop and mobile: Tauri 2. Mobile go/no-go is gated by the Phase 0 spike (issue 08); the recorded fallback is Capacitor for the mobile shell only (same SvelteKit UI + Rust backend).
- Marketing/checkout site: Astro.

## Consequences

Shared Rust core (§20.1) compiles for server, Tauri apps, and WebAssembly — no second implementation of business rules in TypeScript (§31.1). CI needs the Linux/macOS/Windows runner matrix with compile caching (§27.1). Owned Swift/Kotlin plugin code is expected for push, purchases, and capture protection (ARCH-03).
