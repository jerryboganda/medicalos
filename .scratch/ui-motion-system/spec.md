# UI motion system and app-shell overhaul

Status: in-progress (first push green: CI run 36358455756, 71/71 e2e)
Requirement IDs: §6 (IA, global command control), §7.1 (visual direction), §7.3 (functional visual rules: light reading theme, high contrast, reduced motion, 44–48 px targets), §7.4 (responsive shell), UX-01, UX-02, TRUST-06 (partial: a11y states only)

## Problem

1. The shell contradicts §7.4. Every signed-in page has a wrapping 12-button top bar. There is no side navigation on desktop, no bottom navigation on mobile, and no global command control (§6).
2. The shared UI layer is thin. It has one button style and one card style, and no motion vocabulary beyond a hover colour change. About 140 inline `style=""` attributes carry layout across pages, so the same row is spelled five different ways.
3. Real defects:
   - The global `input` rule stretches checkboxes and radios to 100 % width and 44 px height.
   - `textarea` is unstyled outside the session page.
   - `.danger-text` has no colour outside buttons.
   - `--text-xs` is used but never defined.
4. §7.3 requires a light reading theme and a high-contrast mode. Neither exists.
5. The marketing site and the client duplicate their base, button and chip CSS.

## Scope

- `packages/design-system` becomes the single source for tokens, base, components and motion. Both apps import it; app stylesheets keep only app-specific rules.
- Client shell:
  - Desktop: side rail.
  - Mobile: top bar, bottom tab bar, and a sheet holding the full "Main navigation".
  - Study sessions: no primary nav (§6), but the brand link stays.
  - ⌘K / Ctrl+K command palette on native `<dialog>`: navigation plus two real shell actions (theme, sign out). It never pretends to search content.
  - Route transitions through the View Transitions API.
  - Light reading theme toggle (circular view-transition reveal). High contrast follows `prefers-contrast: more`.
  - Native-app readiness for the PWA and the future Tauri shells: `viewport-fit=cover` + safe-area insets, `theme-color` that follows the theme, a web manifest and the brand emblem as favicon.
  - Tab bar labels §6's "Learn" destination "Library": concrete beats abstract for a five-tab bar.
- Motion vocabulary, each with a reduced-motion collapse:
  - page-content entrance stagger
  - button press and hover lift
  - option select and answer-feedback reveal
  - sheet and dialog enter/exit
  - loading-copy shimmer (`.is-loading`) for real loading states
  - goal progress fill
  - nav indicator morph
- Pages: inline layout styles replaced with named utilities; page-level polish on Today (§7.2 greeting/date, one primary action, goal meter), Practice, Progress, Coach, Review, Notes, Notifications, Login, and the session results view; per-page `<title>`s.
- Site: shared system adopted, atmospheric hero, hand-built loop diagram (SVG), scroll-driven reveals, cross-document view transitions.
- `design.md` at the repo root locks the system for future work.

## Non-goals

- No new runtime dependencies (motion is CSS, the View Transitions API, and Svelte built-ins).
- No Tauri shell scaffolding: mobile is still gated on the Phase 0 spike (issue 08).
- No copy changes to claims, no invented metrics, no count-up animations that rewrite asserted text.
- No decorative motion inside timed tasks (§7.3). Session feedback motion is functional only and ≤ 150 ms.

## Contracts the redesign must keep (Playwright)

- Every `data-testid`, label text, heading text, and button name in use stays.
- At 320–768 px there is exactly one visible button named "Menu". It toggles `navigation "Main navigation"` and becomes "Close menu" while open. No other visible control name may contain "menu".
- Exactly one visible link named "Medical Learning OS" on every signed-in route, including `/session/*`.
- No new visible heading containing "Today" on `/today`. No new visible button name containing "Search".
- Diagnostic images keep `filter: none`, and the watermark keeps `pointer-events: none`.

## Acceptance

- The CI `client`, `site`, and `e2e` jobs are green on the branch.
- New `ui-shell.spec.ts` covers:
  - the desktop rail
  - the mobile sheet
  - palette open, filter, and navigate
  - theme persistence
  - nav hidden inside a session
- No horizontal overflow at 320 / 375 / 414 / 768 px on the shell.
