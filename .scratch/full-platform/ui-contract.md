# Mobile navigation UI contract

## Locked shell

- At mobile and tablet widths, preserve the current top bar, bottom tab bar, and full navigation sheet. Keep the existing `Menu` / `Close menu` names and the `Main navigation` landmark.
- The existing `app.css` rules remain the visual authority for the sheet, scrim, and breakpoints. The shared design system continues to own focus treatment and reduced-motion behavior. This accessibility change adds no visual styles, tokens, spacing, navigation items, or renamed controls.
- The desktop side rail and the session navigation rules remain as defined by `design.md` § App shell.

## Keyboard and background behavior

- Activating the existing menu toggle moves focus to the first interactive item in `Main navigation` after the sheet opens.
- While open, Tab and Shift+Tab stay within the menu toggle and the sheet's links and buttons.
- The page content, quick-navigation tab bar, brand link, quick-jump button, and skip link are inert while the sheet is open and return to their normal state when it closes.
- Escape closes the sheet and restores focus to the existing menu toggle. The toggle and scrim also close it and restore focus; selecting a destination closes the sheet through the existing route flow.
- Crossing to the desktop breakpoint closes the mobile state and clears inertness. Focus already in the navigation stays on its visible desktop item; focus on the disappearing toggle moves to the current page link.
- Route changes clear the mobile state and inertness. They do not restore focus to the mobile toggle, which may no longer be present on the destination route.
- No new animation is introduced. The locked reduced-motion rules remain in force.

## Acceptance seam

The browser contract lives in `tests/e2e/ui-shell.spec.ts`: the mobile-menu regression covers keyboard opening, focus transfer and containment, inert background regions, Escape dismissal, focus restoration, desktop resize cleanup, and route cleanup. The full-platform spec names the core focus and inertness behaviors in user story 5 and its testing decisions.

## Source of truth

- `design.md` § Motion and § App shell.
- `.scratch/ui-motion-system/spec.md` § Contracts the redesign must keep and § Acceptance.
- `.scratch/full-platform/spec.md` user story 5 and § Testing Decisions.
- `apps/client/src/routes/+layout.svelte` owns menu state and semantics; `apps/client/src/app.css` owns its existing appearance and responsive motion.
