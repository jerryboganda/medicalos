# Remaining Phases Completion Spec

Status: ready-for-agent

## Problem Statement

Medical OS has progressed through multiple vertical slices, but the traceability ledger still contains incomplete requirements across Phases 1-7 and is stale relative to the current `main` commit. The product needs the remaining code-addressable behavior completed without inventing evidence for gates that require real devices, stores, clinical review, rights clearance, external validation, accreditation, or owner approval.

## Solution

Complete the remaining software behavior by deepening the existing HTTP API and browser UI seams rather than creating parallel architectures. Reconcile already-landed work into the ledger, implement missing behavior in small vertical slices, validate each slice in GitHub Actions, and leave externally unverifiable gates explicitly blocked with the exact evidence required.

## User Stories

1. As a learner, I want one identity, explicit device sessions, navigation, practice, review, notes, library, Coach, Progress, notifications, offline sync, and exports to work coherently across clients.
2. As a learner, I want practice generation to respect exam blueprints, question families, assistance/confidence evidence, timing, accommodations, protected assessment rules, marks, re-tests, and honest empty states.
3. As a learner, I want capacity-aware plans, protected commitments, review-debt recovery, delayed intervention outcomes, editable Coach memory, and source-grounded tutoring without fabricated readiness.
4. As a learner, I want reviewed cards, notes, source-linked library content, media metadata, offline packages, and portable imports/exports with provenance retained.
5. As a learner, I want cross-device reconciliation to be idempotent, conflict-safe, lease-aware, freshness-aware, and incapable of duplicating acknowledged learning evidence.
6. As an institution administrator or instructor, I want scoped roles, programs, cohorts, assignments, assessment author/reviewer/publisher separation, curriculum coverage, privacy-preserving analytics, and interoperability endpoints.
7. As a community participant, I want opt-in handles, groups, challenges, leagues/duels, integrity review, and non-coercive gamification.
8. As a learner in clinical practice labs, I want authored scenarios, patient/examiner separation, rubric evidence, transcript uncertainty, deterministic state transitions, debriefs, appeals, imaging metadata, and supervised feedback.
9. As a global learner, I want locale-safe UI strings, RTL support, exam-switch gap reports, and multilingual tutoring only when evaluation evidence exists.
10. As an operator/editor, I want tenant/audit controls, content-rights/provenance operations, product flags, revenue/license records, recovery evidence, staged updates, and release compatibility metadata.
11. As a product owner, I want prediction, accreditation, store/device, clinical, rights, and externally validated gates reported honestly rather than marked complete from code alone.

## Implementation Decisions

- Preserve the current Axum HTTP API as the primary backend interface and the existing SvelteKit routes as the browser interface.
- Prefer additive migrations and existing tables/modules; introduce a new table only when the requirement represents durable state that cannot live safely in an existing structure.
- Keep shared deterministic business rules in Rust crates when they are reused across server/client; otherwise keep the smallest implementation behind the HTTP seam.
- Use the existing admin-token and authenticated-user patterns until a requirement specifically needs tenant-scoped role checks; tenant-scoped checks must use institution membership rather than global admin authority.
- Use server-authoritative timestamps and persisted receipts for assessment, sync, plan, integrity, and entitlement behavior.
- Do not implement numerical readiness claims without validated outcome evidence; expose coverage/evidence state instead.
- Do not call CE activity accredited without a real accreditation/provider workflow.
- Do not mark native-device, store, rights, clinical-review, or external-validation gates tested from unit/integration code alone.
- Reconcile ledger rows for work already present in current `main` before adding new behavior.

## Testing Decisions

- Primary seam: authenticated/public HTTP endpoints exercised by `apps/api/tests/integration.rs`.
- Browser seam: existing SvelteKit routes through Playwright in GitHub Actions where UI behavior changes.
- Shared Rust crates retain native + wasm32 CI gates.
- Every non-trivial branch added in a slice gets at least one integration test that would fail without the behavior.
- No local builds, test suites, screenshots, or generated release artifacts; GitHub Actions is the compute authority.
- Migration rollback remains part of the API integration gate.

## Out of Scope

- Fabricating clinical review, rights clearance, app-store approval, device-lab evidence, signed release proof, accreditation, real SSO-provider certification, or validated prediction performance.
- COM-04 until the owner resolves the explicit commercial proposal decision.
- Replacing the approved Rust/SvelteKit/Astro/Tauri architecture.

## Further Notes

The canonical source remains `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md`; this spec only converts its still-open ledger into an implementation program. Phase gates remain stricter than code completion and must name external blockers where applicable.
