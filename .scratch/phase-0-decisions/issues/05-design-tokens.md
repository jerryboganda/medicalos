# 05 — Design tokens (§7.1)

Status: implemented (packages/design-system/tokens.css; contrast validation happens in Phase 1 hallmark screens)
Requirement IDs: supports CORE-05/CORE-06/UX-01 (P1); §7.1 'proposed starting tokens'

## Task

`packages/design-system/tokens.css` carrying the §7.1 table verbatim as CSS custom properties (canvas, surfaces, primary action, accent, text, 4-pt spacing scale, radii, motion durations), each labeled proposed/unvalidated. No contrast certification yet — §7.1 explicitly requires validating every foreground/background combination in real screens (hallmark runs the validation during Phase 1 UI work).

## Acceptance evidence

Token file exists with §7.1 values; Phase 1 screens consume tokens by name, never inline hex.
