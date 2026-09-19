# Requirement Traceability Ledger

Every stable requirement ID from master plan §29, its phase, and its delivery status. **No implementation ticket ships without at least one ID from this ledger.** A ticket's completion report must distinguish implemented / tested / partial / blocked / deliberately deferred (master plan §31).

Status vocabulary: `not-started` · `in-progress` · `tested` (acceptance evidence filed) · `blocked` (named blocker) · `deferred` (deliberate, with reason).

Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §29. Phase definitions: §28 + §28.1. Release gates: §30.

**Evidence run for Phase 1 slice 1 (backend loop):** https://github.com/jerryboganda/medicalos/actions/runs/35280682748 — IDs marked `tested (slice-1 scope)` have seam-test coverage for the scope defined in `.scratch/phase-1-slice-1/spec.md`; they remain in scope for the rest of Phase 1 (UI, multi-exam, timed presets, etc.).

## Phase 0 — Decisions and evidence

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| OPS-01 | GitHub Actions development-compute enforcement | in-progress (enforced; proven by green CI run 35273824336) | .scratch/phase-0-decisions/issues/04 |
| OPS-02 | Runtime boundary approval before deployment | blocked (owner decision) | issues/10 |
| OPS-07 | Rust + Tauri CI runner matrix under the compute rule | tested (Linux gates green, run 35273824336; macOS/Windows signing jobs arrive with Tauri shells) | issues/04 |
| ARCH-01 | Shared Rust core across server, Tauri, and WebAssembly | in-progress (three shared crates — domain-contracts, calc-engine, competition-scoring — proven native + wasm32 in CI; runtime parity check in the Phase 0 spike remains, issues/08) | runs 35273824336, 35367410576; issues/08 |

## Phase 1 — Connected vertical slice

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| CORE-01 | One identity with personal and institution contexts | in-progress (identity/auth plus explicit personal + multi-institution context resolution tested; downstream institution-scoped product data remains) | .scratch/phase-1-slice-1/; .scratch/phase-1-core04-tenant-audit/; run 35425807184 |
| CORE-02 | Versioned goals, exam dates, protected commitments | tested | .scratch/phase-1-core02-goals/; run 35424883754 |
| CORE-03 | Entitlement checks across API, media, retrieval, offline manifests | in-progress (free-tier daily question allowance enforced server-side with honest 403 + details payload; media/retrieval/manifest entitlements pending) | run 35372696754 |
| CORE-04 | Multi-tenant role and audit foundations | tested (foundation scope: explicit tenant context, multi-role memberships, separate platform roles, owner bootstrap, tenant-admin assignment, idempotency, append-only audit receipts, tenant isolation) | .scratch/phase-1-core04-tenant-audit/; run 35425807184 |
| CORE-05 | Five-destination learner navigation | blocked (implementation complete for Today/Practice/Learn/Coach/Progress shell and real capability paths; final browser acceptance cannot execute because GitHub Actions is blocked by account billing/spending-limit state before runner steps) | `.scratch/phase-1-core05-navigation/`; red run 35429347073; blocked runs 35429885728, 35430203458 |
| CORE-06 | Truthful loading, error, empty, permission states | tested (slice-2 scope: loading/error+retry/empty/no-evidence states in the learner UI; no fake analytics anywhere) | run 35366131810 |
| CORE-07 | Sign-in methods, in-app account deletion, device limit, single active session | in-progress (email/password security, rotating device sessions, two-device cap, deletion initiation, single-study-session takeover, Account UI tested; production registration/recovery fail closed until real email delivery; Google/Apple sign-in pending) | `.scratch/phase-1-core07-accounts/`; run 35428827015 |
| CORE-08 | Notification policy, push + in-app inbox | blocked (foundation implemented: personal preferences, quiet hours/time zone, write-only mobile token registration, in-app inbox/read state, Account UI; final acceptance cannot execute because GitHub Actions is billing/spending-limit blocked before runner steps; real APNs/FCM transport, delivery caps/quiet-hour enforcement, campaign opt-out analytics, and real event producers remain pending) | `.scratch/phase-1-core08-notifications/`; blocked run 35430685017 |
| CORE-09 | Guest trial before sign-up (SHOULD) | not-started | — |
| CORE-10 | Navigational hierarchy mapped to concept identities | in-progress (hierarchy plus stable/versioned concept identities and seeded curriculum/question mappings implemented; learner evidence is still chapter-keyed and cross-modality mappings remain pending; acceptance CI is billing-blocked before runner steps) | `.scratch/phase-1-core10-concepts/`; blocked runs 35431732049, 35431943529 |
| QB-01 | Immutable published question versions | tested (slice-1 scope: versioned, published-only serving) | run 35280682748 |
| QB-02 | Question-family and variant identities | in-progress (family_id column exists; variant flows pending) | run 35280682748 |
| QB-03 | Timed / untimed / tutor practice | in-progress (tutor, timed, and revision presets all tested; untimed-with-optional-per-question timer pending) | runs 35280682748, 35372696754 |
| QB-04 | Confidence and assistance evidence separation | in-progress (confidence stored; assisted paths pending) | run 35280682748 |
| QB-05 | Per-option explanations and source anchors | tested (slice-1 scope: rationale per option + source_ref, tutor feedback) | run 35280682748 |
| QB-08 | Issue reporting and quarantined-item exclusion | in-progress (report API + 3-vote quarantine + pool exclusion + report UI, CI-green on feat/qb08-item-reports run 35405398143; resolve route honestly 501 until the Phase 2 editorial console) | .scratch/phase-1-qb08-reports/ |
| QB-11 | Two to ten options with generated labels | tested (engine rule in domain-contracts + OptionCount validation on ingestion) | run 35280682748 |
| QB-12 | Qbank builder: hierarchy multi-select, four pools, counts, availability rule, presets, Quick 10 | in-progress (chapter pool + honest empty-pool message tested; full builder is the UI slice) | run 35280682748 |
| QB-13 | Session tools baseline: calculator, converter, text size, hint, auto-submit warnings, submission summary | in-progress (deterministic calc-engine crate tested — BMI, BSA, MAP, GCS, Cockcroft-Gault, CKD-EPI 2021, anion gap, corrected calcium, native+wasm; tool-tray UI, converter, text size, hint, auto-submit pending) | runs 35366131810, 35367410576 |
| QB-14 | Key learning point, exam tip, high-yield flag, authored + empirical difficulty | tested (slice-1 scope: fields stored, served in tutor feedback; empirical rating via Elo state) | run 35280682748 |
| QB-17 | Session results with time + answer-change analysis and result actions (part 1 in P1, part 2 in P2) | in-progress (Phase 1 implemented; acceptance CI blocked before runner steps by account billing/spending-limit state; Phase 2 answer-change analytics pending) | `.scratch/phase-1-qb17-session-results/`; blocked run 35432635774 |
| EX-01 | Official-source exam registry with aliases | in-progress (registry table tested; aliases + official-source records pending) | run 35280682748 |
| EX-04 | Durable answer persistence and submission receipts | tested (slice-1 scope: idempotent replay, first-answer-wins, double-submit rejected) | run 35280682748 |
| AI-01 | Structured learner-concept state and uncertainty | tested (slice-1 scope: per-chapter Elo state + evidence counts, server-authoritative) | run 35280682748 |
| AI-02 | Cold-start plan with honest sparse-data behavior | tested (slice-1 scope: modest first plan, low_evidence level, no fake mastery under 10 attempts) | run 35280682748 |
| AI-04 | Time-budgeted next-best-action selection | in-progress (cold-start task only; capacity-constrained selection pending) | run 35280682748 |
| AI-05 | Bounded event-driven orchestration | in-progress (deterministic in-request handlers; event/queue layer arrives with workers) | run 35280682748 |
| AI-06 | Permissioned action tools | in-progress (the only agent action so far — deterministic plan revision — respects §9.1; LLM tool scopes land with the Coach) | run 35280682748 |
| AI-07 | Action receipts and undoable plan revisions | tested (slice-1 scope: revision receipt JSONB with triggering evidence + checks + diff, undo restores version) | run 35280682748 |
| AI-13 | Cost limits, fallbacks, kill switches | in-progress (no model calls yet — deterministic slice; switches land with OPS-05 remote config) | run 35280682748 |
| AI-14 | No cross-tenant private-memory access | in-progress (tenant authorization/audit isolation foundation tested; learner private-memory/storage tenantization and adversarial isolation tests remain) | runs 35280682748, 35425807184 |
| AI-17 | Transparent baseline estimator, default selection policy, difficulty fallback | in-progress (Elo estimator + shrinking K tested; window difficulty-fallback rule and selection mix pending) | run 35280682748 |
| PLAN-01 | Original / revised / current plan timeline | in-progress (versioned plans + revision list + undo tested; timeline UI pending) | run 35280682748 |
| PLAN-02 | Capacity changes and feasible replanning | in-progress (revision-on-evidence tested; capacity/deadline replanning pending) | run 35280682748 |
| LIB-01 | Versioned articles and references | not-started | — |
| NOTE-01 | Source-linked private notes | not-started | — |
| ADMIN-01 | Real cross-tenant owner dashboard | not-started | — |
| ADMIN-02 | Content and rights operations | not-started | — |
| ADMIN-03 | AI model / cost / policy administration | not-started | — |
| ADMIN-04 | Support, incidents, audit trails | not-started | — |
| TRUST-01 | No fake scores, charts, citations, active-agent states | not-started | — |
| TRUST-02 | Privacy, deletion, export workflows | not-started | — |
| TRUST-03 | Prompt-injection and tenant-isolation tests | not-started | — |
| TRUST-04 | Clinically reviewed shared medical content | not-started | — |
| UX-01 | Touch-first session workspace: gestures, tool tray, navigator, Focus Mode | in-progress (tutor flow: options, feedback, skip, letter-key+Enter navigator, honest results — browser-E2E tested; gestures, tool tray, Focus Mode pending) | run 35366131810 |
| UX-02 | Desktop and web keyboard map and fullscreen | in-progress (letter keys select, N/Enter next — fullscreen pending) | run 35366131810 |
| ENG-01 | Daily goal, streak with freezes, question of the day; all disableable | not-started | — |
| COM-01 | Upgrade triggers and free allowance inside the 7C tiers | in-progress (allowance trigger tested — originates from entitlement checks only, never the Coach; remaining triggers: offline download, full mock, chapter analytics) | run 35372696754 |
| GROW-02 | Astro site: per-exam pages, pricing, checkout, help, legal, app-link files | not-started | — |
| ARCH-02 | TypeScript contracts generated from Rust types | in-progress (hand-written pre-generation client in apps/client/src/lib/api.ts, marked for replacement; generation pipeline pending) | run 35366131810 |
| OPS-03 | Signed releases and reversible migrations | not-started | — |
| OPS-05 | Product-analytics taxonomy, experimentation, remote config, kill switches | not-started | — |

Spanning IDs starting in Phase 1: PROT-01 (capture protection + watermark, completes P3), PROT-03 (store compliance, completes P3), COM-02 (store billing + web checkout, completes P3), ADMIN-06 (editorial console baseline, P1–P2), TRUST-06 (accessibility + device matrix, P1–P3), UX-03 (client performance budgets, P1–P3).

## Phase 2 — Intelligent core

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| QB-06 | Targeted, unseen, marked, incorrect filters | not-started | — |
| QB-07 | Blueprint-balanced session generation | not-started | — |
| QB-09 | Item statistics and editorial review | not-started | — |
| QB-15 | Community statistics with minimum sample + expected-score comparison | not-started | — |
| QB-16 | Psychometric screening defaults + issue-report SLA | not-started | — |
| EX-02 | Date-effective block / timer / break configuration | not-started | — |
| EX-03 | Frozen assessment forms and versions | not-started | — |
| EX-05 | Reserved assessment-family protection | not-started | — |
| EX-06 | Accommodations and assessment-specific AI restrictions | not-started | — |
| EX-07 | Administrator-configured mock tests, types, results | not-started | — |
| EX-08 | Monotonic client timer, grace windows, integrity signals, per-test policy | in-progress (server-issued deadline, server-side answer cutoff after expiry, auto-submit semantics, skew-corrected client countdown — all tested incl. browser E2E; device-clock-tamper tests, grace windows, integrity signals pending) | run 35372696754 |
| AI-03 | Mistake hypotheses, not assumed diagnoses | not-started | — |
| AI-08 | Protected tasks and plan-churn controls | not-started | — |
| AI-09 | Source-grounded contextual tutoring | not-started | — |
| AI-10 | Socratic, explain-back, contrast modes | not-started | — |
| AI-11 | Learner-viewable editable memory | not-started | — |
| AI-12 | Delayed intervention outcome tracking | not-started | — |
| AI-16 | Qualified model routing and regression suites | not-started | — |
| AI-18 | Pre-generated one-tap tutoring, cached and offline | not-started | — |
| PLAN-03 | Review debt recovery and buffer time | not-started | — |
| SR-01 | Deterministic reviewed scheduling engine (FSRS) | tested (product scope: decks, cards, review-events API, review UI — all through the official MIT rs-fsrs implementation behind the shared scheduler crate; native+wasm; browser-E2E tested) | run 35379501317 |
| SR-02 | New-card and workload limits | tested (product scope: QueueLimits 30/10 enforced through GET /v1/reviews/queue; most-at-risk-first triage with overflow counting) | run 35379501317 |
| SR-03 | Cloze, image, explanatory cards | in-progress (front/back text cards tested end-to-end; cloze, image-occlusion, audio, and clinical-discrimination card types pending) | run 35379501317 |
| SR-04 | AI draft vs editorial trust labels | not-started | — |
| SR-05 | Duplicate / sibling handling | not-started | — |
| SR-08 | Automatic question re-test queue, objective grading, family-variant preference | not-started | — |
| SR-09 | Editorial key-point cards | not-started | — |
| LIB-02 | Hybrid search with visibility filters | not-started | — |
| LIB-03 | Page / figure / timestamp citations | not-started | — |
| LIB-04 | Guideline country / date overlays | not-started | — |
| LIB-05 | Source-change propagation | not-started | — |
| LIB-06 | Rights-checked document imports | not-started | — |
| LIB-07 | Table / image / extraction completeness reports | not-started | — |
| LIB-08 | Media player, captions, chapters | not-started | — |
| NOTE-02 | Concepts, backlinks, collections | not-started | — |
| NOTE-03 | Human-controlled revisions and portable export | not-started | — |
| IMG-01 | Rights-checked still-image case library | not-started | — |
| PROG-01 | Hierarchy drill-down analytics, difficulty + trend filters, mastery heat-map | not-started | — |
| ENG-02 | XP, achievements, weekly recap | not-started | — |
| COM-03 | Coupons, referrals, regional price tiers, trials, pause | not-started | — |
| ADMIN-05 | Revenue, licenses, royalties, renewals | not-started | — |
| ADMIN-06 | Editorial console baseline: hierarchy, questions, bulk import (dry run + rollback), settings | not-started | — |
| TRUST-07 | Provenance retained through derived resources | not-started | — |
| OPS-04 | Recovery drills and failure-mode monitoring | not-started | — |

## Phase 3 — Cross-device product

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| SR-06 | Offline reviews and synchronized history | not-started | — |
| SR-07 | Authorized import / export compatibility | not-started | — |
| OFF-01 | Signed resource manifests and resumable downloads | not-started | — |
| OFF-02 | Idempotent event reconciliation | not-started | — |
| OFF-03 | Note conflicts and versioned plan resolution | not-started | — |
| OFF-04 | Offline entitlement and freshness disclosure | not-started | — |
| LIB-09 | Licensed offline media packages | not-started | — |
| IMG-05 | Low-device-capability fallbacks | not-started | — |
| UX-03 | Client performance budgets on reference devices (completes) | not-started | — |
| TRUST-06 | Accessibility and device-matrix testing (completes) | not-started | — |
| PROT-01 | Capture protection per platform + text-surface watermark (completes) | not-started | — |
| PROT-02 | Device attestation, anti-scraping, pack encryption, offline lease | not-started | — |
| PROT-03 | Store and platform compliance checklist (completes) | not-started | — |
| COM-02 | Store billing + web checkout with local wallets; one entitlement service (completes) | not-started | — |
| OPS-06 | Staged rollout, forced / soft update, two-version API compatibility | not-started | — |
| ENG-03 | Widgets and lock-screen mock timer (COULD) | not-started | — |
| ARCH-03 | Owned native Tauri plugins in Swift and Kotlin (completes) | not-started | — |

## Phase 4 — Institutional product + community

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| INST-01 | Programs, cohorts, assignments | not-started | — |
| INST-02 | Distinct faculty and institution workspaces | not-started | — |
| INST-03 | SSO and scoped enrollment integrations | not-started | — |
| INST-04 | Curriculum mapping and coverage | not-started | — |
| INST-05 | Assessment author / reviewer / publisher separation | not-started | — |
| INST-06 | LTI and selected QTI interoperability | not-started | — |
| INST-07 | Privacy-preserving cohort analytics | not-started | — |
| COMMUNITY-01 | Moderated groups and discussions | not-started | — |
| COMMUNITY-02 | Permitted shared content and private challenges | not-started | — |
| COMMUNITY-03 | Optional gamification without coercive defaults | not-started | — |
| COMP-01 | Daily / weekly / monthly / live competitions; same-exam population; opt-in handle | not-started | — |
| COMP-02 | Scoring with guess penalty, capped speed bonus, tie-breaks | tested (engine scope: competition-scoring crate — 5/10/15 configurable points, 25% wrong penalty, 20% capped correct-only speed bonus, tie-break ladder score→accuracy→time→submission; native+wasm; live-event wiring lands with COMP-01) | run 35367410576 |
| COMP-03 | Leagues and duels | not-started | — |
| COMP-04 | Anti-cheat, integrity review, prizes only after review | not-started | — |
| GROW-01 | Share cards, duel links, deferred deep links, ambassador codes | not-started | — |
| CAREER-01 | Longitudinal learning portfolio | not-started | — |
| PLAN-05 | Optional calendar read / write scopes | not-started | — |

## Phase 5 — Clinical practice labs

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| SIM-01 | Versioned fictional / approved case scripts | not-started | — |
| SIM-02 | Separate patient and examiner contexts | not-started | — |
| SIM-03 | Voice / text with transcript uncertainty | not-started | — |
| SIM-04 | Rubric evidence per criterion | not-started | — |
| SIM-05 | Authoritative state transitions and timers | not-started | — |
| SIM-06 | Counterfactual replay and debrief timeline | not-started | — |
| SIM-07 | Human review / appeal for consequential use | not-started | — |
| IMG-02 | Stack viewer and reviewed annotations | not-started | — |
| IMG-03 | DICOM privacy and pixel-integrity workflow | not-started | — |
| IMG-04 | Anatomy / clinical-image linkage | not-started | — |
| CAREER-02 | Human-supervised feedback / sign-off | not-started | — |

## Phase 6 — Global expansion

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| AI-15 | Evaluated multilingual tutoring | not-started | — |
| PLAN-04 | Exam-switch knowledge-gap report | not-started | — |
| SIM-08 | Team-based cases and handover | not-started | — |

## Phase 7 — Advanced intelligence and continuing education

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| QB-10 | Validated advanced calibration when supported | not-started | — |
| TRUST-05 | Validated readiness before predictive claims | not-started | — |
| CAREER-03 | Continuing-education records and provider workflow | not-started | — |

## Awaiting owner approval (no phase commitment)

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| COM-04 | 'Until my exam' + pass extension proposals | blocked (owner decision §32 / §26.1) | .scratch/phase-0-decisions/issues/11 |

Ledger update rule: when a ticket completes, set the ID to `tested` and link the evidence file. Phase gates (§28) require every ID of that phase to be `tested` or explicitly `deferred` with owner sign-off.
