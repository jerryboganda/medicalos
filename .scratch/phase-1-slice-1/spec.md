# Spec — Phase 1 Slice 1: Connected Question Loop (backend)

Requirement IDs: CORE-01(partial), CORE-10(partial), QB-01, QB-03, QB-04, QB-05, QB-11, QB-14, EX-01(partial), EX-04, AI-01, AI-02, AI-07(minimal), AI-17(partial), PLAN-01(partial), PLAN-02(partial). Source: master plan §11.1–11.3, §8.2–8.8, §9, §21.2, §30.1. Triage: `ready-for-agent`.

## Problem Statement

The platform has no working learning loop. A learner cannot register, see a study plan, answer a real reviewed question, receive an explanation, and observe the system's plan change — persisted, justified, and reversible — which is the Phase 1 gate (§28).

## Solution

A backend-only vertical slice: register/login, a tiny exam registry + hierarchy, published immutable question versions (2–10 options, per-option rationale, key learning point), tutor practice sessions with durable idempotent answers, observed learner state with honest sparse-data behavior, and a deterministic today-plan that revises itself after a session (with receipt + undo). No UI, no LLM, no billing, no Tauri in this slice.

## User Stories

1. As a learner, I want to register and sign in, so that my study record belongs to me.
2. As a learner, I want to start a tutor practice session in a chapter, so that I can study that chapter's questions.
3. As a learner, I want to answer a question and immediately see whether I was right, the rationale for the options, and the key learning point, so that the answer teaches me something.
4. As a learner, I want my answer persisted durably with an idempotency key, so that a retry or bad connection can never duplicate or lose it.
5. As a learner, I want to skip a question, so that I am not forced to guess.
6. As a learner, I want to submit a session and see my score, so that I know where I stand.
7. As a learner, I want today's plan to add a focused re-practice task after a session with wrong answers, so that my mistakes turn into a next action.
8. As a learner, I want every plan change to say what changed and why, so that the system is trustworthy.
9. As a learner, I want to undo a plan revision, so that I stay in control of my plan.
10. As a learner, I want mastery shown as 'Not enough evidence' until I have real data, so that no fake readiness is shown (decision 12, AI-02).
11. As a learner, I want to re-practice exactly the questions I got wrong or skipped, so that revision targets my gaps.

## Implementation Decisions

- **Seam (testing):** the HTTP API is the only seam tested — POST /v1/auth/{register,login}, GET /v1/me/today, POST /v1/practice/sessions, POST /v1/practice/sessions/{sid}/answers, POST /v1/practice/sessions/{sid}/submit, POST /v1/plans/{pid}/revisions/{rid}/undo. Highest public seam (§21.2 paths); no internal-function tests.
- **Stack:** Axum 0.8 + SQLx (compile-time-checked `query!` macros per §31.1) against PostgreSQL 17 in CI; single error format `{error:{code,message}}` (§21.2); bearer tokens (random, stored as SHA-256), Argon2id password hashing; UUIDs everywhere.
- **Content model:** `questions` (family identity) + immutable `question_versions` (status published; options JSONB array with text+rationale, length enforced by `domain-contracts::OptionCount` in Rust; key_learning_point, exam_tip, high_yield, authored difficulty, source_ref). Chapters live in `curriculum_nodes` (subject→system→chapter with display_order) — CORE-10 minimal; aliases/versioned blueprints arrive with the full exam-pack work.
- **Evidence:** `attempts` rows are the learning evidence — unique (session, item) and (session, idempotency_key); skipped = chosen NULL; `assisted=false` only for now; first answer wins (re-answer rejected 409 — answer-change analysis is Phase 2).
- **Learner state:** Elo-style per (user, chapter): ability 1500 start, difficulty tiers easy=1350/medium=1500/hard=1650, K shrinks with evidence (max(8, 32−2·independent)); displayed as observed accuracy + evidence level (`low_evidence` under 10 independent attempts — AI-02, §8.8); never a prediction (ADR 0004).
- **Plan + revisions:** one plan per user per day, versioned rows; revisions are new versions with copied tasks + the added task; `plan_revisions.receipt` JSONB carries triggering evidence, checks, and diff (AI-07 minimal — a full receipts table comes with the Coach); undo deletes the revision's added tasks in a new version. Revisions are deterministic (no LLM) per §9.1/§8.5: submit with incorrect/skipped on a **tutor** session → one revision task (re-practice wrong+skipped, capped). Revision sessions themselves spawn no further revisions (anti-loop, §8.6) — the SR-08 re-test queue is Phase 2.
- **Seed:** synthetic fictional content only (a fictional 'gloopoid gland' system), clearly labeled as fixtures — never real medical claims in tests (§27).
- **Schema application:** one forward SQL migration + paired down file applied by the same Rust helper everywhere (tests, CI bootstrap via psql, startup); CI proves up→down→up reversibility (§31.1 rollback guardrail).

## Testing Decisions

- Tests verify public behavior through the API only, never internals (tdd skill).
- Fixtures: unique random emails per test; seeded synthetic content; shared CI database is safe because all rows are user-scoped with random UUIDs.
- Prior art: none (greenfield) — this slice establishes tests/integration.rs as the pattern.
- Cases: register/login; full loop (today cold start → session → answer → replay no-duplicate → submit → revision exists with receipt → undo); revision-session pool equals wrong+skipped set; learner state honest low-evidence; migration up→down→up.

## Out of Scope

UI of any kind (hallmark governs the next slice), Coach/LLM turns, billing/entitlements, offline layer, Tauri shells, editorial console (bulk fixture seeding only), full exam-pack templates, spaced repetition, answer-change analysis, multi-device sessions/limits, notifications.

## Further Notes

Open owner decision 09 (pilot exam pack) does not block this slice: the registry is exam-agnostic and the seed exam is a placeholder. Slice 2 candidates: minimal SvelteKit client for this loop (hallmark), Coach explanation turn, review-mode replay.
