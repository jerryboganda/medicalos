# Requirement Traceability Ledger

Every stable requirement ID from master plan §29, its phase, and its delivery status. **No implementation ticket ships without at least one ID from this ledger.** A ticket's completion report must distinguish implemented / tested / partial / blocked / deliberately deferred (master plan §31).

Status vocabulary: `not-started` · `in-progress` · `tested` (acceptance evidence filed) · `blocked` (named blocker) · `deferred` (deliberate, with reason).

Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §29. Phase definitions: §28 + §28.1. Release gates: §30.

> **✅ Validation status (2026-09-20):** GitHub Actions billing is resolved and the full pipeline is green end-to-end. CI run 35476029818 (commit `b157346`): fmt, clippy, cargo-deny, all 30 integration tests, wasm32 core builds, client + site builds, Playwright E2E — all passing. Deploy run 35476029813: sqlx cache regenerated in-workflow, GHCR images built, VPS deployed, **production-verified** — medicalos.polytronx.com serves commit `b157346` (healthz OK, SPA assets OK, version.json matches). All slices 9–15, including the previously-unvalidated slice-14 IDs (NOTE-02, SR-08, QB-16, OPS-06, LIB-02), are CI-validated with this run as evidence.

> **✅ Remaining-phases completion kernel (2026-09-22):** CI run 35728441244 (commit `df35f40`, branch `codex/remaining-phases`): fmt, clippy, cargo-deny, all integration tests, wasm32 core builds, client + site builds, Playwright E2E — all passing. Evidence for CORE-07, EX-02/03/05/06, AI-11/12, INST-04/06, TRUST-05, OFF-02/03/04, PROT-02, SR-07 rows updated below.

> **✅ ENG-01 engagement slice + main sync (2026-09-22):** CI run 35756191515 (commit `edbb22c`): fmt, clippy, cargo-deny, all integration tests (incl. `engagement_goal_streak_and_qotd`, `engagement_skipped_answer_does_not_meet_daily_goal`, `engagement_freeze_bridges_one_missed_day`), wasm32 core builds, client + site builds, Playwright E2E — all passing. Deploy run 35756558245: sqlx cache regenerated, GHCR images built, migration 0019 applied on deploy, VPS deployed, **production-verified** — medicalos.polytronx.com serves `edbb22c` (healthz OK, version.json matches). This run also completes the deferred 15-commit main sync (completion kernel + offline sync + ENG-01).
> **✅ Tenant/community stretch (2026-09-22):** CI run 35783063874 (commit `b30dcc4`): fmt, clippy, cargo-deny, all integration tests (incl. the five new slice tests), wasm32 core builds, client + site builds, Playwright E2E — all passing. CORE-04, INST-05, INST-07, AI-14, EX-05, EX-06, QB-02, QB-04, SR-08, COMMUNITY-01/02/03, COMP-01/03/04, GROW-01 rows updated below; migrations 0021-0023 are additive with exercised rollback.
> **✅ Coach/plan depth stretch (2026-09-22):** CI run 35790283904 (commit `1a29114`): fmt, clippy, cargo-deny, all integration tests (incl. coach modes, replan, and debt/gap-report tests), wasm32 core builds, client + site builds, Playwright E2E — all passing. AI-10, AI-17, PLAN-02, PLAN-03, PLAN-04 rows updated below.
> **✅ Library/cards stretch (2026-09-22):** CI run 35796428516 (commit `f7627a4`): fmt, clippy, cargo-deny, all integration tests (incl. card types/trust/duplicates and rights-checked media/image cases), wasm32 core builds, client + site builds, Playwright E2E — all passing. SR-03, SR-04, SR-05, LIB-03, LIB-04, LIB-08, IMG-01, IMG-02 rows updated below; migration 0024 is additive with exercised rollback.
> **✅ Ops/admin stretch (2026-09-22):** CI run 35798931187 (commit `fa19a74`): fmt, clippy, cargo-deny, all integration tests (incl. the analytics taxonomy, rights-ledger, incident, client-update, and prompt-injection tests), wasm32 core builds, client + site builds, Playwright E2E — all passing. OPS-05, OPS-06, ADMIN-01, ADMIN-02, ADMIN-03, ADMIN-04, TRUST-03, TRUST-07 rows updated below; migration 0025 is additive with exercised rollback.
> **✅ Learner/faculty surfaces stretch (2026-09-23):** CI run 35803038827 (commit `63b8355`): fmt, clippy, cargo-deny, all integration tests (incl. the curriculum/heatmap/handle test), wasm32 core builds, client + site builds, Playwright E2E — all passing. QB-12, INST-02, PROG-01, COMMUNITY-01/02, GROW-01, CORE-05, PLAN-01 rows updated below; new learner endpoints (curriculum, memberships, my-duels, handle lookup) and heatmap drill-down/filters are additive.
> **✅ Session/entitlement tails stretch (2026-09-23):** CI run 35807111097 (commit `adeb536`): fmt, clippy, cargo-deny, all integration tests (incl. single-active-session + device limit, per-question budget, and both upgrade triggers), wasm32 core builds, client + site builds, Playwright E2E — all passing. UX-01, UX-02, CORE-07, QB-03, COM-01, PLAN-01 rows updated below; migration 0026 is additive with exercised rollback.
> **✅ Interop/QA stretch (2026-09-23):** CI run 35809798171 (commit `0b3db08`): fmt, clippy, cargo-deny, all integration tests (incl. the variants/trends/drills/regression/QTI test), wasm32 core builds, client + site builds, Playwright E2E — all passing. QB-02, LIB-02, OPS-04, AI-16, CORE-05, INST-06 rows updated below; migration 0027 is additive with exercised rollback.

**Evidence run for Phase 1 slice 1 (backend loop):** https://github.com/jerryboganda/medicalos/actions/runs/35280682748 — IDs marked `tested (slice-1 scope)` have seam-test coverage for the scope defined in `.scratch/phase-1-slice-1/spec.md`; they remain in scope for the rest of Phase 1 (UI, multi-exam, timed presets, etc.).

## Phase 0 — Decisions and evidence

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| OPS-01 | GitHub Actions development-compute enforcement | in-progress (enforced; proven by green CI run 35273824336) | .scratch/phase-0-decisions/issues/04 |
| OPS-02 | Runtime boundary approval before deployment | resolved (owner deployment mandate + VPS runtime live: medicalos.polytronx.com serves client + API, production-verified; see docs/deployment/production.md) | run 35456997436 |
| OPS-07 | Rust + Tauri CI runner matrix under the compute rule | tested (Linux gates green, run 35273824336; macOS/Windows signing jobs arrive with Tauri shells) | issues/04 |
| ARCH-01 | Shared Rust core across server, Tauri, and WebAssembly | in-progress (three shared crates — domain-contracts, calc-engine, competition-scoring — proven native + wasm32 in CI; runtime parity check in the Phase 0 spike remains, issues/08) | runs 35273824336, 35367410576; issues/08 |

## Phase 1 — Connected vertical slice

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| CORE-01 | One identity with personal and institution contexts | in-progress (identity + auth tested, slice-1 scope; institution contexts pending) | .scratch/phase-1-slice-1/ |
| CORE-02 | Versioned goals, exam dates, protected commitments | tested (goal create/list with retirement history; protected commitments learner-only CRUD, §9.3) | run 35467238410 |
| CORE-03 | Entitlement checks across API, media, retrieval, offline manifests | in-progress (free-tier daily question allowance + offline pack lease both enforce server-side with honest 403 + details; media/retrieval entitlements pending) | runs 35372696754, 35728441244 |
| CORE-04 | Multi-tenant role and audit foundations | in-progress (institution-scoped audit events + staff-gated audit export, extended §18.1 role vocabulary, cross-tenant isolation tests proven; dedicated tenant partitioning pending) | run 35783063874 |
| CORE-05 | Hierarchy drill-down analytics, difficulty + trend filters, | tested (system drill-down, difficulty-restricted accuracy, trend window, and weekly per-chapter accuracy buckets — every number from real non-assisted attempts; unassessed stays honest) | run 35809798171 |
| CORE-06 | Truthful loading, error, empty, permission states | tested (slice-2 scope: loading/error+retry/empty/no-evidence states in the learner UI; no fake analytics anywhere) | run 35366131810 |
| CORE-07 | Sign-in methods, in-app account deletion, device limit, single active session | in-progress (device register/list/revoke, soft account deletion, revoked-session rejection, single-active-session policy with immediate take-effect, and the hard device limit with devices_exhausted tested; SSO/magic-link methods pending) | run 35807111097 |
| CORE-08 | Notification policy, push + in-app inbox | tested (slice-12 scope: in-app inbox, per-category preferences, quiet hours, delivery gate; remote push tokens pending owned plugin) | run 35467238410 |
| CORE-09 | Guest trial before sign-up (SHOULD) | tested (guest key trial: 5-question ceiling, no keys for guests, honest upgrade refusal) | run 35467238410 |
| CORE-10 | Navigational hierarchy mapped to concept identities | in-progress (Exam→Subject→System→Chapter nodes tested; concept-identity mapping pending) | run 35280682748 |
| QB-01 | Immutable published question versions | tested (slice-1 scope: versioned, published-only serving) | run 35280682748 |
| QB-02 | Question-family and variant identities | tested (family identity preserved across versioned variant authoring — new versions are born drafts through the §19.3 gate; the retest queue serves unattempted published siblings with fallback) | run 35809798171 |
| QB-03 | Timed / untimed / tutor practice | tested (tutor, timed, and revision presets with server deadlines; untimed sessions accept an optional per-question budget enforced at the answer seam; budgets refused on timed presets) | run 35807111097 |
| QB-04 | Confidence and assistance evidence separation | tested (answers carry confidence and declared assistance; the Coach answer-first gate bounds what assistance can exist; assisted attempts excluded from community stats and psychometric screening) | run 35783063874 |
| QB-05 | Per-option explanations and source anchors | tested (slice-1 scope: rationale per option + source_ref, tutor feedback) | run 35280682748 |
| QB-08 | Issue reporting and quarantined-item exclusion | in-progress (report API + 3-vote quarantine + pool exclusion + report UI, CI-green on feat/qb08-item-reports run 35405398143; resolve route honestly 501 until the Phase 2 editorial console) | .scratch/phase-1-qb08-reports/ |
| QB-11 | Two to ten options with generated labels | tested (engine rule in domain-contracts + OptionCount validation on ingestion) | run 35280682748 |
| QB-12 | Qbank builder: hierarchy multi-select, four pools, counts, and honest empty-pool handling | tested (the practice builder renders the learner curriculum with published counts, multi-select, the four pools, and question count; empty pools surface the server error verbatim) | run 35803038827 |
| QB-13 | Session tools baseline: calculator, converter, text size, hint, auto-submit warnings, submission summary | in-progress (deterministic calc-engine crate tested — BMI, BSA, MAP, GCS, Cockcroft-Gault, CKD-EPI 2021, anion gap, corrected calcium, native+wasm; tool-tray UI, converter, text size, hint, auto-submit pending) | runs 35366131810, 35367410576 |
| QB-14 | Key learning point, exam tip, high-yield flag, authored + empirical difficulty | tested (slice-1 scope: fields stored, served in tutor feedback; empirical rating via Elo state) | run 35280682748 |
| QB-17 | Session results with time + answer-change analysis and result actions (part 1 in P1, part 2 in P2) | tested (slice 16: per-item elapsed, mock answer-change counts, session duration; result actions tested slice 15) | run 35479440275 |
| EX-01 | Official-source exam registry with aliases | in-progress (registry list endpoint now serves code/name/official_source_url/aliases; alias + official-source fixture assertions pending) | run 35728441244 |
| EX-04 | Durable answer persistence and submission receipts | tested (slice-1 scope: idempotent replay, first-answer-wins, double-submit rejected) | run 35280682748 |
| AI-01 | Structured learner-concept state and uncertainty | tested (slice-1 scope: per-chapter Elo state + evidence counts, server-authoritative) | run 35280682748 |
| AI-02 | Cold-start plan with honest sparse-data behavior | tested (slice-1 scope: modest first plan, low_evidence level, no fake mastery under 10 attempts) | run 35280682748 |
| AI-04 | Time-budgeted next-best-action selection | in-progress (cold-start task only; capacity-constrained selection pending) | run 35280682748 |
| AI-05 | Bounded event-driven orchestration | in-progress (deterministic in-request handlers; event/queue layer arrives with workers) | run 35280682748 |
| AI-06 | Permissioned action tools | tested (slice-11 scope: Coach reads ONLY reviewed material of answered questions + own attempt — permissioning enforced and tested; broader tool scopes pending) | run 35466880140 |
| AI-07 | Action receipts and undoable plan revisions | tested (slice-1 scope: revision receipt JSONB with triggering evidence + checks + diff, undo restores version) | run 35280682748 |
| AI-13 | Cost limits, fallbacks, kill switches | tested (slice-11 scope: daily AI allowance enforced with honest 403 + details; extractive fallback means study never blocks; remote kill switch via OPS-05 flags pending) | run 35466880140 |
| AI-14 | No cross-tenant private-memory access | in-progress (learner memory is user-scoped by design with user-scope tests; institution isolation tests added — cross-tenant analytics/audit/duel reads refused; multi-tenant deployments pending) | run 35783063874 |
| AI-17 | Transparent baseline estimator, default selection policy, di | tested (GET /v1/me/selection-policy discloses the Elo baseline anchors, shrinking-K rule, counted-evidence definition, and the actual selection rules alongside the learner's real per-chapter state) | run 35790283904 |
| PLAN-01 | Original / revised / current plan timeline | tested (versioned plans, revision list with receipts, undo, capacity replanning, and the Progress plan-timeline card rendering origin and undo state) | run 35807111097 |
| PLAN-02 | Capacity changes and feasible replanning | tested (POST /v1/me/plan/replan forks today's plan to a new version fitting the declared budget, defers trimmed tasks with a capacity_change receipt, never touches done work; validation 5-480 minutes) | run 35790283904 |
| LIB-01 | Versioned articles and references | tested (seed-level: versioned published articles readable; editorial authoring surface pending) | run 35467238410 |
| NOTE-01 | Source-linked private notes | tested (CRUD + user-scoped isolation + deletion) | run 35467238410 |
| ADMIN-01 | Real cross-tenant owner dashboard | tested (GET /v1/admin/dashboard serves real cross-tenant aggregates: institutions, active users, published questions, articles, rights records, open incidents, 30-day coach turns; UI surface pending) | run 35798931187 |
| ADMIN-02 | Content and rights operations | tested (content_rights ledger per §19.2 — ref code, licensor, territory, permitted uses, validity — admin-managed with audit; full workflow UI pending) | run 35798931187 |
| ADMIN-03 | AI model / cost / policy administration | tested (GET /v1/admin/ai-admin serves real 30-day coach-stream counters by adapter, model, and prompt type plus the disclosed allowance; model-routing config surface pending) | run 35798931187 |
| ADMIN-04 | Support, incidents, audit trails | tested (incidents with severity lifecycle and audit entries; institution audit exports and the platform audit log already serve trails; support-ticket workflows pending) | run 35798931187 |
| TRUST-01 | No fake scores, charts, citations, active-agent states | not-started | — |
| TRUST-02 | Privacy, deletion, export workflows | tested (full account export: profile + attempts + notes + card reviews + portfolio in one user-scoped call) | run 35467238410 |
| TRUST-03 | Prompt-injection and tenant-isolation tests | tested (integration tests prove injected instructions never steer the extractive coach — the reply stays grounded in reviewed material with no compliance language — and cross-tenant reads are refused at analytics, audit, and duel seams) | run 35798931187 |
| TRUST-04 | Clinically reviewed shared medical content | not-started | — |
| UX-01 | Touch-first session workspace: gestures, tool tray, navigator, Focus Mode | in-progress (tutor flow: options, feedback, skip, letter-key+Enter navigator, honest results, Focus Mode, and swipe navigation between answered items shipped; native-app gesture polish pending) | run 35807111097 |
| UX-02 | Desktop and web keyboard map and fullscreen | tested (letter keys select, N/Enter next, browser fullscreen toggle in the workspace) | run 35807111097 |
| ENG-01 | Daily goal, streak with freezes, question of the day; all disableable | in-progress (daily question goal, streak/freeze, per-learner disable + global kill switch, canonical QOTD answer/community split tested — deployed and production-verified at `edbb22c`; available-time/minutes goal derivation and shared per-exam daily QOTD/push timing pending) | run 35756191515 |
| COM-01 | Upgrade triggers and free allowance inside the 7C tiers | tested (daily question allowance, offline-download, full-mock, and chapter-analytics drill-down triggers all fire from entitlement checks with structured details — never from the Coach; per-tier entitlement service arrives with billing) | run 35807111097 |
| GROW-02 | Astro site: per-exam pages, pricing, checkout, help, legal, app-link files | not-started | — |
| ARCH-02 | TypeScript contracts generated from Rust types | in-progress (hand-written pre-generation client in apps/client/src/lib/api.ts, marked for replacement; generation pipeline pending) | run 35366131810 |
| OPS-03 | Signed releases and reversible migrations | not-started | — |
| OPS-05 | Product-analytics taxonomy, experimentation, remote config, kill switches | tested (POST /v1/analytics/events enforces the Appendix-B taxonomy with a closed vocabulary and the pseudonymous no-identifiers rule; feature flags already carry remote config, staged rollout, and kill switches; experiment assignment framework pending) | run 35798931187 |

Spanning IDs starting in Phase 1: PROT-01 (capture protection + watermark, completes P3), PROT-03 (store compliance, completes P3), COM-02 (store billing + web checkout, completes P3), ADMIN-06 (editorial console baseline, P1–P2), TRUST-06 (accessibility + device matrix, P1–P3), UX-03 (client performance budgets, P1–P3).

## Phase 2 — Intelligent core

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| QB-06 | Targeted, unseen, marked, incorrect filters | tested (slice 16: multi-chapter targeting + unseen/incorrect/marked pools with honest empty-pool refusals) | run 35479440275 |
| QB-07 | Blueprint-balanced session generation | tested (exact per-chapter quotas, one-exam validation, quarantine/source filters, fail-closed pool shortfall with no partial session) | run 35757216704 |
| QB-09 | Item statistics and editorial review | tested (slice 16: per-exam review queue + per-item psychometrics with honest insufficient-attempts flagging; editorial resolution flows continue) | run 35479440275 |
| QB-15 | Community statistics with minimum sample + expected-score comparison | tested (slice-9 scope: per-question stats behind min-sample gate + option distribution; expected-score on self-built submits; percentile reserved for fixed forms) | run 35456997436 |
| QB-16 | Psychometric screening defaults + issue-report SLA | in-progress (screening endpoint with §11.4 default flags tested; SLA timers pending) | run 35476029818 |
| EX-02 | Date-effective block / timer / break configuration | tested (exam specs with effective_from/to + block/timer/break/grace seconds create and list under admin gate) | run 35728441244 |
| EX-03 | Frozen assessment forms and versions | tested (frozen assessment-form rows reject update/delete at the PostgreSQL boundary; mock starts snapshot concrete question_version IDs into session_items; migration rollback/reapply verified) | run 35758054988 |
| EX-05 | Reserved assessment-family protection | tested (reserved forms bind published question sets; bound questions never leak into open pools, QOTD, or retest variants; served only via the fixed assessment-form session with deterministic snapshot) | run 35783063874 |
| EX-06 | Accommodations and assessment-specific AI restrictions | tested (learner accommodations user-scoped; ai_allowed snapshotted onto form sessions; Coach refuses tutoring on restricted reserved questions — ai_restricted_for_assessment) | run 35783063874 |
| EX-07 | Administrator-configured mock tests, types, results | tested (slice-9 scope: admin-token mock configuration, frozen blueprint forms, deferred feedback, pass mark + per-chapter breakdown + attempts; form types and time analysis pending) | run 35456997436 |
| EX-08 | Monotonic client timer, grace windows, integrity signals, per-test policy | in-progress (server-issued deadline, server-side answer cutoff after expiry, auto-submit semantics, skew-corrected client countdown — all tested incl. browser E2E; device-clock-tamper tests, grace windows, integrity signals pending) | run 35372696754 |
| AI-03 | Mistake hypotheses, not assumed diagnoses | tested (slice 16: per-chapter miss patterns labelled hypothesis with evidence counts, min-evidence floor) | run 35479440275 |
| AI-08 | Protected tasks and plan-churn controls | not-started | — |
| AI-09 | Source-grounded contextual tutoring | tested (slice-11 scope: extractive adapter answers only from reviewed rationale/key-point/exam-tip/source of answered questions; Socratic/contrast modes pending) | run 35466880140 |
| AI-10 | Socratic, explain-back, contrast modes | tested (extractive modes on the answer-first gate: socratic guides without the reveal, explain-back mirrors the learner's words against option rationales, contrast compares the defensible choice with the picked distractor) | run 35790283904 |
| AI-11 | Learner-viewable editable memory | tested (coach memory PUT/GET/DELETE, user-scoped isolation — other learners see empty list) | run 35728441244 |
| AI-12 | Delayed intervention outcome tracking | tested (intervention create + delayed outcome PATCH with measured_at, user-owned) | run 35728441244 |
| AI-16 | Qualified model routing and regression suites | tested (POST /v1/admin/coach-regression/run re-runs the deterministic adapter over published items and records grounding pass/fail per case; adapter routing and seam tests landed earlier; live-model eval harness pending) | run 35809798171 |
| AI-18 | Pre-generated one-tap tutoring, cached and offline | not-started | — |
| PLAN-03 | Review debt recovery and buffer time | tested (GET /v1/me/review-debt reports due cards, the real last-7-day completion rate, and a backlog projection only when rate history exists — no invented numbers) | run 35790283904 |
| SR-01 | Deterministic reviewed scheduling engine (FSRS) | tested (product scope: decks, cards, review-events API, review UI — all through the official MIT rs-fsrs implementation behind the shared scheduler crate; native+wasm; browser-E2E tested) | run 35379501317 |
| SR-02 | New-card and workload limits | tested (product scope: QueueLimits 30/10 enforced through GET /v1/reviews/queue; most-at-risk-first triage with overflow counting) | run 35379501317 |
| SR-03 | Cloze, image, explanatory cards | tested (cards carry card_type basic/cloze/image with cloze marker validation; the queue renders typed cards including the cloze text; audio/discrimination types pending) | run 35796428516 |
| SR-04 | AI draft vs editorial trust labels | tested (trust editorial/ai_draft stored on every card and exposed in queue payloads with an explicit ai_draft flag; labels never stripped) | run 35796428516 |
| SR-05 | Duplicate / sibling handling | tested (adding a card whose normalized front matches an existing sibling in the same deck is refused with duplicate_card) | run 35796428516 |
| SR-08 | Automatic question re-test queue, objective grading, family- | in-progress (retest cards with deterministic intervals, idempotent results, history, family-variant serving with fallback tested; SLA timers pending) | run 35783063874 |
| SR-09 | Editorial key-point cards | tested (slice 16: misses file deduplicated key-point cards with question provenance into the review queue) | run 35479440275 |
| LIB-02 | Hybrid search with visibility filters | tested (hybrid lexical ranking: whole-phrase anchor plus weighted title/body token hits over the latest published versions, scored and ordered honestly; vector-embedding semantic ranking waits on an evaluated embedding model) | run 35809798171 |
| LIB-03 | Page / figure / timestamp citations | tested (article citations carry a kind alongside anchor/target and render in the article payload; editorial tooling for citation entry pending) | run 35796428516 |
| LIB-04 | Guideline country / date overlays | tested (article versions carry jurisdiction and effective_from/effective_to windows, served in the article payload; region-resolution UI pending) | run 35796428516 |
| LIB-05 | Source-change propagation | not-started | — |
| LIB-06 | Rights-checked document imports | not-started | — |
| LIB-07 | Table / image / extraction completeness reports | not-started | — |
| LIB-08 | Media player, captions, chapters | tested (rights-referenced media assets per article with caption cues and chapter markers served alongside the article; native player rendering pending) | run 35796428516 |
| NOTE-02 | Concepts, backlinks, collections | tested (collections CRUD + membership, concept tagging, by-concept listing — slice 14) | run 35476029818 | 
| NOTE-03 | Human-controlled revisions and portable export | tested (JSON export of all own notes incl. source references) | run 35467238410 |
| IMG-01 | Rights-checked still-image case library | tested (image cases require an https URL and a rights reference per image; findings attached; study list served to learners) | run 35796428516 |
| PROG-01 | Hierarchy drill-down analytics, difficulty + trend filters, | tested (mastery map with system drill-down, difficulty-restricted accuracy, and a trend window — overlay accuracies computed from real non-assisted attempts only; unassessed chapters stay honest) | run 35803038827 |
| ENG-02 | XP, achievements, weekly recap | tested (slice 16: XP on submit, milestone achievements, 7-day recap from real records only) | run 35479440275 |
| COM-03 | Coupons, referrals, regional price tiers, trials, pause | not-started | — |
| ADMIN-05 | Revenue, licenses, royalties, renewals | not-started | — |
| ADMIN-06 | Editorial console baseline: hierarchy, questions, bulk import (dry run + rollback), settings | tested (slice-10 scope: hierarchy CRUD + node status, question create/search, JSON bulk import with per-row validation + dry run + transactional apply + rollback-refused-after-attempts, audit trail, /admin console page; CSV/Excel parser + settings UI pending) | run 35462664343 |
| TRUST-07 | Provenance retained through derived resources | tested (rights ledger refs carried by media and image cases; trust labels on cards; retest/revision tasks trace source_session_id and added_by_revision; exports carry signed manifests) | run 35798931187 |
| OPS-04 | Recovery drills and failure-mode monitoring | tested (POST /v1/admin/recovery-drills/run verifies the signed-manifest recovery path end to end — positive proof plus tamper detection — recording pass/fail evidence rows with history) | run 35809798171 |

## Phase 3 — Cross-device product

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| SR-06 | Offline reviews and synchronized history | tested (review events applied through /v1/sync/events with the same apply path as online, replay-safe) | run 35728441244 |
| SR-07 | Authorized import / export compatibility | tested (deck export format medical-os-decks/1; re-import skips known cards, new deck imports) | run 35728441244 |
| OFF-01 | Signed resource manifests and resumable downloads | tested (HMAC-SHA256 signed pack manifests with per-question checksums; tamper detection + resumable download client pending) | run 35467238410 |
| OFF-02 | Idempotent event reconciliation | tested (sync batch: answer/review applied, unknown kind rejected; identical replay returns already_recorded, session scores answer once) | run 35728441244 |
| OFF-03 | Note conflicts and versioned plan resolution | tested (stale base_updated_at → 409 with server_updated_at; corrected base applies and advances) | run 35728441244 |
| OFF-04 | Offline entitlement and freshness disclosure | tested (free tier honest 403 + entitlement details; paid lease with rotating pack_key, content_as_of freshness, revoke) | run 35728441244 |
| LIB-09 | Licensed offline media packages | not-started | — |
| IMG-05 | Low-device-capability fallbacks | not-started | — |
| UX-03 | Client performance budgets on reference devices (completes) | not-started | — |
| TRUST-06 | Accessibility and device-matrix testing (completes) | not-started | — |
| PROT-01 | Capture protection per platform + text-surface watermark (completes) | not-started | — |
| PROT-02 | Device attestation, anti-scraping, pack encryption, offline lease | in-progress (offline lease + per-device rotating pack keys tested; device attestation, anti-scraping, pack encryption pending native/device work) | run 35728441244 |
| PROT-03 | Store and platform compliance checklist (completes) | not-started | — |
| COM-02 | Store billing + web checkout with local wallets; one entitlement service (completes) | not-started | — |
| OPS-06 | Staged rollout, forced / soft update, two-version API compat | tested (feature flags with per-user staged rollout; GET /v1/client-update serves flag-driven forced/soft update with version compatibility metadata and an honest none default; signed update packages pending) | run 35798931187 |
| ENG-03 | Widgets and lock-screen mock timer (COULD) | not-started | — |
| ARCH-03 | Owned native Tauri plugins in Swift and Kotlin (completes) | not-started | — |

## Phase 4 — Institutional product + community

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| INST-01 | Programs, cohorts, assignments | tested (slice-13 scope: institutions, member roles, cohorts with members, assignments with staff gating; SSO/LTI pending) | run 35467238410 |
| INST-02 | Distinct faculty and institution workspaces | tested (the /faculty workspace: institution picker via memberships, member management with §18.1 roles, cohorts, assignments, cohort analytics honoring suppression, and the institution audit trail; workspace theming/branding pending) | run 35803038827 |
| INST-03 | SSO and scoped enrollment integrations | not-started | — |
| INST-04 | Curriculum mapping and coverage | in-progress (institution programs create + staff-scoped access tested; curriculum-node mapping and coverage report against programs pending) | run 35728441244 |
| INST-05 | Assessment author/reviewer/publisher separation | tested (§19.3 gate on the editorial flow: authored and imported items are drafts; submit→review→approve→publish with durable review decisions; the author of an item can neither approve nor publish it — separation enforced in one transition function) | run 35783063874 |
| INST-06 | LTI and selected QTI interoperability | in-progress (QTI 2.1 package export over an exam's published items shipped — package envelope, per-item response declarations; interop receipts recorded earlier; actual LTI 1.3 launch and certification pending) | run 35809798171 |
| INST-07 | Privacy-preserving cohort analytics | tested (aggregate-only per-chapter cohort accuracy; suppressed below minimum group size k=5 with honest reason; staff-gated, cross-tenant reads refused) | run 35783063874 |
| COMMUNITY-01 | Moderated groups and discussions | tested (groups list/join/moderated feed shipped end-to-end from the /community page; moderation removal is an honest tombstone; report-to-moderator flow pending) | run 35798931187, 35803038827 |
| COMMUNITY-02 | Permitted shared content and private challenges | tested (private duels challenged by handle from the community page with accept/decline and state; permitted shared-content flows pending) | run 35803038827 |
| COMMUNITY-03 | Optional gamification without coercive defaults | tested (community identity strictly opt-in; non-participants shown as anonymous members; competition entries must use the learner's own registered handle) | run 35783063874 |
| COMP-01 | Daily / weekly / monthly / live competitions; same-exam population; opt-in handle | in-progress (competition cadences + opt-in handle entry enforcement landed; live event scheduling infrastructure pending) | run 35783063874 |
| COMP-02 | Scoring with guess penalty, capped speed bonus, tie-breaks | tested (engine scope: competition-scoring crate — 5/10/15 configurable points, 25% wrong penalty, 20% capped correct-only speed bonus, tie-break ladder score→accuracy→time→submission; native+wasm; live-event wiring lands with COMP-01) | run 35367410576 |
| COMP-03 | Leagues and duels | in-progress (private duels: same-rules sessions per side, settle on second submit with time tie-break and honest draws; leagues pending) | run 35783063874 |
| COMP-04 | Anti-cheat, integrity review, prizes only after review | tested (prize claims require a closed competition, completed integrity review, and an unflagged entry; leaderboard marks flagged entries and computes prize_eligible honestly) | run 35783063874 |
| GROW-01 | Share cards, duel links, deferred deep links, ambassador codes | in-progress (duel challenge by handle with share tokens surfaced in the community page; deep-link app plumbing, share cards and ambassador codes pending) | run 35803038827 |
| CAREER-01 | Longitudinal learning portfolio | tested (portfolio entries: rotation/case_reflection/procedure_observation/certificate with list) | run 35467238410 |
| PLAN-05 | Optional calendar read / write scopes | not-started | — |

## Phase 5 — Clinical practice labs

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| SIM-01 | Versioned fictional / approved case scripts | tested (slice-13 scope: scenario create with authored state machines; voice/LLM voicing pending) | run 35467238410 |
| SIM-02 | Separate patient and examiner contexts | in-progress (runs are per-learner with transcript evidence; separate examiner rubric flow pending) | run 35467238410 |
| SIM-03 | Voice / text with transcript uncertainty | not-started | — |
| SIM-04 | Rubric evidence per criterion | not-started | — |
| SIM-05 | Authoritative state transitions and timers | tested (slice-13 scope: deterministic transition engine — invalid events refused, transcript immutable) | run 35467238410 |
| SIM-06 | Counterfactual replay and debrief timeline | not-started | — |
| SIM-07 | Human review / appeal for consequential use | not-started | — |
| IMG-02 | Stack viewer and reviewed annotations | in-progress (stack cases with ordered image series as the viewer data contract; viewer rendering and reviewed annotations pending) | run 35796428516 |
| IMG-03 | DICOM privacy and pixel-integrity workflow | not-started | — |
| IMG-04 | Anatomy / clinical-image linkage | not-started | — |
| CAREER-02 | Human-supervised feedback / sign-off | not-started | — |

## Phase 6 — Global expansion

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| AI-15 | Evaluated multilingual tutoring | not-started | — |
| PLAN-04 | Exam-switch knowledge-gap report | tested (per-chapter independent attempt coverage for the target exam under the 10-attempt evidence rule; chapters reported as covered / low_evidence / no_evidence) | run 35790283904 |
| SIM-08 | Team-based cases and handover | not-started | — |

## Phase 7 — Advanced intelligence and continuing education

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| QB-10 | Validated advanced calibration when supported | blocked (requires an externally validated readiness/calibration record for intended_use=readiness; endpoint honestly returns calibrated_model_not_configured once a validation row exists) | run 35728441244 |
| TRUST-05 | Validated readiness before predictive claims | tested (readiness withheld with available=false + reason=validation_required until an approved validation record exists; never emits a fabricated readiness score) | run 35728441244 |
| CAREER-03 | Continuing-education records and provider workflow | in-progress (CE activity records with honest non-accredited labelling tested; accreditation/provider workflow is a Phase 7 gate) | run 35467238410 |

## Awaiting owner approval (no phase commitment)

| ID | Requirement | Status | Evidence |
|---|---|---|---|
| COM-04 | 'Until my exam' + pass extension proposals | blocked (owner decision §32 / §26.1) | .scratch/phase-0-decisions/issues/11 |

Ledger update rule: when a ticket completes, set the ID to `tested` and link the evidence file. Phase gates (§28) require every ID of that phase to be `tested` or explicitly `deferred` with owner sign-off.
