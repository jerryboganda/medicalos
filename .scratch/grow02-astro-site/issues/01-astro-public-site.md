# GROW-02 — Public Astro site (landing, exams, pricing, help, legal, app-link files)

Status: ready-for-human
Requirement IDs: GROW-02, TRUST-01, CORE-09, EX-01
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §6, §26.1, §28.1, §31.1
Implementation state: complete — 12 static pages, deep-link endpoints, and the
Playwright seam (14 specs) pass locally against `astro build` + `astro preview`;
final CI validation joins the end-of-stretch batch run. Site deployment origin
and the real Tauri app identifiers remain owner inputs (see Out of scope).

## Problem

`apps/site` is a placeholder page. The master plan makes the Astro site a
Phase 1 deliverable, but the billing API (COM-02) does not exist and payment
rules are unconfirmed per market (§26.1). The site must go public without
pretending checkout, content packs, or outcomes exist (TRUST-01; §28 "no
nonfunctional buttons as finished features"; the pilot must be labelled a
pilot, §28).

## Solution

Static, typography-led Astro pages sharing `@medical-os/design-system`
tokens (hallmark: Narrative Workflow landing, Catalogue pricing/registry,
Index-First help, Long Document legal, N6 masthead nav, Ft1 footer, motion-cut).
All CTAs route to the live app's real free-tier entry points. `.well-known`
deep-link association files are config-driven with documented placeholders.

## Acceptance

See `../spec.md` — every page static, honest, mobile-safe, focus-visible,
reduced-motion safe; Playwright seam covers all pages and the 404.

## Out of scope

Web checkout + billing API (COM-02), coupons/regional prices (COM-03),
Tauri app identifiers and real deep-link fingerprints (first signed beta),
Astro site deployment target/origin (owner decision), waitlist/ambassador
mechanics (GROW-01 tails).
