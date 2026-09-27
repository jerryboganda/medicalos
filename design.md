# Design — Medical Learning OS

The locked design system for every surface: the SvelteKit client (web/PWA and the Tauri desktop and mobile shells) and the Astro marketing site. Read this before touching UI. Extend or amend this file when the system needs to grow; don't override it per page.

Code lives in [`packages/design-system/`](packages/design-system/):

| File | Owns |
|---|---|
| `tokens.css` | Every colour, space, type, radius, shadow and motion value, plus the light reading and high-contrast themes |
| `base.css` | Element defaults: body, headings, links, focus ring, selection, skip link, scrollbars |
| `components.css` | Shared components: `.btn`, `.card`, `.chip`/`.status`, `.field` + inputs, `.feedback`, `.stat-row`, `.meter`, `.kbd`, `.is-loading` |
| `motion.css` | Keyframes, entrance stagger, scroll reveal, view-transition defaults, reduced-motion collapse |
| `layout.css` | Layout utilities that replace inline `style=""`: `.row`, `.cluster`, `.form-row`, `.bare-list`, `.flush`, `.small` |

App-only rules (the shell and the session workspace) live in `apps/client/src/app.css`. Site-only rules live in `apps/site/src/styles/site.css`. Page `<style>` blocks hold only what is unique to that page.

## Genre and tone

**Atmospheric**, calm and utilitarian. §7.1 sets the direction: a near-black canvas, a controlled plum/violet atmosphere, soft elevated cards, subtle borders, restrained glow, and one visually dominant action per view. Reading always beats atmosphere (§7.3): there is no glow behind vignettes, no tint on diagnostic images, and no decorative motion during timed tasks.

## Theme (owner-locked, §7.1)

| Token | Dark (default) | Light reading |
|---|---|---|
| `--color-canvas` | `#09090f` | `#f7f6fb` |
| `--color-surface` | `#14141e` | `#ffffff` |
| `--color-surface-elevated` | `#1b1728` | `#efedf7` |
| `--color-action-primary` | `#7c3aed` | `#6d28d9` |
| `--color-accent` | `#a78bfa` | `#6d28d9` |
| `--color-text-primary` | `#f5f3ff` | `#17141f` |
| `--color-text-secondary` | `#b8b4c6` | `#57536a` |

- **Dark** is the product default. **Light reading** is opt-in and persists per device (`mlos_theme`).
- **High contrast** follows the OS through `prefers-contrast: more` and needs no toggle.
- New colours are derived from these tokens with `color-mix()` in `tokens.css`; they are never inlined in components.
- One violet action per viewport. The accent (`#a78bfa`) marks focus, links, and the active nav indicator only.

## Typography

- Display: **Fraunces 600**, roman only (italic headings are banned).
- Body: **Manrope 400 / 600 / 700**. Reading text is 16–18 px (§7.3).
- Numbers that update (timers, counts, stats) use `font-variant-numeric: tabular-nums`.
- Both families are self-hosted through `@fontsource`. There is no font CDN (offline-first, §30.2).
- Scale tokens: `--text-xs` 12, `--text-sm` 14, `--text-body` 16, `--text-body-lg` 18, `--text-lg` 20, `--text-xl` 24, `--text-2xl` 30, `--text-3xl` clamp(30–40), `--text-display` clamp(38–68).

## Space, radius, elevation

- Spacing follows the 4-pt scale from §7.1 (`--space-xs` 4 → `--space-3xl` 48), plus `--space-4xl` 64 for marketing sections only.
- Radius: `--radius-card` 20, `--radius-control` 12, `--radius-sm` 8, `--radius-pill`.
- Elevation, low to high:
  - `--shadow-1`: cards, a hairline highlight plus a soft drop.
  - `--shadow-2`: the rail, bars, and floating chips.
  - `--shadow-3`: dialogs and sheets.
  - `--glow-action` is the restrained violet glow for the one primary action.

## Motion

Motion must communicate something: arrival, the result of a press, or a state change. If removing an animation would lose nothing, it goes.

| Token | Value | Use |
|---|---|---|
| `--dur-fast` | 120 ms | Press, toggle tick, colour shift |
| `--dur-base` | 200 ms | Hover, focus border, tooltip, small reveals |
| `--dur-slow` | 320 ms | Sheet, dialog, feedback panel, page content |
| `--dur-page` | 420 ms | Page-load orchestration, marketing reveals |
| `--ease-out` | `cubic-bezier(0.22, 1, 0.36, 1)` | Anything entering |
| `--ease-in` | `cubic-bezier(0.7, 0, 0.84, 0)` | Anything leaving (~75 % of the enter duration) |
| `--ease-in-out` | `cubic-bezier(0.65, 0, 0.35, 1)` | State toggles |
| `--ease-spring` | `linear()` spring, ≤ 3 % overshoot | Physical surfaces only: the sheet, the dialog, the nav indicator |

The vocabulary, and nothing outside it:

1. **Route change.** The View Transitions API fades the old page out (120 ms) and the new one in (200 ms) while its content staggers up. The chrome stays put and the active nav pill glides to the new item.
2. **Page entrance.** Direct children of `<main>` rise 8 px and fade in, staggered 45 ms each and capped at 8 items (about 360 ms total). The entrance plays once per insertion, never on scroll.
3. **Press.** Buttons scale to 0.97 on `:active`. On hover-capable pointers only, buttons lift 1 px.
4. **Loading.** Buttons show an inline spinner via `data-loading="true"`. `.is-loading` text shimmers only while a real request is in flight (§7.3: no fake "analyzing"). Put it on the text element, never on a card.
5. **Feedback.** Answer feedback, alerts and notices fade in with an 8 px rise. A correct option gets a check stamp, an incorrect one a single 120 ms tint. Nothing loops.
6. **Surfaces.** The sheet slides up and the dialog scales from 0.96, both via `@starting-style`. Backdrops fade.
7. **Progress.** Meters fill with `transform: scaleX()` over 600 ms `--ease-out`.
8. **Marketing only.** An ambient aurora drifts in the hero, the loop diagram's pulse travels, and sections reveal with scroll-driven animation. All of it stops under reduced motion.

Hard rules:

- Animate only `transform` and `opacity` (the one exception is colour/border on state changes).
- Focus rings appear instantly and never animate.
- `prefers-reduced-motion: reduce` collapses everything to opacity at ≤ 150 ms and skips view transitions. Spinners and shimmer keep running, slower, because they are functional.
- The session workspace (`/session/*`) allows only functional motion (press, select, feedback) and no entrance stagger (§7.3).
- Never animate text content that tests or screen readers read (no count-up on asserted values).

## App shell (client, §7.4)

- **Desktop (≥ 1024 px):** a sticky 264 px side rail holds the brand, the ⌘K "Quick jump" trigger, grouped navigation (Study · Learn · You · Workspaces), the theme toggle and sign-out. Content is centred to a 760 px reading measure (wider for workbench pages via `.wide`).
- **Mobile and tablet (< 1024 px):** a sticky top bar holds the brand, "Quick jump" and "Menu". A bottom tab bar holds Today · Practice · Library · Progress · Coach. "Menu" opens a bottom sheet holding the full "Main navigation".
- **Study session (`/session/*`):** the primary nav is hidden (§6). A slim bar keeps the brand link as the way out, and Focus Mode hides that too.
- **Signed out:** only the brand bar, with the login page as a centred atmospheric card.
- **Navigation data** lives in one place, `apps/client/src/lib/nav.ts`, and the rail, sheet, tab bar and command palette all read it. Add a destination there, once.
- Safe-area insets (`env(safe-area-inset-*)`) pad the top bar, the tab bar and the sheet for notched phones and Tauri mobile.
- Touch targets are ≥ 44 px (§7.3).

## Component voice

- **Primary button:** violet gradient fill, white text, `--glow-action`. One per view.
- **Secondary button:** the elevated surface with a hairline border.
- **Destructive:** `.danger-text`, an outline in the error colour.
- **Button labels** are verb-first ("Start session", "Replan my day") and never wrap (`white-space: nowrap`).
- **Cards:** a surface fill, hairline border, top highlight and `--shadow-1`. A card nested inside a card becomes a tile (elevated fill, `--radius-control`, tighter padding).
- **Status:** always icon or text plus colour, never colour alone (§7.3). Chips carry a leading dot.
- **Inputs:** a canvas fill and hairline border. Focus shows an accent border plus the instant focus ring. Invalid shows the error border plus a message.

## Marketing site (Astro)

- **Macrostructure:** Marquee Hero on the landing page. Catalogue, Index-First and Long Document on inner pages.
- **Nav:** a floating pill header (N5), sticky with a blur backdrop.
- **Footer:** a statement footer (Ft5).
- **Enrichment:** Tier-A CSS aurora plus a Tier-B hand-built SVG of the product loop (aim → plan → practice → evidence). No stock imagery, no fake screenshots, no re-drawn browser chrome.
- **Cross-document view transitions:** `@view-transition { navigation: auto; }`.
- **Honest copy is enforced by `site-pages.spec.ts`:** no prices, no success-rate claims, no testimonials, and external links go only to the app origin.

## What every surface must share

The tokens, the two fonts, the button and card voice, the focus ring, the motion tokens, and the brand emblem (`packages/design-system/emblem.svg`, an ECG pulse on a violet tile).

## Exports

`tokens.css` is the canonical export. A Tailwind v4 `@theme` or DTCG mirror is deliberately not maintained; add one only when a consumer needs it.
