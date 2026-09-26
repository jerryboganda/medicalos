# LIB-01 / LIB-03 / LIB-04 — Versioned article authoring and reading

Status: in-progress (implementation complete; final acceptance pending)
Requirement IDs: LIB-01, LIB-03, LIB-04, TRUST-01, ARCH-02
Source: `.scratch/lib01-article-authoring/spec.md`; `docs/requirements/traceability.md`; `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§12.1–12.2

## Acceptance

- Admins can create an unpublished version, create a revision from the latest published version, edit drafts, and publish exactly one draft at a time.
- Only authenticated users with the existing admin token can use editorial routes; every mutation records a compact actor/action audit event.
- Published versions are read-only; their content, source, citations, jurisdiction, and date window do not change when a later draft is edited.
- Citations preserve kind, anchor, and target; invalid kinds, blank fields, oversized values, invalid slugs, and reversed dates fail with stable validation errors.
- Learners can search and open a full published article. Article search and read honor explicit jurisdiction and inclusive as-of date filters.
- Exact-country content wins over global content. A different country's version is never a fallback; no match is an explicit not-available state.
- Learner pages show the selected jurisdiction/date and version citation targets, render article text safely, and work at 320, 375, 414, 768, and desktop widths without horizontal overflow.
- Rust-generated client DTOs are checked in and match the current Rust serialization contract.
- API integration, migration rollback/reapply (if a migration becomes necessary), type export, client build, and Playwright E2E all pass in fresh GitHub Actions.

## Seams

- Authenticated HTTP integration tests in `apps/api/tests/integration.rs`.
- Playwright flows in `tests/e2e/` against the existing SvelteKit routes.
- Existing ts-rs export and generated-file drift checks.

## Verification boundary

Tests are authored during implementation and executed only after all requested code slices are complete. Clinical validation, rights clearance, merge, and deployment remain separate gates.
