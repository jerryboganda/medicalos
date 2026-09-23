# GROW-02 — Astro marketing site (public, learner-data-free)

Requirements: GROW-02, TRUST-01, CORE-09, EX-01 (public face)
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §6 (stack), §26.1 (payment routes),
§28.1 Phase 1 ("Astro site with checkout"), §31.1 ("The Astro site holds no
learner data and calls only public or checkout API endpoints").

## Scope

Replace the `apps/site` placeholder with a static Astro site: landing,
exam-registry pages, pricing, help, download, legal (terms + privacy),
app-link association files, robots. The site shares the owner-locked
`@medical-os/design-system` tokens with the app and never renders or
fabricates learner data (TRUST-01: no invented success rates, counts,
testimonials, or prices).

## Honest-commerce rule

The billing/checkout API does not exist yet (COM-02 not-started; §26.1 says
payment rules must be confirmed per market before building the paywall).
The pricing page therefore presents the approved 7C tier *names and feature
differences* with prices explicitly marked "to be announced" and CTAs that
route to the free tier of the live app. No fake payment form, no invented
prices, no "buy" button that cannot complete. Web checkout arrives with the
COM-02 slice; this spec deliberately does not stub it.

## Acceptance

- Every page renders static HTML from shared layout + tokens; mobile-safe at
  320–768 px; no horizontal scroll; focus-visible rings; reduced-motion safe.
- Landing leads with the real product loop (aim → plan → practice → evidence);
  each stage names shipped features only. CTAs link to the live app
  (guest trial / sign-up) — real, reachable destinations only.
- Exam registry page lists the §registry families with honest availability
  status ("in development", "registry planned"). Per-exam pages exist only for
  families with drafted blueprint scope, each carrying a status banner. The
  synthetic pilot exam is labelled a pilot fixture, never marketed as content.
- Pricing page shows the five 7C tiers with feature differences grounded in
  the master plan; every price cell reads "to be announced"; an honest note
  states web checkout is not open yet.
- Help page answers real, shipped behaviors (accounts/device limit, offline
  packs, free allowance, question reporting, account deletion).
- Legal pages are drafts grounded in actual data practices, marked for owner
  review; they describe beta/pilot status truthfully.
- Download page links the live web app and states desktop/mobile shell status
  honestly (internal beta, not published).
- `.well-known/assetlinks.json` and `apple-app-site-association` are emitted
  from one config module with placeholder package identifiers documented as
  pending the first signed Tauri beta build.
- Playwright E2E exercises every page for: title, main heading, honest-CTA
  destinations, absence of fabricated metric patterns, and 404 behavior.

## Seams

- Static Astro build (`npm run build -w @medical-os/site`, already a CI gate).
- Playwright specs under `tests/e2e/site-*.spec.ts` with a second webServer
  for the site preview.
- No API surface; no database; no client app changes.
