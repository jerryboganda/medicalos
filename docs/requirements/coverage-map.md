# Master-Plan Coverage Map

Maps every section of `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` to the phase(s) and requirement IDs that implement it. **A section with no mapping is a gap** — this file proves nothing from the plan is left behind. (Appendix C already proved nothing was lost in the v1.0→v2.0 merge; this proves nothing is lost in the build.)

| Plan section | Topic | Phase | Implementing IDs / artifacts |
|---|---|---|---|
| §1 + §1.1 | Approved owner decisions + merge defaults | 0 | ADRs 0001–0005, AGENTS.md compute policy |
| §2.1–2.3 | Connected learning system; success definition; claims discipline | cross-cutting | TRUST-01, §30 gates; optimization-target rule in code review checklist |
| §2.4 | Operational health metrics | 1 (views), 4 (product-health console) | OPS-05, ADMIN-01 |
| §3 | Competitive synthesis | — (research input) | Informs requirements; no build artifact. Marketing claims blocked by §2.3 rule |
| §4 | Gaps → requirements | 1–4 | Mapped row-by-row into §29 IDs (plan already did the mapping) |
| §5.1–5.3 | Pathways; exam-pack contract; cross-exam transfer | 1 (registry), 6 (packs, PLAN-04) | EX-01, §5.2 template fields in pack schema, PLAN-04 |
| §5.4 | Clinical-use boundary | cross-cutting | Legal gate; labels in UI copy review (P1) |
| §5.5 | Exam→Subject→System→Chapter hierarchy | 1 | CORE-10, EX-01 |
| §6, §6.1, §6.2 | IA, onboarding, core journeys | 1 | CORE-05, CORE-09, onboarding slice |
| §6.3 | Accounts and sign-in | 1 | CORE-07 |
| §6.4 | Notification policy | 1 | CORE-08 |
| §7.1–7.5 | Visual direction, signature screens, functional rules, responsive, session workspace | 0 (tokens), 1 (screens) | packages/design-system, CORE-06, UX-01, UX-02 |
| §8.1–8.8 | Personalization spec + estimator defaults | 1 (AI-01/02/04/17), 2 (AI-03/08–12) | see ledger |
| §9.1–9.5 | Orchestration, runtime flow, autonomy matrix, reliability, pre-generated tutoring | 1 (AI-05/06/07/13), 2 (AI-18) | see ledger |
| §10 | Worked personalization scenario | 1–2 | Acceptance test fixture in tests/ai-evaluation |
| §11.1–11.9 | QBank + exam engine (content model, modes, integrity, psychometrics, builder, tools, mocks, community stats, results) | 1 (11.1–11.3 core, 11.5–11.6), 2 (11.4, 11.7–11.9, integrity full) | QB-01..17, EX-01..08 |
| §12.1–12.5 | Library, search, ingestion, media, notebook | 1 (12.1 base), 2 (rest) | LIB-01..08, NOTE-01..03 |
| §13 | Spaced repetition + re-test queue + key-point cards | 2 (engine), 3 (offline) | SR-01..09 |
| §14.1–14.4 | OSCE, patient/examiner split, dynamic patient engine, simulation features | 5 | SIM-01..08 |
| §15 | Anatomy, radiology, visual learning | 2 (IMG-01), 5 (IMG-02..04) | see ledger |
| §16 | Clinical reference + CE | 1 (calc engine via QB-13), 7 (CAREER-03) | calc-engine crate, CAREER-03 |
| §17, §17.1, §17.2 | Community, competition, engagement mechanics | 1 (ENG-01), 2 (ENG-02), 3 (ENG-03), 4 (§17.1) | ENG-01..03, COMP-01..04, COMMUNITY-01..03 |
| §18.1–18.5 | Institutions, faculty, owner console, interoperability | 1 (ADMIN-01..04 baseline), 4 (INST-01..07) | see ledger |
| §19.1–19.5 | Content channels, rights ledger, editorial workflow, corrections, editorial console | 1 (rights checks, ADMIN-02), 2 (workflow + console + reports) | ADMIN-02/05/06, QB-08/16, LIB-05/06, TRUST-04/07 |
| §20.1–20.4 | Stack, deployment shape, tenant security, repository tree | 0–1 | ADR 0002, scaffold, CORE-04, §20.3 RLS tests (TRUST-03) |
| §21.1–21.3 | Entities, APIs, events | 1 (contracts), 2+ (new domains per phase) | ARCH-02, domain-contracts crate, App. B stream |
| §22 | Offline + sync spec incl. packs, conflict rules, surface differences | 1 (local-first layer), 3 (full) | OFF-01..04, SR-06/07, LIB-09 |
| §23 | AI model strategy + evaluation | 2 | AI-16, tests/ai-evaluation |
| §24, §24.1 | Readiness governance + hierarchy analytics | 2 (analytics), 7 (prediction) | PROG-01, TRUST-05, QB-10; outcomes collection from P1 (POST /v1/outcomes) |
| §25, §25.1, §25.2 | Security/privacy, content protection, store compliance | 1 (TRUST-01..04 baseline, PROT-01 start), 3 (PROT-02/03) | see ledger |
| §26, §26.1 | Monetization tiers, mechanics, growth | 1 (COM-01, entitlement), 2 (COM-03), 3 (COM-02), 7 (COM-04 if approved) | see ledger |
| §27, §27.1 | CI + compute rule, Rust/Tauri consequences | 0 | OPS-01/02/07, AGENTS.md compute policy, .github/workflows |
| §28, §28.1 | Phased build program | — (this program) | docs/requirements/* + .scratch/phase-* |
| §29 | Feature inventory | — (ledger) | docs/requirements/traceability.md |
| §30.1–30.5 | Release acceptance criteria, budgets, learning-quality tests, validation, device matrix | cross-cutting | Per-phase gate evidence; §30.3 tests in tests/; §30.5 matrix in P3 (TRUST-06) |
| §31, §31.1 | Agentic execution contract + Rust/Tauri guardrails | cross-cutting | AGENTS.md, AGENT_IMPLEMENTATION_HANDOFF.md, code-review checklist |
| §32 | Open decisions | 0 | .scratch/phase-0-decisions/issues/09–19 |
| Appendix A | Merge ledger | — (reference) | No action |
| Appendix B | Product-analytics taxonomy | 0 (doc), 1 (stream) | docs/requirements/analytics-taxonomy.md, OPS-05 |
| Appendix C | Quiz LMS v1.0 coverage | — (reference, verified complete) | No action |

Unplaced items check: source register (S01–S51) is reference only — no build artifact. `UI_REFERENCE.png` is referenced by the plan but **missing from the project** — owner item, see issue 19.
