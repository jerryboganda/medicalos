# ADR 0005 — One combined platform (owner decision 13)

Status: Accepted (owner decision 13, 17 September 2026) · Date: 2026-09-18

## Context

Quiz LMS and Medical Learning OS began as separate plans (v1.0, 169 sections, mobile-only). The v2.0 merge reconciled them into one canonical plan, relocating every Quiz LMS feature (Appendix C maps all 169 sections; nothing was dropped) and restoring desktop per decisions 9–10.

## Decision

Quiz LMS and Medical Learning OS are one platform. This repository implements the merged plan only; there is no separate Quiz LMS codebase, engine, or nav stack.

## Consequences

Session vocabulary is the §11.2 one-session model (Tutor / Timed exam-style / Review / Revision / Smart Practice as presets — no duplicate engines). Engagement mechanics follow §17.2 phase assignments. Navigation is the §6 five-destination model, not Quiz LMS's five tabs.
