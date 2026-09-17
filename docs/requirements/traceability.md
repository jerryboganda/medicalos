# Requirement Traceability Ledger

Every stable requirement ID from master plan §29, its phase, and its delivery status. **No implementation ticket ships without at least one ID from this ledger.** A ticket's completion report must distinguish implemented / tested / partial / blocked / deliberately deferred (master plan §31).

Status vocabulary: `not-started` · `in-progress` · `tested` (acceptance evidence filed) · `blocked` (named blocker) · `deferred` (deliberate, with reason).

Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §29. Phase definitions: §28 + §28.1. Release gates: §30.

## Phase 0 — Decisions and evidence

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| OPS-01 | GitHub Actions development-compute enforcement | in-progress (enforced; proven by green CI run 35273824336) | .scratch/phase-0-decisions/issues/04 |
| OPS-02 | Runtime boundary approval before deployment | blocked (owner decision) | issues/10 |
| OPS-07 | Rust + Tauri CI runner matrix under the compute rule | tested (Linux gates green, run 35273824336; macOS/Windows signing jobs arrive with Tauri shells) | issues/04 |
| ARCH-01 | Shared Rust core across server, Tauri, and WebAssembly | in-progress (native + wasm32 compile proven in CI run 35273824336; runtime parity check remains in the Phase 0 spike, issues/08) | issues/03, issues/08 |

## Phase 1 — Connected vertical slice

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| CORE-01 | One identity with personal and institution contexts | not-started | — |
| CORE-02 | Versioned goals, exam dates, protected commitments | not-started | — |
| CORE-03 | Entitlement checks across API, media, retrieval, offline manifests | not-started | — |
| CORE-04 | Multi-tenant role and audit foundations | not-started | — |
| CORE-05 | Five-destination learner navigation | not-started | — |
| CORE-06 | Truthful loading, error, empty, permission states | not-started | — |
| CORE-07 | Sign-in methods, in-app account deletion, device limit, single active session | not-started | — |
| CORE-08 | Notification policy, push + in-app inbox | not-started | — |
| CORE-09 | Guest trial before sign-up (SHOULD) | not-started | — |
| CORE-10 | Navigational hierarchy mapped to concept identities | not-started | — |
| QB-01 | Immutable published question versions | not-started | — |
| QB-02 | Question-family and variant identities | not-started | — |
| QB-03 | Timed / untimed / tutor practice | not-started | — |
| QB-04 | Confidence and assistance evidence separation | not-started | — |
| QB-05 | Per-option explanations and source anchors | not-started | — |
| QB-08 | Issue reporting and quarantined-item exclusion | not-started | — |
| QB-11 | Two to ten options with generated labels | not-started | — |
| QB-12 | Qbank builder: hierarchy multi-select, four pools, counts, availability rule, presets, Quick 10 | not-started | — |
| QB-13 | Session tools baseline: calculator, converter, text size, hint, auto-submit warnings, submission summary | not-started | — |
| QB-14 | Key learning point, exam tip, high-yield flag, authored + empirical difficulty | not-started | — |
| QB-17 | Session results with time + answer-change analysis and result actions (part 1 in P1, part 2 in P2) | not-started | — |
| EX-01 | Official-source exam registry with aliases | not-started | — |
| EX-04 | Durable answer persistence and submission receipts | not-started | — |
| AI-01 | Structured learner-concept state and uncertainty | not-started | — |
| AI-02 | Cold-start plan with honest sparse-data behavior | not-started | — |
| AI-04 | Time-budgeted next-best-action selection | not-started | — |
| AI-05 | Bounded event-driven orchestration | not-started | — |
| AI-06 | Permissioned action tools | not-started | — |
| AI-07 | Action receipts and undoable plan revisions | not-started | — |
| AI-13 | Cost limits, fallbacks, kill switches | not-started | — |
| AI-14 | No cross-tenant private-memory access | not-started | — |
| AI-17 | Transparent baseline estimator, default selection policy, difficulty fallback | not-started | — |
| PLAN-01 | Original / revised / current plan timeline | not-started | — |
| PLAN-02 | Capacity changes and feasible replanning | not-started | — |
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
| UX-01 | Touch-first session workspace: gestures, tool tray, navigator, Focus Mode | not-started | — |
| UX-02 | Desktop and web keyboard map and fullscreen | not-started | — |
| ENG-01 | Daily goal, streak with freezes, question of the day; all disableable | not-started | — |
| COM-01 | Upgrade triggers and free allowance inside the 7C tiers | not-started | — |
| GROW-02 | Astro site: per-exam pages, pricing, checkout, help, legal, app-link files | not-started | — |
| ARCH-02 | TypeScript contracts generated from Rust types | not-started | — |
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
| EX-08 | Monotonic client timer, grace windows, integrity signals, per-test policy | not-started | — |
| AI-03 | Mistake hypotheses, not assumed diagnoses | not-started | — |
| AI-08 | Protected tasks and plan-churn controls | not-started | — |
| AI-09 | Source-grounded contextual tutoring | not-started | — |
| AI-10 | Socratic, explain-back, contrast modes | not-started | — |
| AI-11 | Learner-viewable editable memory | not-started | — |
| AI-12 | Delayed intervention outcome tracking | not-started | — |
| AI-16 | Qualified model routing and regression suites | not-started | — |
| AI-18 | Pre-generated one-tap tutoring, cached and offline | not-started | — |
| PLAN-03 | Review debt recovery and buffer time | not-started | — |
| SR-01 | Deterministic reviewed scheduling engine (FSRS) | not-started | — |
| SR-02 | New-card and workload limits | not-started | — |
| SR-03 | Cloze, image, explanatory cards | not-started | — |
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
| COMP-02 | Scoring with guess penalty, capped speed bonus, tie-breaks | not-started | — |
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
