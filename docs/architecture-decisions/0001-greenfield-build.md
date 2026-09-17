# ADR 0001 — Greenfield build

Status: Accepted · Date: 2026-09-18 · Deciders: owner via master plan §32 (execution: agent audit)

## Context

Master plan §32 requires auditing the actual repository before choosing greenfield vs reuse. The working directory `D:\Projects\Medical OS` contained only `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` before Phase 0 scaffolding began (verified 2026-09-18). The plan's preamble forbids assuming permission to modify the existing OET platform, reuse its private content, or deploy into its production environment.

## Decision

Build greenfield in this repository per §20.4. No code, content, or infrastructure is reused from the OET platform.

## Consequences

No legacy-migration work; every requirement starts from the traceability ledger. The existing production VPS remains out of scope entirely (AGENTS.md compute policy also bars using it for compute).
