# Medical Learning OS - Master Product and Engineering Plan

**Version:** 2.0 (merged)  
**Research checked:** 17 September 2026  
**Prepared for:** Dr Ahmed Hesham  
**Status:** Product architecture and implementation specification, not a claim of implemented functionality.  
**Supersedes:** Medical Learning OS Master Plan v1.0 and the Quiz LMS v2.0 Mobile-First Specification, now one combined platform. Quiz LMS v1.0 (169 sections) remains the functional baseline for the question-practice rules cited here.  
**Working name:** Medical Learning OS. A final brand and trademark review are still required.

> One account. One connected learning record. One deeply personalized Study Agent. A career-long medical education platform, delivered through a clean, consistent web, mobile and desktop experience.

## How to use this document

This is the canonical planning baseline for product, design, medical editorial, engineering and agentic development teams. Requirements are proposed product requirements unless identified as approved owner decisions or externally sourced facts. Source identifiers such as [S01] resolve in the source register at the end.

The requested visual reference is included as `UI_REFERENCE.png`. It governs the dark-purple aesthetic and the Home -> Coach -> Adaptive Plan interaction language, not the fitness content or tiny screenshot text. No application screens have been implemented by this planning exercise.

The plan assumes a new product with reusable integration boundaries. It does not assume permission to modify the existing OET platform, reuse its private content, or deploy anything into its production environment.

**Reading the version 2.0 merge.** Version 1.0 text is preserved verbatim except where an owner decision of 17 September 2026 replaced it (the technology stack in section 20). Headings marked **[NEW]** were added from the Quiz LMS specification or from the new stack decisions. Headings or paragraphs marked **[AMENDED]** contain owner-directed changes. Table rows ending in **[NEW]** were appended. Every change is listed in Appendix A (merge ledger), which also lists what was deliberately not carried over and why. Appendix B holds the product-analytics taxonomy and Appendix C maps Quiz LMS v1.0 to this plan. Version 1.0 requirement IDs are unchanged; new IDs continue a series or use a new prefix.

---

## 1. Approved owner decisions

| Decision | Approved scope |
|---|---|
| 1C | Full medical ecosystem: undergraduate medicine, global licensing, postgraduate and specialty examinations, clinical learning, and a future accredited continuing-education pathway. |
| 2C | Extensible global examination architecture, not an application hard-coded for one examination. |
| 3C | Hybrid content: original and commissioned material, properly licensed third-party material, permitted integrations, suitable open resources, and rights-checked private imports. |
| 4C | Both B2C individual learners and B2B institutions, universities, teaching hospitals and training organizations. |
| 5C | Responsive web, PWA, iOS, Android, Windows and macOS. |
| 6C | Full agentic intelligence behind one coherent learner-facing Coach. |
| 7C | Free, Student, Pro, Ultimate and Institutional subscription tiers. No lifetime plan is assumed. |
| 8A | Autonomous AI Agent Active as a central product foundation, with exceptionally deep personalization. |
| 9 | Technology stack (17 September 2026): Rust for the backend and all APIs; Tauri for desktop and mobile applications; Astro for the marketing and landing website. Replaces the TypeScript backend and Capacitor candidates of version 1.0. |
| 10 | The same user interface also ships as a responsive web application and PWA, reaffirming 5C. |
| 11 | SvelteKit and TypeScript remain the UI layer inside the Tauri shells and the web build. |
| 12 | Readiness policy: no numerical readiness, predicted score or pass probability is shown until validated under section 24. The Quiz LMS early-estimate feature is deferred accordingly. |
| 13 | Quiz LMS and Medical Learning OS are one combined platform. This document is the single canonical plan. |

Owner approval establishes the product capability. Each learner must still choose the permissions and privacy settings applicable to their own account. Autonomous study management does not imply permission to spend money, expose private data, change official examinations, or publish medical content without review.

### 1.1 Merge defaults applied [NEW]

These were proposed on 17 September 2026 and applied without objection. They are not explicit approvals; the owner may revise any of them.

| Topic | Treatment in this plan |
|---|---|
| Navigation | The five destinations of section 6 stand. Competition, session history and marked questions live inside Practice; the notes library and downloads live inside Learn. |
| Gamification | The section 17 guardrails govern: opt-in leaderboards under a handle, every mechanic can be disabled, no streak-anxiety targets. Daily goal, streak and question of the day ship early; leagues and duels arrive with community. |
| Subscription tiers | Free, Student, Pro, Ultimate and Institutional stand (7C). 'Until my exam' and pass extension are proposals only (section 26.1). |
| Success metrics | The section 2.2 learning objective leads. Section 2.4 numbers are health metrics, never optimization targets. |
| Mock delivery | Personal practice mocks may pre-download with answer keys kept on the server. Institutional and secured assessments stay online and server-timed. |
| Phasing | Phases 0-7 and their gates stand. The local-first data layer moves into Phase 1 (section 28.1). |
| Compute rule | The section 27 rule stands unchanged. Its consequences are recorded in section 27.1. |
| Earlier Quiz LMS 'mobile app only' decision | Superseded by decisions 9 and 10. Desktop layouts, keyboard shortcuts and fullscreen from Quiz LMS v1.0 are restored. |

## 2. Product principles and success definition

### 2.1 The product is a connected learning system, not a bundle of disconnected tools

The core loop is:

**Study -> observe evidence -> update the learner model -> diagnose the learning need -> choose a useful intervention -> act within permission -> assess retention and transfer -> improve the next plan.**

A question, a lesson, a video timestamp, an image, a flashcard, an OSCE criterion and an examination blueprint objective should connect to the same versioned concept identity. Completing one modality must inform the others without falsely treating passive viewing as proven competence.

### 2.2 Success measures

The primary learning objective is improved delayed performance on unseen, appropriately balanced items per unit of learner time. Operational measures include independent mastery, transfer to new cases, reduction in repeated misconceptions, useful-plan completion, learner-reported usefulness, and equitable performance across supported languages and device classes.

Business measures include activation, retention, conversion, institutional renewal and contribution margin. These must not replace learning outcomes as the optimization target. Do not optimize the agent primarily for screen time, chat volume, streak anxiety or upselling.

### 2.3 Claims the product must not make without evidence

No guaranteed pass, fabricated success rate, guaranteed zero hallucinations, unvalidated exam score, unsupported clinical competence certificate, or promise of support on every historical operating system. A polished dashboard is not evidence of predictive validity.

### 2.4 Operational health metrics and initial targets [NEW]

These are proposed targets, not measured achievements. They monitor product health and are subordinate to the learning objective in 2.2; none may become the agent's optimization target. Recalibrate after eight weeks of live data.

| Dimension | Initial target |
|---|---|
| Activation | Within 24 hours of first use, the learner completes one study session and starts a second task from Today. |
| Leading indicator | Reviewed questions per week: questions answered whose explanation was opened, or answered correctly with stated confidence. |
| Retention | Day-1 >= 45%, day-7 >= 25%, day-30 >= 15% of activated learners. Streak length is monitored, never targeted. |
| Guided study | At least 50% of answered questions come from Today, adaptive practice or due reviews rather than manual catalog browsing. |
| Reliability | Crash-free sessions >= 99.8%; zero lost acknowledged answers; Android not-responding rate < 0.2%. |
| Content trust | 100% of shared questions dual-reviewed; issue reports acknowledged within 24 hours and resolved within 72 hours; fewer than 0.5% of live questions carrying an open valid report. |
| Commerce | Free-to-paid conversion >= 5% within 30 days; monthly churn <= 8%; refunds < 3%. |
| Growth | At least 25% of installs from referrals and shares. |
| Outcome evidence | Consented exam outcomes collected from >= 30% of learners who sit an exam, feeding the validation work in section 24. |
| Store quality | Public store rating >= 4.7 once review volume is meaningful. |

## 3. Competitive synthesis and research limits

Public product descriptions establish what companies advertise. They do not establish independent educational effectiveness, content permissions, actual uptime, hidden implementation details, or the absence of a feature behind a login. Earlier numerical claims should not be copied into the new product's marketing or business model.

| Benchmark family | Relevant publicly described capability | Requirement for this product |
|---|---|---|
| iMD | Broad QBank, textbook, reference and video aggregation. [S01-S02] | A unified resource catalog, rights-aware content access, offline packs, and search across modalities. |
| QBankly | Questions, SRS, libraries, analytics and cloud synchronization. [S03] | Integrate these capabilities around a shared learner record; validate analytics rather than repeat marketing claims. |
| Coursology | Questions, explanations, flashcards, media, notes and cross-device study. [S04] | A low-friction browser experience with coherent transitions between activities. |
| StepwiseMD | Broad examination preparation positioning. [S05] | Country-specific exam packs on a reusable engine. |
| UWorld | Questions, library, notes, study planning, assessments and AI assistance. [S06] | Deep question review, reliable exam delivery, misconception feedback and source-linked tutoring. |
| AMBOSS | Interlinked QBank/library, recommendations, clinical reference and educator workflows. [S07] | Bidirectional question-to-knowledge connections and institutional teaching tools. |
| USMLE-Rx | Integrated USMLE study tools. [S08] | Coordinate lessons, question practice and retrieval practice around one objective. |
| Osmosis | Visual content, recall tools and contextual AI tutoring. [S09-S10] | Explain, quiz and review within the lesson without forcing context switches. |
| Lecturio | Socratic tutoring and educator AI for courses and assessments. [S11-S12] | Learner support plus human-controlled faculty authoring and remediation workflows. |
| USMLE Fighter | Quiz creation, AI notes, mistake-derived flashcards and group study. [S13] | One-click transitions from a mistake to a reviewed learning intervention. |
| StepUp / MDSteps | CCS and clinical-reasoning practice alongside QBank tools. [S14-S15] | Time-dependent management cases and explanation of reasoning errors. |
| MedLumen / TopQBank | International-exam QBank positioning and learning support. [S16-S17] | Shared medical foundations with distinct local guidance and exam configurations. |
| Anki | FSRS scheduling with configurable retention/workload behavior. [S18] | Evidence-driven spaced repetition with backlog controls, not an LLM inventing review dates. |
| Complete Anatomy | Interactive anatomy, cross sections and radiological learning resources. [S19] | A licensed or original anatomy-and-imaging layer connected to the knowledge graph. |
| Body Interact | Dynamic virtual patients and decision-dependent physiology. [S20] | Authoritative simulation state, consequences, timelines and debriefing. |
| Geeky Medics / SimChat | Conversational patients and structured examiner feedback. [S21-S22] | Voice/text OSCEs, transcript evidence, formative rubrics and faculty review. |
| UWorld mobile app [NEW] | App Store reviewers describe failures and lost progress when connectivity drops, alongside strong content. [S44] | Treat offline reliability and never-lost answers as a primary differentiator (sections 22 and 30). |
| AMBOSS mobile app [NEW] | Offline Qbank sessions and library; a publicly described score predictor and readiness analytics. [S45-S46] | Offline is an entry requirement, not a differentiator. Score prediction stays withheld until validated (decision 12). |
| Regional platforms: MedAngle, PreMed.PK and MDCAT practice apps [NEW] | Local exam coverage, large question counts and low prices; public listings mention offline mode, AI chat and score prediction. [S47-S48] | Compete on reviewed quality, exam-faithful configuration and personalization rather than volume. This is a hypothesis to validate with learner interviews. |
| Habit products such as Duolingo, a non-medical benchmark [NEW] | Goals, streaks, leagues and notifications that sustain daily use. Not audited in this research. | Borrow mechanics only within the section 17 guardrails: optional, low-pressure, accuracy over speed. |

**Additional previously identified comparators retained in the audit register:** USMLE.Study, Medora, MedicoSpira, NextStepMD, PassMedicine, CanadaQBank, AceQBank, BMJ OnExamination, BoardVitals, TrueLearn and specialty services such as RADPrimer. Their existence in the comparison set is not a licensing or integration commitment. Pages with limited readable content were not treated as fully audited applications. An authenticated audit should capture feature evidence, limitations, accessibility, payments, export behavior and offline behavior before making comparative marketing claims. [S40-S43]

**Rights warning:** QBankly's own terms distinguish its platform from third-party content and state that it does not license that third-party content. Do not adopt such a disclaimer as this product's content-acquisition strategy. [S23]

## 4. Gaps converted into concrete product requirements

These are proposed differentiators and failure modes to address, not assertions that every competitor lacks them.

| Gap or failure mode | Required advancement | Acceptance evidence |
|---|---|---|
| Resource overload | One prioritized Today plan across questions, lessons, reviews and cases | Users can identify and begin the next useful task without opening separate catalogs. |
| Repeated questions inflate performance | Item-family deduplication and first-seen/assisted/repeated attempt separation | Readiness evaluation excludes or discounts contaminated evidence. |
| Wrong answer automatically means weak knowledge | Error-triage engine that considers ambiguity, accidental taps, translation and answer-key defects | Suspected defects can quarantine an item and reverse affected analytics. |
| Generic personalization | Concept-level learner state with uncertainty, constraints and transfer evidence | Two learners with different histories receive meaningfully different justified plans. |
| AI generates endless study work | Time-budgeted intervention selection and new-card caps | No automatic plan exceeds the learner's approved capacity. |
| Prediction without validation | Coverage first; calibrated prediction only after external validation | Insufficient data shows an honest message instead of a percentage. |
| Old exam interfaces | Versioned exam specifications and effective dates | Exam-date-specific configuration fixtures pass official-format checks. |
| Chat that cannot act | Permissioned action tools and atomic plan revisions | Accepted plan changes persist, synchronize, explain themselves and can be undone. |
| Unsafe or stale knowledge | Source provenance, country/date scoping and change-impact tracking | Corrected content identifies affected cards, questions and learners. |
| Hidden agent costs | Event-driven orchestration, budgets and graceful degradation | Static learning remains usable when AI allowance or provider availability is exhausted. |
| Offline claims without conflict handling | Download manifests, durable event queues and deterministic reconciliation | Offline attempts synchronize once without lost answers or duplicate reviews. |
| Institutional surveillance | Explicit visibility boundaries and private-by-default coaching | A lecturer cannot access private chat simply because a learner belongs to a cohort. |
| Fragile imports | Page/slide/table/image preservation and extraction QA | Unparsed regions are surfaced; incomplete content is not marked complete. |
| Feature-heavy clutter | Five primary learner destinations and progressive disclosure | Advanced tools do not occupy every home screen. |
| Percentiles on self-built quizzes [NEW] | Expected-score comparison computed from community correct-rates of the exact questions served; percentiles only for fixed forms | A self-built session shows 'You 84% - expected 68%' and no percentile. |
| Question-bank piracy and account sharing [NEW] | Platform capture protection, traceable watermark on text surfaces, device limits, encrypted packs and attestation | A pack copied to another device cannot be opened; a leaked screenshot of question text is traceable. |
| Leaderboards without integrity [NEW] | Opt-in competitions with single-sitting rules, answers locked until close, anomaly review | Flagged entries are hidden pending review; any prizes follow review. |
| Poor experience on low-end phones [NEW] | Performance budgets on a reference 2-3 GB Android device and a minimum WebView version | Section 30.2 client budgets pass on the reference device using CI-produced builds. |
| Store rejection risk [NEW] | Compliance checklist: in-app account deletion, Sign in with Apple, purchase rules, medical-tool labeling | Both stores approve without policy exceptions. |
| Issue reports accumulate [NEW] | Report SLA with visible timers and reporter feedback | Reports are acknowledged within 24 hours and resolved within 72 hours. |

## 5. Product boundaries and global curriculum model

### 5.1 Learning pathways

Architecture supports preclinical and clinical undergraduate study, licensing preparation, postgraduate/specialty examination study, clinical skills, rotations, lifelong review, and continuing education.

The exam registry must accommodate the requested families: USMLE; COMLEX; subject/shelf-style preparation; UK MLA/PLAB; MRCP and MRCS; AMC; MCCQE; FCPS; JCAT/PGET; Saudi and UAE licensing examinations; FMGE; NEET-PG; INI-CET; and subsequent country or specialty packs. This is an architectural coverage target, not a representation that all packs already exist or that every named pathway retains the same current format.

**[NEW] Quiz LMS pathway families.** The Quiz LMS baseline also names MBBS professional examinations, university examinations and pre-medical entrance tests such as MDCAT. Undergraduate professional examinations fall inside decision 1C. Pre-medical entrance packs bring under-18 learners and non-medical subjects, so they remain an open decision in section 32; the exam-pack architecture must not preclude them.

### 5.2 Exam pack contract

Each released pack has an official source, owning editorial team, country, language, current name and aliases, blueprint version, effective dates, supported examination dates, content coverage map, question formats, block rules, scoring rules, breaks, accessibility behavior and last verification date.

**[NEW] Template fields.** Block rules and scoring rules explicitly include: number of blocks and questions per block; time per block; scheduled breaks and carry-over rules; options per question (two to ten, labels generated); marking scheme including negative marking and unanswered scoring; navigation rule (free, or no return to a completed block); tools allowed (calculator, lab values, highlight, elimination); pass mark and any score-scale mapping; and the lab reference-value set in conventional and SI units. Templates are versioned with the exam version.

An alias such as a former exam name must resolve to a current record without silently serving retired formats. Examination eligibility and registration information are separate, source-maintained reference records; an AI must not invent them.

The need is practical: official USMLE documentation describes 2026 changes to block structure and software, while the MCC describes the current MCCQE as 230 MCQs in two sections. Copying a competitor's historical labels would not be sufficient. [S24-S25]

### 5.3 Cross-exam transfer

Maintain a shared biomedical foundation and separate jurisdiction-specific management, ethics, prevention and legal-context overlays. An exam-switch report shows transferable knowledge, new blueprint objectives, guideline differences and unassessed areas. Do not transfer a numerical score or mastery estimate across examinations without appropriate evidence.

### 5.4 Clinical-use boundary

Launch as an education and general-reference product using fictional, consented or appropriately de-identified teaching cases. Personalized real-patient diagnosis, treatment recommendations and live hospital-system integration are outside the initial authorization. Any future clinical decision-support product requires its own intended-use, governance, regulatory and validation process.

### 5.5 Navigational hierarchy [NEW]

Learners and editors browse content through **Exam -> Subject -> System -> Chapter**, extensible to Topic and Subtopic, with many-to-many links where a system spans subjects. This hierarchy is a navigational and reporting taxonomy mapped onto the versioned concept identities of section 2.1. It is not a second source of truth. It drives the Qbank builder, analytics drill-down and editorial management. Each node carries status, display order and its blueprint mapping.

## 6. Learner experience and information architecture

Five primary learner destinations:

1. **Today:** the next action, current plan, meaningful agent changes and resume points.
2. **Practice:** QBank, flashcards, examinations, OSCE, simulation and image practice.
3. **Learn:** library, video, anatomy, notebooks, courses and licensed resources.
4. **Coach:** contextual conversation, voice, explanations and agent activity.
5. **Progress:** concept mastery, coverage, retention, assessed readiness and longitudinal portfolio.

Profile, downloads, billing, accessibility, notifications and privacy live behind the avatar. A global command/search control opens anything without adding more primary navigation.

Faculty, institution administrator and platform-owner workspaces are distinct role-aware surfaces. The owner dashboard must never be merely the dashboard of one institution.

**[NEW] Placement of merged features.** Competition, leagues and duels open from Practice. Session history (filterable by date, type, exam, subject and score, with review and revision actions), marked questions and revision sessions live in Practice. The notes library and downloads manager live in Learn. Today stays minimal as 7.2 requires: continue-session, due reviews, question of the day and goal progress appear as compact entries in the remaining-plan list, while recent results, performance overview and competition rank live in Progress and Practice. No more than three taps separate app launch from the first question. Primary navigation is hidden during a study session. Every screen is deep-linkable so notifications and shared links land on the exact action.

### 6.1 Onboarding flow

Create account -> choose personal/institutional context -> choose current pathway and exam date or exploratory mode -> specify realistic available time -> choose language and accessibility preferences -> inspect autonomy permissions -> optional short baseline assessment -> receive a modest first plan.

Initial estimates are explicitly provisional. Onboarding must not demand a long diagnostic before allowing useful study. User changes to availability or exam date must be easy and reversible.

**[NEW] Onboarding additions.** Target under three minutes to the first question. The declared available time doubles as the daily goal; no separate goal question is asked. A short screen explains the benefit before the operating-system notification prompt. A guest trial may let a visitor attempt a few sample questions before account creation, with progress migrated on sign-up; this is a proposed SHOULD, subject to abuse controls.

### 6.2 Core learner journeys

**Question recovery:** answer -> state confidence optionally -> receive review -> identify likely error -> inspect source -> targeted remediation -> approved/private card -> delayed unseen retest.

**Study interruption:** pause -> preserve answer/state -> resume on another device or offline -> reconcile without duplicate attempts -> adapt remaining plan.

**Goal change:** update target/date -> show plan and workload impact -> retain completed learning -> generate a feasible revised plan -> flag infeasible deadlines honestly.

**Institution assignment:** accept enrollment -> see course-specific work alongside personal study -> prevent conflicts with institutional deadlines -> keep private coaching separate.

**License loss:** an expired resource is removed from future recommendations and available copies according to license rules -> substitute an authorized equivalent -> retain permissible personal progress.

### 6.3 Accounts and sign-in [NEW]

| Method | Treatment |
|---|---|
| Email and password with verification | Required. Forgot and reset flows, session list and sign-out of other devices. |
| Google sign-in | Required. |
| Sign in with Apple | Required on iOS whenever a third-party login is offered (App Store guideline 4.8). [S36] |
| Phone number with one-time code | Proposed SHOULD for markets where it is expected; evaluate delivery cost. |
| Institutional SSO | Per section 18.3. |
| Biometric app unlock | Optional convenience on mobile. |

Account deletion must be startable inside the app, not only through support. Propose a limit of two active devices per personal account and one active study session at a time, with explicit takeover. Use short-lived access tokens with rotating refresh tokens held in the platform keychain or keystore. Privileged roles use MFA as in section 25.

### 6.4 Notification policy [NEW]

Push is the primary channel on mobile, mirrored by an in-app inbox; email is reserved for account and receipt messages. Categories: plan and review reminders, new mock or assignment, competition start and end, duel invitation, report resolved and subscription events.

Provide per-category toggles, quiet hours in the learner's time zone, a cap of three pushes per day of which at most one is promotional, and deep links to the exact action. Track opt-out rate per campaign. Reminder copy must not shame the learner; this applies the section 2.2 rule against streak anxiety.

## 7. UI/UX specification based on the supplied image

### 7.1 Visual direction

Default to a near-black canvas, controlled plum/violet atmosphere, soft elevated cards, subtle borders, restrained glow, rounded controls, readable typography and one visually dominant action. Use the reference as an aesthetic target, not permission to copy proprietary assets or fitness content.

Proposed starting tokens, to be validated in real screens:

| Token | Initial design value |
|---|---|
| App canvas | `#09090F` |
| Card surface | `#14141E` |
| Elevated surface | `#1B1728` |
| Primary action | `#7C3AED` |
| Accent/highlight | `#A78BFA` |
| Primary text | `#F5F3FF` |
| Secondary text | `#B8B4C6` |
| Spacing | 4, 8, 12, 16, 24, 32, 48 px |
| Card radius | 20-24 px |
| Control radius | 12-16 px |
| Reading text | 16-18 px baseline, user adjustable |
| Motion | Brief, purposeful transitions; reduced-motion alternative |

These values are proposed tokens, not a measured extraction or contrast certification. Validate every foreground/background combination and state.

### 7.2 Three signature screens

**Today / Agent Active:** greeting; exam/date context; one agent status card; one next-best task; a compact summary of meaningful changes; remaining plan. A medical/scientific emblem may replace the reference's mascot, but it must not displace useful content.

**AI Coach:** chat thread plus structured action cards showing before/after values, explanation, source links and status. Automatic low-risk changes show `Applied - Undo`; higher-impact changes show `Review changes` rather than pretending to be already applied.

**Adaptive Timeline:** original plan -> change -> current plan. Every revision records what changed, why, whether it was automatic, and what remained locked. A learner can undo a specific revision without deleting completed work.

### 7.3 Functional visual rules

Do not animate an endless 'analyzing' state when no job is running. Use truthful states: Up to date, Adapting plan, Updated, Offline, Paused, Awaiting approval, or Unable to update.

The question reader and examination workspace must prioritize reading over atmosphere. Avoid glow behind vignettes, decorative animation during timed tasks, color tints on diagnostic images and overlays that alter anatomy. Provide an exam-appropriate theme when format familiarity requires it.

Offer a light reading theme and high-contrast mode. Use icon plus text for status. Support keyboard navigation, visible focus, screen readers, captions, adjustable text, zoom/reflow and reduced motion. Target WCAG 2.2 AA, and use approximately 44-48 px primary touch targets rather than reproducing the very small controls in the screenshot. This target is a design choice above WCAG's minimum target-size criterion. [S26]

### 7.4 Responsive patterns

Mobile: bottom navigation and single-column cards. Tablet: reading/workspace split views. Desktop: side navigation, central work surface and optional contextual coach panel. No design should simply enlarge a phone screen into a desktop dashboard.

Every screen requires loading, empty, error, retry, partial-data, offline, expired-license and permission-denied states. Do not use placeholder analytics in live accounts.

### 7.5 Touch-first session workspace [NEW]

One shared session screen serves every practice and assessment mode. Available tools depend on the mode and the exam template.

| Zone | Content |
|---|---|
| Top bar | Progress such as '18 / 50', remaining time, mark control and overflow menu. In tutor sessions the timer can be hidden. |
| Reading area | Scrollable vignette and media. Question text remains the strongest visual element, consistent with 7.3. |
| Options | Full-width targets. Tap selects; tapping the selected option clears it. |
| Bottom bar (thumb zone) | Previous, navigator, next. Next becomes submit on the last question. |
| Tool tray (bottom sheet on mobile, side panel on desktop) | Calculator, unit converter, lab values, notes, highlight, text size and theme. |

Gestures on touch devices: swipe between questions, disabled while highlighting; long-press or swipe an option to strike it out, and again to restore it, with optional haptic feedback. Elimination never counts as an answer.

The question navigator shows five statuses using icon plus color: answered, unanswered, marked (independent of answer state), current and not visited. It filters by marked or unanswered and jumps on tap. Submission shows the count for each status with a shortcut to the first unanswered question.

Focus Mode on mobile hides system bars, keeps the screen awake and suggests Do Not Disturb. On desktop and web the equivalent is fullscreen. Keyboard control on desktop, web and tablets with keyboards follows the Quiz LMS map: letter keys select options, N or right arrow for next, P or left arrow for previous, F to mark, E for elimination mode and H for hint in tutor sessions. Shortcuts are inactive while typing notes.

After an incoming call, app switch, termination, reboot or power loss, the session restores to its exact state. All questions and media for a started session are prefetched before the first question appears.

## 8. AI Agent Active: personalization specification

### 8.1 What personalization means

The agent should adapt to demonstrated knowledge, uncertainty, learning goals, time constraints, context and measured response to interventions. It must not equate personalization with placing the learner's name in a chatbot message.

| Model layer | Stored evidence | Intended use |
|---|---|---|
| Declared profile | Training stage, pathway, exam/date, language, accessibility and preferred explanation depth | Select appropriate objectives and presentation. |
| Time and context | Available minutes, user-declared shifts, deadlines, offline needs and protected commitments | Build a feasible plan. |
| Concept knowledge | Independent attempts, difficulty context, recency, coverage and uncertainty | Identify what deserves assessment or teaching. |
| Memory | Review history, retrieval success, estimated stability and review burden | Schedule useful repetition. |
| Reasoning | Misconception hypotheses supported by multiple observations or learner explanation | Choose diagnostic follow-ups and contrast cases. |
| Calibration | Confidence, correctness and changes of answer | Separate confident misconceptions from cautious knowledge. |
| Transfer | Performance on new vignettes, images, OSCE tasks and simulations | Test application rather than wording familiarity. |
| Intervention response | Delayed performance after different activities | Learn which intervention was useful for this learner and objective. |

Do not infer a medical or psychological diagnosis from interaction patterns. Reading speed is not intelligence. A long response time may mean interruption, language processing, accessibility needs or careful reasoning.

### 8.2 Learner state is a structured data product

Use a versioned learner-concept record with an ability/mastery estimate, uncertainty interval or confidence category, evidence count, independent evidence count, last assessment time, memory state, misconception hypotheses, modality-specific evidence and model version.

Keep observation, inference and decision distinct. Example: 'three errors on unseen renal physiology items' is an observation; 'possible confusion between two mechanisms' is an inference; 'assign one discriminator item' is a decision. None should be silently rewritten as a fact about the person.

### 8.3 Evidence weighting and calibration

First-seen unaided performance, repeated questions, hinted answers, tutor-assisted answers, open-book attempts and self-rated flashcard reviews are separate evidence types. Content-family IDs identify close variants and mirrored content.

Time measurements exclude inactivity where possible and record accommodations. Hint use must not secretly punish a learner, but must not inflate independent exam readiness either.

Uncertain cases trigger a small diagnostic task or a question to the learner. The agent should not prescribe a long remedial course because of one questionable MCQ.

### 8.4 Cold start and sparse data

Begin with declared goals, curriculum priors and a short optional diagnostic. Show 'Not enough evidence' for fine-grained mastery. Early plans should cover representative objectives and gather useful evidence without exhausting the learner.

Use simple interpretable estimation until there is enough reliable data for more elaborate knowledge-tracing models. Train and compare alternatives against a transparent baseline; complexity is not itself intelligence.

### 8.5 The next-best-action engine

For each candidate activity, estimate likely learning value, prerequisite relevance, forgetting risk, blueprint importance, novelty and expected duration. Penalize redundancy, excessive workload and low-confidence recommendations.

Subject the recommendation to hard constraints: actual available time, paid-content entitlement, accessibility, offline availability, exam deadlines, protected tasks and user preferences. A recommended activity that violates a hard constraint must not be silently scheduled.

The initial prioritization can be explicit and auditable. Later personalized ranking or contextual-bandit experiments require sufficient consented data, safe exploration, controlled comparison and demonstrated benefit. Do not let an unconstrained reinforcement-learning agent experiment freely on learners.

### 8.6 Sustainable planning

Reserve time for breaks, realistic review, buffer and error correction. Add caps for new flashcards, intervention volume and plan-revision frequency. Missed work should trigger reprioritization, not an ever-growing punishment backlog.

A workload forecaster must distinguish 'possible in available time' from 'everything the catalog contains.' When a requested exam target becomes infeasible, explain the tradeoff and offer alternatives without fabricating predicted success.

### 8.7 Memory ownership

Provide 'What the Coach knows about me' with view, edit, delete, export and pause-personalization controls. Each remembered preference has a source and last-confirmed date. Separate self-reported facts from tentative inferences.

Use relevant retrieved state, not an indefinitely growing transcript dumped into every model request. Apply retention policies to raw conversations and event data. Do not train shared models on private coaching by default.

### 8.8 Initial estimator and selection defaults [NEW]

These are starting parameters for the interpretable baseline required by 8.4 and the explicit prioritization required by 8.5. They are configuration, not validated findings.

**Estimator.** Maintain an Elo-style ability estimate per learner and concept, rolled up along the section 5.5 hierarchy, with a matching difficulty rating per question version. Step size shrinks as independent evidence accumulates, and uncertainty is stored with the estimate. Evidence types are weighted as 8.3 requires.

**Selection.** Within the hard constraints of 8.5: target a 60-75% expected success probability per question; start from a session mix of roughly 60% unseen questions in weak areas, 25% due reviews and 15% maintenance from strong areas; prefer unseen items; do not repeat a question or a member of its content family within 48 hours.

**Difficulty fallback.** Until a chapter has about 20 independent attempts, authored difficulty moves by the Quiz LMS window rule: four correct of the last five steps up a level; two or fewer correct of five on hard steps down. Repeated errors on easy items raise the priority of that chapter. This rule remains an acceptance-test baseline.

**Displayed mastery.** A 0-100 index per chapter summarizes observed independent performance, recency, coverage and review retention. It describes practice evidence; it is not an exam-score prediction. Show an evidence level beside it, and show 'Not enough evidence' below roughly ten independent attempts. Administrators may configure the four Quiz LMS bands: below 50, 50-69, 70-84, and 85 and above.

**Learner error tag.** After a wrong answer the learner may tap one optional tag: knowledge gap, misread, between two options, or ran out of time. Store it as self-reported evidence for the error-triage engine, never as fact.

The one-tap adaptive session is labeled Smart Practice, defaulting to 20 questions with 10 and 40 as alternatives. It is the adaptive practice mode of 11.2, not a separate engine.

## 9. Agent orchestration and permissions

### 9.1 Internal roles, one visible coach

Logical services or agents include Learning State, Diagnostic Tutor, Planner, Retrieval, Flashcard Curator, Simulation Patient, Simulation Examiner, Content QA and Policy/Cost Controller. They are not necessarily separate LLMs or always-running processes.

Use a deterministic workflow graph to call only the components needed for an event. Prefer ordinary code for arithmetic, timers, permissions, scheduling invariants, billing, scoring and persistence. Use language models for explanation, dialogue, source-grounded synthesis and constrained interpretation.

### 9.2 Runtime flow

```
Learning event
  -> validate and deduplicate
  -> update authorized learner state
  -> determine whether intervention is necessary
  -> retrieve permitted evidence
  -> construct bounded proposed actions
  -> validate content, permissions, capacity and cost
  -> execute allowed reversible actions atomically
  -> persist action receipt and plan revision
  -> notify only when useful
  -> evaluate delayed outcome
```

### 9.3 Autonomy matrix

| Action | Default behavior after learner opts in |
|---|---|
| Reorder unstarted personal tasks within approved study windows | Automatic, logged and reversible. |
| Select reviewed questions and short lessons | Automatic within entitlement and capacity. |
| Reschedule existing reviews | Automatic within workload and protected-task rules. |
| Draft private cards from permitted content | Automatic; mark AI draft and validate before trusted scheduling. |
| Schedule an uncertain medical AI card | Require content validation or explicit private-draft treatment; never present as editorially approved. |
| Change an official exam date or protected institutional deadline | Not automatic. |
| Purchase content, change a paid plan or incur new charges | Explicit approval. |
| Send email/messages or edit an external calendar | Separate scoped integration permission and action-specific safeguards. |
| Share private coaching with faculty or a group | Explicit learner authorization or a clearly disclosed institutional assessment workflow. |
| Publish a question, correct a shared answer key or certify competence | Authorized human editorial/examiner workflow. |
| Delete records, change consent, grant privileges or access another tenant | Not delegated to the learning agent. |
| Enter a competition, join a league, appear on a leaderboard or send a duel invitation [NEW] | Explicit learner action only. |
| Show a paywall or upgrade prompt [NEW] | Originates from entitlement checks only; never from the agent's recommendation ranking. |

Do not interpret owner approval as a blanket authorization over other people's accounts. OWASP identifies excessive agency as a failure mode; permission enforcement belongs in application code and tool scopes, not just the prompt. [S27]

### 9.4 Reliability controls

Bound every workflow by time, cost, tool-call count, retry count and event ID. Use idempotency, an outbox, concurrency checks, a dead-letter queue and explicit cancellation. A stale plan proposal must be rejected or regenerated rather than overwriting a newer learner edit.

Each action receipt includes: triggering evidence IDs, learner/tenant context, policy version, model version where used, selected sources, proposed diff, checks, actual mutation, cost, status and undo reference. Store concise decision rationale, not hidden chain-of-thought.

### 9.5 Intelligent is not continuously expensive

Trigger work after meaningful study events, at an approved daily planning boundary, when a learner changes availability, when an assignment changes, or when relevant content is corrected. Coalesce noisy events. Do not run every agent after every click or animate work while no meaningful work exists.

**[NEW] Pre-generated tutoring.** The standard one-tap prompts - explain simply, why is my answer wrong, compare two options, give me a mnemonic, test me on this - are generated once per question version from its reviewed explanation and sources, sampled by medical reviewers, cached, and included in offline packs. A new question version invalidates them through the section 19.4 dependency graph. Free-form Coach turns remain live and metered.

## 10. Worked personalization scenario

The following is a fictional acceptance-test example, not a prediction about a real learner.

A Step 2 learner has 70 minutes today instead of the usual 150. They have recently answered two similar image questions incorrectly, both with high confidence. The second answer may have been affected by a misleading crop.

The agent first checks the question versions, image quality, previous exposure and answer keys. It flags the suspect crop for editorial review and excludes that item from the immediate weakness inference. It chooses a short unseen discriminator question to test the suspected misconception.

After the follow-up supports a specific gap, the agent schedules a focused explanation, an authorized annotated image pair and a few unseen application questions. It retains the most important due reviews, postpones a low-priority lecture, and keeps total work within 70 minutes. Any private generated cards are source-linked and clearly labeled.

The Coach says: 'I kept your highest-priority reviews and moved the longer lecture. I added a brief contrast exercise because the new question suggests this distinction is still uncertain. One earlier question is under review and is not counted against you.'

The timeline shows the exact changes and `Undo`. A delayed test checks a new case, not the same screenshot. Success updates transfer evidence; another error triggers a different approach, not simply a larger repetition queue.

## 11. Universal QBank and examination engine

### 11.1 Content model

Each question has a stable identity and immutable published versions. Required fields include learning objective, primary/secondary concepts, blueprint mapping, jurisdiction, difficulty metadata, vignette, lead-in, options, answer key, per-option rationale, images, source provenance, rights, reviewer history, content family and status.

Support single-best-answer questions initially; extensible types include multiple response, matching, ordered actions, image hotspots, short answers and case sequences. Enable a format only for an exam pack whose rules support it. Do not force every exam into the same answer or marking scheme.

**[NEW] Additional fields from the Quiz LMS baseline.** Two to ten options with generated labels; key learning point of 40 words or fewer; exam tip; optional hint; high-yield flag; authored difficulty (easy, medium, hard) alongside an empirical 1-5 rating derived from attempts; guideline review date; and alternative text for images that does not reveal the answer.

### 11.2 Practice modes

Tutor, timed, untimed, targeted weak area, blueprint-balanced random, unseen-only, incorrect, marked, confidence review, revision and adaptive practice. Adaptive practice must be visibly different from a fixed-format official-style simulation.

Features include option elimination, highlights, searchable notes, approved lab reference tables, image zoom, per-option review, keyboard controls, block creation and immediate issue reporting. Accessibility accommodations must not corrupt timing analytics.

**[NEW] One session vocabulary.** A session is defined by feedback timing, clock and pool. Named modes are presets, so the two source plans do not create duplicate engines.

| Preset | Feedback | Clock | Quiz LMS name |
|---|---|---|---|
| Tutor | Immediate answer, explanation, hint and Coach | Untimed, or optional per-question timer | Practice mode |
| Timed exam-style | Deferred to submission; hints and Coach off | Exam template timer | Exam mode |
| Review | Read-only replay of a completed session; no new attempt recorded | None | Review mode |
| Revision | New session from the incorrect and skipped questions of one earlier session | Either | Revision mode |
| Smart Practice | Tutor or timed; items chosen by 8.5 and 8.8 | Either | Adaptive quiz |

### 11.3 Examination integrity

Freeze a form and question versions at session start. Persist answers durably. Enforce the configured block navigation and timer on the server for institution-delivered assessments. Do not send future answers or full explanations to the browser before permitted release.

Record pauses and network interruptions according to the exam policy. Offline practice is allowed; offline high-stakes assessment is a separate design and risk decision. A browser alone cannot guarantee cheat prevention.

AI assistance is disabled in assessment mode when required. A learner's tutoring sessions must not disclose unreleased institutional questions or reserved assessment items.

**[NEW] Client timer and integrity signals.** The server issues a deadline. The client counts down with a monotonic clock that is unaffected by changing the device date or time, and re-anchors at each sync. Offline, the countdown continues and auto-submit occurs locally. Propose configurable grace windows: about 10 minutes after the deadline for practice mocks and 2 minutes for competitions, provided the event log is consistent. Later uploads count toward the learner's own analytics but are not ranked. For secured tests, log integrity signals: app sent to background, screenshot or recording detected, split-screen or picture-in-picture, device clock change, attestation failure, a second session attempt and, on desktop and web, fullscreen exit or window blur. Policy per test is log only, warn, or auto-submit after a set time away.

### 11.4 Psychometric lifecycle

Follow sound item-writing and blueprint principles, using official assessment guidance as a reference rather than a license to reproduce exam content. [S28]

Start with item difficulty, discrimination, distractor behavior, reliability and coverage analysis. Evaluate differential item functioning with qualified assessment expertise and appropriate data. Use IRT or computerized adaptive testing only after checking sample size, model assumptions, calibration and target-exam relevance.

Separate pilot items, operational practice items and reserved assessment families. Revisions that change difficulty or meaning require new calibration. Do not assume an IRT parameterization automatically maps onto official exam scores.

**[NEW] Initial screening defaults.** Begin analysis once a question version has about 100 attempts. Flag for review when the proportion correct is below 0.20 or above 0.95, point-biserial discrimination is below 0.15, a distractor is chosen more often than the key, three or more valid reports exist, an explanation is missing, or empirical difficulty disagrees with authored difficulty. These thresholds are screening heuristics to be confirmed by the assessment specialist.

### 11.5 Qbank builder [NEW]

The learner selects one or many subjects, systems and chapters from a searchable tree that shows available, attempted and unattempted counts. On mobile the builder is one screen with collapsible sections, not a multi-page wizard.

Exactly four question pools are offered: all questions; incorrect plus skipped; unattempted only; marked only. Due reviews are deliberately not a fifth pool; they belong to section 13. Counts are 5, 10, 20, 25, 40, 50, 100, custom or all available. The system never builds more questions than exist and says so, for example 'Only 64 questions are available for your current selection.' Difficulty is multi-select. A high-yield toggle, saved presets, 'repeat last setup' and a one-tap Quick 10 from Today are included. The builder works offline from downloaded packs, with pool counts calculated on the device.

### 11.6 Session tools baseline [NEW]

In addition to the features in 11.2:

| Tool | Requirement |
|---|---|
| Medical calculator | BMI, BSA, mean arterial pressure, GCS, creatinine clearance, eGFR, anion gap and corrected calcium at first release. It is the section 16 deterministic calculation engine surfaced in the tool tray, labeled for exam practice and not for clinical use. |
| Unit converter | General units and substance-specific laboratory conversions such as mg/dL to mmol/L. |
| Text size | Four sizes, also following the operating-system text setting. |
| Hint | Tutor sessions only. Guides reasoning without revealing the answer. Use is recorded as assisted evidence under 8.3. |
| Notes | Autosaved, attached to the question, available offline and surfaced in the 12.5 notebook. |
| Marking | Persists beyond the session and feeds the marked pool. |
| Local-first save | Selected answer, current question, mark, notes, highlights, eliminated options and timing are written to the device before any network call. |
| Auto-submit | At expiry of the session, block or section timer, with warnings at 10, 5 and 1 minute by banner and haptic. Works offline. |

### 11.7 Mock tests [NEW]

Administrators configure mocks; learners do not pick the questions. Settings: name, exam and template version, question count, blueprint distribution by subject, system and chapter, difficulty distribution, time limit, pass mark, availability window, attempts allowed, question and option randomization, and visibility of results, explanations and community comparison. Types: full, mini, subject, system, chapter, grand test and final assessment. Eligible questions are selected automatically against the blueprint, and the form is frozen as 11.3 requires.

Results show score, percentage, correct, incorrect, skipped, time taken, result against the configured pass mark, community average and percentile among takers of the same mock, with breakdown by subject, system, chapter and difficulty. Add time analysis (rushed and over-long questions) and answer-change analysis (right to wrong, wrong to right, wrong to wrong). A mock result is an observation about that form. It is not a prediction of an official result, and the screen says so.

A personal practice mock may be downloaded before it starts so that a dropped connection cannot ruin a long sitting. Answer keys and explanations stay on the server until submission. Institution-delivered and secured assessments remain online and server-timed.

### 11.8 Community statistics [NEW]

Per question: option distribution, percentage correct and optional average response time, hidden until a configurable minimum sample is met (default 20 attempts). Fixed forms show community average, median, attempts, pass rate and percentile, calculated on the server against the same exam population and the same form. Self-built and adaptive sessions show the expected-score comparison described in section 4. Community statistics are aggregates and never identify a learner.

### 11.9 Session results [NEW]

Every completed session shows score, percentage, correct, incorrect, skipped, total and time taken, with breakdown by subject, system, chapter and difficulty. One primary next-best action is offered. The Quiz LMS result actions remain available: review, revision, retry, practice incorrect plus skipped, practice similar questions and practice weak chapters.

## 12. Knowledge library, media and notebooks

### 12.1 Connected library

Versioned articles, references, ebooks, procedural guides, algorithms, glossaries, tables, annotated figures and calculators. Each object links to concepts, questions, related lessons, date and jurisdiction. Search results must distinguish editorial content, licensed excerpts, private documents and AI-generated notes.

### 12.2 Search and retrieval

Combine exact text search and semantic retrieval. Filter by entitlement, tenant, user visibility, jurisdiction, content status and source version before information enters a response. Return a specific page, paragraph, figure or video timestamp, not only a generic book title.

A question about a disputed or changing topic should surface relevant differences rather than silently blend contradictory recommendations. A source-free answer should not acquire invented citations.

### 12.3 Document ingestion

Support PDF, DOCX, PPTX, structured web exports, EPUB where authorized, images and transcripts. Preserve sections, reading order, tables, equations, figure captions, page anchors and source-file checksums.

Use reliable native text/layout extraction first. Use OCR or multimodal recovery only for scans or failed regions, with review of important tables and medical quantities. Extracted prose and generated summaries remain separate. Missing regions, uncertain symbols and broken columns must trigger QA, not silent omission.

Uploads are scanned for malware, prohibited scripts and sensitive identifiers. Parsing is sandboxed. Imported instructions are data, not agent commands. Private uploads are private by default and subject to verified processing rights.

### 12.4 Video and audio

Adaptive bitrate, captioning, transcripts, chapters, bookmarks, playback speed and timestamp-based notes. A lesson can pause into a quiz or Coach explanation. Completion is engagement evidence, not mastery proof.

Offer data-saving downloads where licensed, an audio-first lesson alternative and chapter-level resource packs. Do not require streaming high-resolution media for basic learning.

### 12.5 Personal notebook

Source-linked notes, backlinks, concepts, drawings, image annotations, collections, private imports, version history and portable export. The agent proposes edits rather than silently rewriting a learner's notes. Shared notes and instructor annotations have explicit authorship and visibility.

## 13. Spaced repetition and recall system

Implement a testable scheduling engine, with FSRS as the initial benchmark candidate and license review of the chosen implementation. Anki's documentation makes the retention/workload tradeoff explicit; the product must expose this tradeoff instead of promising perfect recall at no cost. [S18]

Support front/back, cloze, image occlusion, audio, clinical discrimination and short explanatory cards. Cards link to their source and question/concept origins.

Required controls: deduplicate near-identical cards; bury siblings; suspend obsolete cards; cap new cards; redistribute overdue work; allow easy days; preserve scheduling history; and separate self-rating from objectively marked performance.

A medical claim generated by AI is not automatically a trusted card. Editorial decks require review. Private drafts require visible status and citation checks, and must never contaminate a shared trusted deck simply because they were used frequently.

Anki import/export should be evaluated against format compatibility and content rights. Do not promise lossless compatibility for every add-on or proprietary deck without testing.

**[NEW] Question re-test queue.** Every incorrect, skipped, guessed or hint-assisted question enters the same scheduler as a re-test, optionally joined by marked questions. Grading is objective: correct and sure counts as good, correct but unsure as hard, wrong as again. Where an unseen member of the same content family exists, schedule that variant in preference to the identical item, consistent with the transfer principle in section 10; otherwise reshuffle the options. Re-tests are objectively marked evidence and remain separate from self-rated card reviews.

**[NEW] Editorial key-point cards.** The reviewed key learning point of a question can be issued as a short card. Because it comes from reviewed content it carries the editorial trust label, unlike AI drafts.

**[NEW] Defaults.** Propose a cap of 30 reviews per day, adjustable, with most-at-risk-first triage of any backlog. Interval compression before an exam date is a learner-visible option that states its workload cost. The Quiz LMS revision-tracking rule - wrong, wrong, correct, correct gradually contributing to mastery - is implemented through these review states. A Rust FSRS implementation is the first candidate, so that the same scheduler runs on the server and on devices.

## 14. Clinical reasoning, OSCE, CCS and simulation

### 14.1 Voice/text OSCE

Support history-taking, explanation, consent, counseling, breaking difficult news, handover, shared decision-making and oral examination. Each station has a reviewed scenario, patient facts, disclosure rules, learning objectives, rubric and permitted variations.

Use speech recognition, text and speech synthesis behind replaceable adapters. Display uncertain transcript segments and allow correction before final feedback when appropriate. Evaluate accent, language and disability effects.

### 14.2 Patient and examiner separation

The patient agent sees the patient script and allowed disclosure state, not an unrestricted examiner answer guide. The examiner receives the completed transcript and action log plus a locked rubric. Tutor hints and scoring are separate modes.

Every criterion-level judgment should link to transcript or action evidence. If evidence is absent or a physical skill was not observed, report 'not assessed' rather than awarding or failing it by invention. Human review and appeal are required for consequential institutional assessments.

### 14.3 Dynamic patient engine

The authoritative source of physiology is a reviewed simulation model or authored state machine. The LLM can express patient dialogue, but cannot spontaneously invent the patient's vital signs, lab results, contraindications or treatment response.

Support time progression, investigation ordering, treatment actions, resource constraints, deterioration, escalation, adverse effects and replay. Calculations and state transitions are deterministic and tested.

### 14.4 Simulation learning features

Debrief timelines, counterfactual replay, team roles, handovers, supervision prompts, medication-safety cases and jurisdiction-specific pathways. A simulation score is formative unless independently validated for the intended use.

Natural conversation and decision-dependent virtual patients already exist in the market. The differentiation sought here is connecting their evidence to the same longitudinal learning system used by questions and reviews. [S20-S22]

## 15. Anatomy, radiology and visual-learning center

Build licensed/original 3D models, cross-sectional anatomy, image hotspots, progressive reveal and spatial quizzes. Include ECG, pathology, dermatology, ultrasound and radiology cases as distinct reviewed content types.

For radiology teaching, support stack scrolling, window/level, zoom, structured findings, localization, normal/abnormal comparison and expert annotations where image rights permit. Preserve diagnostic image values; never apply the purple UI treatment to the medical pixels.

A DICOM teaching pipeline must address metadata and burned-in identifiers, consent, provenance and access. Image-processing operations must be documented and validated. Automated redaction alone is not proof that a case is safe to publish.

Use lightweight still-image modes on devices that cannot support advanced rendering. Introduce complex 3D/volume rendering only after performance and educational-value checks. License or partner for high-quality anatomy assets rather than treating an accurate atlas as a trivial asset-generation task. [S19]

## 16. Clinical reference and continuing education

General-reference content may include drug monographs, interaction-reference links, guideline summaries, differential tables, evidence-linked calculators and procedural checklists. Calculation engines use deterministic versioned formulas with unit tests, not free-form LLM arithmetic.

Education mode must remain distinct from real-patient clinical decision support. A 'clinical reference' label is not permission to process patient records or offer unvalidated individualized care.

Build continuing-education activity records, attendance, assessment, feedback, credits ledger and export capabilities. Issue completion records without calling them accredited credits until a valid accreditation or joint-provider arrangement exists for the relevant jurisdiction and activity. ACCME, for example, provides a formal accreditation and joint-providership framework. [S29]

**[NEW] Store review note.** Apple reviews medical tools with extra scrutiny and restricts drug-dosage calculators to approved bodies. Confirm eligibility before including any dosage calculator in a store build. [S36]

## 17. Community, mentoring and career continuity

Opt-in study groups, shared permitted decks, group quizzes, moderated discussions, accountability rooms, peer explanation and journal clubs. Prevent copyrighted exam leaks, harassment and deceptive credential claims.

Peer explanation must remain visibly peer-authored, with a correction/report route. Do not rank individual students publicly by default. Support private goals and optional low-pressure gamification; allow streaks and leaderboards to be disabled.

The career portfolio may store rotation learning objectives, case reflections without patient identifiers, procedure observations, feedback, learning certificates and exam preparation history. Mentoring and faculty sign-off require real accountable human roles, not an AI impersonating a supervisor.

### 17.1 Competition module [NEW]

Carried from the Quiz LMS baseline and governed by the guardrails above. Joining a competition is the learner's opt-in to appear on that leaderboard under a chosen handle; nobody is ranked publicly otherwise. Learners under 18, if such packs are approved, default to a generated handle.

Types: daily, weekly, monthly and live competitions among learners preparing for the same exam. Duels (head-to-head, live or asynchronous, five to ten questions, same items, time and difficulty) and group challenges implement the private challenges of COMMUNITY-02. Leagues place about 30 learners on the same exam into weekly cohorts with promotion and relegation.

Scoring: easy 5, medium 10 and hard 15 points, configurable. Wrong answers carry a penalty (proposed 25% of the item's points) so that random guessing is not rewarded. A speed bonus applies only to correct answers and is capped at 20% of item points, so accuracy always dominates. Ties break on accuracy, then total time, then earlier submission.

Leaderboards show rank, handle, score, accuracy, questions attempted and average response time, with daily, weekly, monthly and all-time views and optional country, institution, batch, friends and group filters. The learner's own row is pinned. Email and profile data are never shown.

Fairness: one attempt in a single sitting; per-learner randomized question and option order; answers and explanations locked until the window closes; live events served one question at a time by the server; the integrity signals of 11.3; anomaly detection for implausibly fast correct answers and near-identical answer patterns. Flagged entries are hidden pending human review. Prizes, if ever offered, are confirmed only after review.

### 17.2 Engagement mechanics [NEW]

| Mechanic | Rule | Phase |
|---|---|---|
| Daily goal | Derived from the learner's declared available time; questions or minutes. | 1 |
| Streak | A day counts when the goal is met. Streak freezes are earned, with at most two held. | 1 |
| Question of the day | One question per exam per day, pushed at a time the learner chooses; community split shown after answering. | 1 |
| XP | Per correct answer scaled by difficulty, with a review bonus and nothing for speed. | 2 |
| Achievements and weekly recap | Coverage, mastery and consistency milestones; a weekly summary of progress and next focus. | 2 |
| Home-screen widgets and lock-screen mock timer | Native extension targets outside the webview. Build only if the Phase 0 spike shows acceptable cost. | 3 |

Every mechanic can be switched off, rest days are not punished harshly, and none of these numbers is an optimization target for the agent. Question discussions are visible only after answering, never inside assessments, and may carry a reviewer-verified badge.

## 18. Institutions, faculty and platform-owner administration

### 18.1 Tenant hierarchy

Platform -> institution -> campus/program -> cohort/course -> assignments/assessments. A person may belong to multiple institutions while maintaining a separate personal learning context.

Implement roles such as learner, instructor, author, medical reviewer, examiner, program lead, institution administrator, billing administrator, support and platform owner. Separate content authoring, clinical approval and publishing permissions.

### 18.2 Faculty workspace

Curriculum maps, objective coverage, lesson sequencing, assignment creation, permitted question selection, rubrics, formative remediation, cohort trends, individual support needs, announcements and office-hour support.

An educator AI may assemble draft lessons and assessments from authorized content, but faculty review governs publication and consequential use. Lecturio's public educator workflows offer a relevant benchmark for this division of work. [S12]

### 18.3 Institution dashboard

Seat/license management, enrollment, SSO, cohorts, branding, data policies, retention, billing, usage, learning outcomes, curriculum coverage, accommodations, audit exports and designated contacts.

Do not expose private learner chats, personal notes or unrelated institution activity by default. Cohort aggregates require minimum group-size and disclosure controls appropriate to the deployment.

### 18.4 Real SaaS owner console

Separate views for total customers, institution status, revenue/renewals, payment health, plan entitlements, resource royalties, editorial backlog, content licenses, model costs, agent failures, source freshness, support cases, safety incidents and service health.

Provide feature flags, model-routing configuration, tenant-level limits, reversible suspensions, content withdrawal, audit logs, incident response and supervised support access. Support impersonation, if implemented at all, must be time-limited, purpose-recorded and highly restricted.

**[NEW]** Add the product-health views of section 2.4: activation, retention cohorts, conversion, reviewed questions per week, crash-free rate and report SLA.

### 18.5 Education interoperability

Plan LTI 1.3/LTI Advantage for authorized launches, content linking, role/context exchange and grade services; QTI for supported question/test interchange; and an extensible competency mapping layer. Certification is a separate activity, not something claimed because an endpoint exists. [S30-S31]

## 19. Content business and editorial operating model

### 19.1 Four content channels

**Original:** internally authored or commissioned questions, lessons, media and cases with documented ownership and contributor agreements.

**Licensed:** commercial content with explicit scopes for display, search, offline use, derivatives, translations, embeddings and AI processing where applicable.

**Open:** sources whose actual terms support the intended commercial and derivative use, with attribution and other obligations preserved.

**Private import/integration:** material the learner or institution is permitted to process, with visibility and export restrictions retained. A personal subscription to another service must not be assumed to permit bulk copying or rehosting.

### 19.2 Rights ledger

Store licensor, contract/version, assets, territories, audiences, dates, seat constraints, offline terms, quotation limits, AI permissions, derivatives, attribution, royalties and revocation behavior. Authorization checks cover the original and generated descendants.

Include permission-aware deep links when embedding or importing is not permitted. Do not bypass access controls or claim official partnerships that do not exist.

### 19.3 Editorial workflow

Acquire -> rights verify -> ingest -> parse QA -> classify -> map concepts -> draft -> medical review -> item-writing review -> accessibility and bias review -> pilot -> approve -> publish -> monitor -> revise or retire.

Required editorial roles include medical lead, specialty reviewers, assessment specialist, content operations and rights owner. AI accelerates drafting and triage; it does not replace accountability.

**[NEW]** The author of an item cannot be its approver. The launch content bar for a pack additionally requires per-option rationale and a key learning point on every question.

### 19.4 Correction and change propagation

Use a dependency graph from source passages to questions, explanations, cards, lessons and simulations. A correction identifies affected resources and learners, creates review tasks and can immediately quarantine unsafe content.

Published edits create new versions. Historical examination attempts retain the version used. Recalculation and notification policies distinguish a small spelling fix from an invalid answer key.

**[NEW] Issue reports.** Learners can report a question as wrong answer, incorrect explanation, typo, duplicate, broken image, outdated information or other. Reports are acknowledged within 24 hours and resolved within 72 hours, with timers visible in the console. The reporter is told the outcome, and corrected questions carry a 'corrected' badge and changelog.

### 19.5 Editorial console baseline [NEW]

Carried from the Quiz LMS administration requirements and delivered as a role-aware surface of the web build.

- Hierarchy management for exams, subjects, systems and chapters: name, description, status and display order.
- Question management: search; filter by hierarchy, difficulty and status; create, edit, duplicate, archive, publish and bulk actions; version history; psychometrics panel.
- Bulk import from Excel and CSV, using a template that supports variable options, per-option rationale, key learning point, hint, hierarchy, difficulty, tags, references and media references. Validate before import and report valid rows, warnings and errors. Support dry run and rollback of an import batch. The rights checks of section 19.2 apply to every batch.
- Mock builder (11.7) and competition configuration (17.1).
- User management: search, filter, view profile and analytics within policy, suspend, reactivate, reset access and change role.
- Settings: mastery bands, competition points and tie rules, community-statistics minimum sample, free-tier allowance, review caps, offline lease length and integrity policies.
- Reports and exports in CSV and Excel: user activity, attempts, mock performance, competition participation, question usage, issue reports and weakest chapters across learners, subject to the section 18.3 disclosure controls.
- Audit log of question created, edited or deleted, answer key changed, mock or competition created and user suspended, with actor, action, entity, old value, new value and timestamp.

## 20. Technical architecture: proposed default

This is a greenfield recommendation, not a verified description of an existing repository. If the actual project already exists, audit it before choosing to replace working components.

### 20.1 Application stack [AMENDED]

Replaced by owner decisions 9-11 of 17 September 2026. Version 1.0 proposed a TypeScript backend and Capacitor for mobile; both are withdrawn. Named crates and plugins below are candidates to confirm in Phase 0, not verified selections.

| Layer | Decision or candidate |
|---|---|
| Backend and APIs | Rust modular monolith with explicit domain boundaries, typed request validation and a versioned API. Candidates: Axum on Tokio, SQLx with compile-time-checked queries, generated OpenAPI, tracing with OpenTelemetry. |
| Workers | Rust worker processes for ingestion, agent jobs, pack builds, scheduling, psychometrics and purchase reconciliation, on a queue and durable workflow layer with a transactional outbox as the reliability foundation. Media transcoding, OCR and document parsing may call sandboxed external tools from Rust workers. |
| Real-time | WebSocket endpoints in the same Rust service for live competitions, duels and streaming Coach or voice turns. |
| Data | PostgreSQL for transactional data and policy-controlled records; a suitable vector-search extension/index for semantic retrieval after benchmarking; Redis or an equivalent for cache, leaderboards, rate limits and locks. |
| Storage and delivery | Object storage and a CDN for licensed media and offline packages. |
| Shared Rust core | One set of crates - domain contracts, exam engine, scheduler, learner model, sync, policy and calculation engine - compiled natively for the server and the Tauri apps, and to WebAssembly for the web build. Identical rules on every surface; no second implementation in TypeScript. |
| UI | SvelteKit and TypeScript in static/SPA mode, shared by the Tauri shells and the web/PWA build. The Tauri shell must use an appropriate static/SPA build rather than embedding server-only SvelteKit behavior. [S32, S34] |
| Contracts | TypeScript types and the API client are generated from Rust types and the OpenAPI document. Rust is the single source of truth. |
| Desktop | Tauri 2 for Windows and macOS: signed installers, signed updater and content-protected windows. |
| Mobile | Tauri 2 for iOS and Android. |
| Local data | Encrypted SQLite accessed from the Rust core in the Tauri apps. On the web, IndexedDB or origin-private file storage behind the same storage trait, with the eviction handling of section 22. |
| Marketing site | Astro: landing and per-exam pages, pricing, web checkout and billing portal calling the Rust API, help, legal, download pages and the app-link association files for deep links. Shares design tokens and Svelte components with the app. |
| Adapters | Replaceable model, speech, billing, identity, email, search and learning-standard adapters. |

**Tauri mobile risk.** The official plugin set covers deep links, biometrics, local notifications, haptics, barcode scanning and key-value storage, but the official notification plugin is local-only. Remote push (APNs and FCM) and in-app purchases (StoreKit 2 and Play Billing) currently depend on community plugins. [S49-S51] Expect to own native Swift and Kotlin plugin code for remote push, purchases, the secure-window flag and capture detection, Play Integrity and App Attest, background sync scheduling, keep-awake and keychain access. Widgets and lock-screen activities are native extension targets outside the webview.

Phase 0 therefore includes a go/no-go spike on a reference low-end Android phone and an iPhone. It covers those plugins, encrypted SQLite, cold start, scroll and tap latency in the system WebView, and the shared core running natively and as WebAssembly. If the spike fails on mobile only, the fallback keeps the SvelteKit UI and the Rust backend and changes only the mobile shell; Capacitor was the version 1.0 candidate. [S33] Shared code does not remove platform-specific accessibility, storage, signing or device testing.

### 20.2 Deployment shape

Separate frontend delivery, application API, asynchronous workers, AI gateway, transactional database, object storage and observability. Keep the first release operationally simple; split individual services only when load, security or independent scaling justify it.

Do not place model execution, transcoding, bulk document extraction and user-facing APIs in one small VPS process. Exact infrastructure sizing is a deployment decision requiring traffic, content and budget assumptions.

### 20.3 Tenant security

Use explicit tenant/user scopes, least-privilege application roles and row-level policies as defense in depth. PostgreSQL owners and privileged roles can bypass ordinary row-security behavior, so application credentials must not casually use those roles. Test API, retrieval, background jobs, exports, caches and support tools for isolation. [S35]

Shared public/licensed knowledge and tenant-private knowledge are separate namespaces. Never share a cache key or vector retrieval result between private contexts merely because prompts are similar.

### 20.4 Repository shape [AMENDED]

```
apps/api                 Rust API (modular monolith)
apps/worker              Rust workers
apps/client              SvelteKit SPA shared by Tauri and web/PWA
apps/client/src-tauri    Tauri 2 shells: Windows, macOS, iOS, Android
apps/site                Astro marketing site, checkout, help, legal
crates/domain-contracts
crates/exam-engine
crates/learner-model
crates/planner
crates/scheduler
crates/sync
crates/policy
crates/calc-engine
crates/content-pipeline
crates/agent-tools
crates/telemetry
crates/core-wasm         WebAssembly bindings for the web build
crates/tauri-plugins     Owned native plugins (Swift and Kotlin)
packages/design-system   Tokens and Svelte components, shared with apps/site
packages/api-client      Generated TypeScript types and client
content/schemas
content/evaluation-fixtures
docs/architecture-decisions
docs/requirements
tests/integration
tests/e2e
tests/security
tests/ai-evaluation
.github/workflows
```

These are suggested boundaries, not a requirement to create empty packages or extra deployments before needed.

**[AMENDED]** The tree above replaces the version 1.0 tree: `apps/web`, `apps/mobile` and `apps/desktop` become one `apps/client` with its Tauri shells, and the TypeScript `packages/*` domain packages become Rust `crates/*`. Faculty, institution, editorial and owner workspaces are role-aware routes of `apps/client`, delivered primarily through the web build. LTI launches (18.5) require that browser surface.

## 21. Core entities, APIs and event contracts

### 21.1 Data domains

Identity: User, Profile, Consent, Session, Device, Tenant, Membership, Role, Permission.

Commerce: Product, Plan, Entitlement, Subscription, PaymentEvent, Invoice, LicenseGrant, RoyaltyRecord.

Curriculum: Exam, ExamAlias, ExamVersion, Blueprint, BlueprintCell, Concept, ConceptRelation, Objective, Jurisdiction, GuidelineVersion.

Content: Asset, Source, SourceVersion, ContentItem, ContentVersion, RightsPolicy, Question, QuestionVersion, QuestionFamily, ReviewDecision, Publication, Correction.

Learning: Attempt, AttemptEvidence, SessionBlock, AssessmentForm, LearnerConceptState, MisconceptionHypothesis, LearningEvent, Recommendation, InterventionOutcome.

Planning: Goal, AvailabilityWindow, ProtectedCommitment, StudyPlan, PlanRevision, PlanTask, AgentAction, ActionReceipt.

Recall: Deck, Card, CardVersion, ReviewEvent, SchedulingState.

Simulation: Scenario, ScenarioVersion, PatientState, SimulationEvent, TranscriptSegment, RubricVersion, RubricEvidence, Feedback, ExaminerReview.

Institutions: Program, Course, Cohort, Enrollment, Assignment, Grade, Accommodation, LearningRecord.

Operations: Job, OutboxEvent, AuditEvent, FeatureFlag, ModelVersion, EvaluationRun, Incident, SyncCursor, Tombstone.

**[NEW] Added by the merge.**

Competition and engagement: Competition, CompetitionEntry, League, LeagueMembership, Duel, LeaderboardSnapshot, Streak, XpLedger, Achievement, QuestionOfTheDay.

Community: Friendship, StudyGroup, GroupMembership, DiscussionPost, ModerationAction.

Commerce additions: PriceTier, Coupon, Referral, StoreReceipt, CheckoutSession.

Delivery and trust: ContentPack, PackLease, DeviceAttestation, IntegrityEvent, PushToken, NotificationPreference, Notification, Experiment, RemoteConfig.

Assessment additions: ExamTemplate, LabReferenceSet, MockTest, KeyPoint, CommunityItemStat, ExamOutcome.

Attempt evidence additionally records the first option chosen, number of answer changes, time to first answer, time on the explanation, learner error tag, offline flag, device and client event ID.

### 21.2 API examples

```
GET  /v1/me/today
GET  /v1/me/learning-state
GET  /v1/exams/{examId}/versions
POST /v1/practice/sessions
POST /v1/practice/sessions/{id}/answers
POST /v1/assessments/{id}/submit
POST /v1/coach/turns
GET  /v1/plans/{id}/revisions
POST /v1/plans/{id}/revisions/{revisionId}/undo
POST /v1/reviews/events
POST /v1/simulations
POST /v1/simulations/{id}/events
POST /v1/imports
GET  /v1/imports/{id}/quality-report
POST /v1/sync/push
GET  /v1/sync/pull
POST /v1/institutions/{id}/assignments
POST /v1/content/{id}/reviews
POST /v1/content/{id}/publish
GET  /v1/admin/operations
```

Every write requires authorization, validation and appropriate idempotency. Use optimistic concurrency for plans and editable content. Never let an LLM write arbitrary SQL or bypass service methods.

**[NEW] Further examples from the merge.**

```
GET    /v1/config
POST   /v1/auth/apple
POST   /v1/auth/otp/request
DELETE /v1/account
GET    /v1/devices
GET    /v1/packs
POST   /v1/packs/{id}/lease
GET    /v1/mock-tests
POST   /v1/mock-tests/{id}/start
GET    /v1/questions/{id}/community-stats
POST   /v1/questions/{id}/report
GET    /v1/qotd
GET    /v1/competitions
POST   /v1/competitions/{id}/join
GET    /v1/competitions/{id}/leaderboard
POST   /v1/duels
WS     /v1/live
GET    /v1/products
POST   /v1/billing/verify
GET    /v1/entitlements
POST   /v1/coupons/redeem
POST   /v1/push-tokens
PUT    /v1/notification-preferences
POST   /v1/outcomes
```

Conventions: an idempotency key on every write; cursor pagination; entity tags for cacheable hierarchy and config; one error format with machine-readable codes; per-user and per-device rate limits; a published minimum supported client version; and compatibility with the previous two released client versions.

### 21.3 Event schema

Common fields: event ID, schema version, actor, tenant, subject, aggregate ID/version, occurred-at, received-at, device, correlation ID, privacy scope and payload.

Representative events: answer submitted, explanation viewed, confidence recorded, review completed, lesson completed, availability changed, rubric scored, source corrected, license expired, plan revised and subscription changed.

Events are append-only evidence. Derived state is recomputable. Correcting a bad event uses an explicit correction, not silent rewriting of historical facts.

**[NEW]** Product-analytics events are a separate stream from learning evidence. They are keyed by a pseudonymous ID, listed in Appendix B, and never feed the learner model.

## 22. Offline and synchronization specification

Classify capabilities honestly:

| Capability | Offline commitment |
|---|---|
| Downloaded licensed reading/media | Available within license and device policy. |
| Prepared practice questions | Available with local progress capture. |
| Flashcard review | Available with a deterministic local queue and history. |
| Cached plan | Viewable and locally actionable, pending reconciliation. |
| Cloud AI chat and advanced voice | Not available without connectivity unless a separately validated on-device option exists. |
| Institutional high-stakes exams | Not included in ordinary offline mode. |
| Newly updated clinical guidance | Shows last synchronization time; cannot imply live freshness. |
| Competitions, duels and live leaderboards [NEW] | Online only. |
| Pre-generated tutoring answers [NEW] | Available offline inside the pack; live Coach turns are not. |
| Progress and analytics [NEW] | Last synchronized snapshot plus local updates, labeled with the sync time. |
| Personal practice mocks [NEW] | Questions pre-downloaded; start and submission need connectivity; drops in between are tolerated. |

Use signed versioned manifests, resumable downloads, storage limits, checksums, license-aware access and encrypted local storage where feasible. Browser storage eviction must be handled explicitly.

Persist local events with unique IDs before acknowledging them. Use server sequence/cursors and deduplication; do not trust client clocks for ordering critical assessments. Quiz attempts are immutable records, note conflicts preserve branches or merge safely, and plan changes use version checks.

Test simultaneous devices, airplane mode, token expiry, partial uploads, app termination, clock skew, duplicate events and license revocation. Immediate revocation on a disconnected device cannot be promised; define contractual offline windows and expiry behavior.

**[NEW] Packs.** Offer packs per subject or system, with size shown before download, a Wi-Fi-only option, delta updates and a storage manager. Propose a default offline license lease of 14 days, renewed silently when online, as the concrete form of the contractual offline window above. Packs are encrypted with per-device keys.

**[NEW] Additional conflict rules.** One active device per study session, with explicit takeover and the highest sequence number winning. Marks use latest-toggle-wins. Highlight ranges merge. The learner model is server-authoritative, with the device holding a snapshot and applying provisional updates. Sessions generated offline use client-generated IDs that the server accepts. Once an answer is shown as selected it must never be lost.

**[NEW] Surface differences.** The Tauri apps carry the full offline commitment through encrypted SQLite. The web/PWA build offers a reduced, best-effort commitment with smaller packs because browsers may evict storage; the interface states this difference honestly.

## 23. AI model strategy and evaluation

Do not freeze the product to a single named frontier model. Select candidates by measured performance on medical tutoring, language coverage, image reasoning, speech, source citation, privacy, latency and cost. Use permitted commercial APIs or properly licensed deployed models, not repurposed personal subscription sessions.

Route deterministic work to ordinary code, straightforward retrieval-based explanations to a cost-efficient qualified model, and complex synthesis to a stronger qualified model. Use an independent review step for selected high-risk outputs, but do not mistake agreement between two models for medical validation.

Evaluate retrieval recall and authorization, citation existence and support, medical correctness, unsupported assertions, appropriate abstention, tutoring quality, question quality, action correctness, memory accuracy, voice fairness, model drift and cost per useful intervention.

Construct evaluation sets with medical reviewers. Keep assessment families out of tutoring and training data. Run regression tests after changes to prompts, retrieval, source versions, model versions or tool schemas.

## 24. Readiness, mastery and prediction governance

The initial dashboard reports observed accuracy, independent-attempt accuracy, blueprint coverage, retention trends, uncertainty and modality-specific performance. It must not convert any convenient percentage into a claimed pass probability.

A numerical readiness model requires a defined target outcome, suitable consented outcome data, representative sampling, leakage controls, a held-out time period, external validation where feasible, calibration analysis and subgroup checks. Official-score prediction also requires appropriate lawful linking to official outcomes and a model for each relevant exam/version.

Account for repeated items, tutoring assistance, selective practice and self-reported outcome bias. Report interval estimates and limitations. Withhold unsupported predictions rather than showing precise-looking invented numbers.

Simulation, confidence and question accuracy remain distinct dimensions. High flashcard recall does not by itself prove clinical competence.

**[AMENDED] Owner decision 12 (17 September 2026).** The Quiz LMS specification proposed an early predicted score and pass probability behind an evidence gate. That feature is deferred to Phase 7 and ships only after the validation described above. Until then Progress shows the observed measures listed in this section, and mock results are presented as observations about a form (11.7). To make validation possible, collect consented exam outcomes through a post-exam prompt. A modest reward for a verified result is acceptable if it does not bias reporting toward passes.

### 24.1 Hierarchy analytics [NEW]

Progress offers drill-down along the section 5.5 hierarchy. Each of the four levels - exam, subject, system, chapter - shows questions attempted, correct, incorrect, skipped, accuracy, independent-attempt accuracy, average response time and, where a meaningful comparison exists, the community figure. Add accuracy by authored difficulty, and trends over 7 days, 30 days, 3 months, 6 months and all time for accuracy, volume, correct-to-incorrect ratio and mastery index.

On mobile, tables become cards and a mastery heat-map allows drill-down by tap. Descriptive insights may include confidence calibration, learner error-tag breakdown, speed against accuracy and time-of-day patterns. They are descriptions, never judgments about the person, consistent with 8.1.

## 25. Security, privacy and trust

Threat model document uploads, prompt injection, cross-tenant retrieval, malicious integrations, unsafe tool calls, leaked answer keys, credential abuse, payment events, insecure desktop updates and private-media sharing.

Use appropriate authentication and MFA for privileged roles; secrets management; encrypted transport and storage; malware scanning; sandboxed extraction; signed URLs; access logs; rate limiting; tested backup restoration; dependency review; and signed application artifacts.

Treat retrieved content as untrusted data. All tools have narrow schemas and permissions. Prevent arbitrary shell commands, unrestricted URLs, arbitrary outbound destinations and cross-tenant memory retrieval from the learning agent. These are architectural controls, not assurances that prompt injection has been eliminated. [S27]

Before launch in each market, obtain appropriate legal review of privacy, education records, age eligibility, consumer billing, content licenses, medical intended use and data transfers. Do not claim GDPR, HIPAA, SOC 2 or other certification/compliance solely because common security features exist.

Define recording consent for voice/video, retention, deletion, export, incident communication and access requests. Do not collect patient identifiers in routine study workflows. Private account deletion must propagate through active stores, indexes and queued jobs under a documented retention policy.

### 25.1 Content protection and device trust [NEW]

| Area | Requirement |
|---|---|
| Transport | TLS with certificate pinning in the Tauri apps, and a rotation plan. |
| On device | Tokens in keychain or keystore; encrypted local database; no secrets in the application bundle. |
| Device trust | Play Integrity on Android and App Attest on iOS. A rooted or jailbroken device lowers trust and may restrict secured tests; it does not hard-block study by default. |
| Screen capture | Android secure-window flag on question and explanation screens. iOS: detect capture and mirroring, obscure content and log screenshots. Desktop: Tauri content-protected windows on Windows and macOS. The web build cannot prevent capture, so licensed material on the web follows the display terms in the rights ledger. |
| Watermark | A faint tiled account identifier on question and explanation text surfaces, so leaks are traceable. Never over diagnostic images or anatomy, consistent with sections 7.3 and 15. |
| Anti-scraping | Per-account limits on question views per hour and day, with anomaly alerts. |
| Accounts | Device limit and one active session (6.3). |
| Response | A takedown process for leaked content. |

Protection must never degrade accessibility: screen readers, zoom and text scaling continue to work. These controls raise the cost of copying; they do not make copying impossible.

### 25.2 Store and platform compliance checklist [NEW]

In-app account deletion; Sign in with Apple beside other third-party logins; accurate App Store privacy labels and Google Play Data Safety entries, with a maintained SDK inventory; store billing for digital content sold inside the mobile apps, and current storefront rules for any external route; medical tools labeled for education; a clearly labeled AI Coach, with disclosure when a third-party model processes learner text; and age-appropriate privacy defaults if under-18 packs are approved. [S36]

## 26. Monetization and unit economics

| Tier | Proposed product boundary |
|---|---|
| Free | Useful sample content, basic planning, limited practice, basic progress and a small visible AI allowance. |
| Student | Core released exam pack, library, flashcards, offline study and routine autonomous planning. |
| Pro | Broader released pathways, more tutoring, advanced analytics, private imports and deeper notes. |
| Ultimate | Higher speech/simulation allowances, advanced specialty tools and premium licensed add-ons where contracts permit. |
| Institutional | Seats, faculty tools, curriculum, tenant administration, assessment workflows, integrations and contracted support. |

All paid tiers retain necessary accessibility, security, export and transparency controls. Clinical accuracy is not deliberately reduced in cheaper tiers. Advanced cost-heavy usage may be limited, but source reliability must remain consistent.

Do not assume one universal 'all content' price: royalty contracts may require pack-specific entitlements. Expose these restrictions clearly before purchase.

Calculate contribution margin after royalties, model usage, speech, media delivery, payment/store fees, hosting and support. Set prices after licensing and representative usage measurements, not by copying a low-cost aggregator's advertisement.

Use monthly/annual plans, regional pricing experiments, institutional contracts and optional lawful add-ons. Handle cancellation, refunds, receipts, grace periods, web/mobile entitlement reconciliation and receipt validation. Review current store rules for each storefront; never rely on hidden activation behavior. [S36]

When an AI limit is reached, show why, when it resets and what remains available. Static explanations, saved plans and ordinary practice should continue whenever their content entitlement remains valid.

### 26.1 Commercial mechanics and growth [NEW]

These operate inside the approved tiers of decision 7C.

**Upgrade triggers.** Daily free allowance reached (propose 10-15 questions per day with full explanations); offline download; full mock test; chapter-level analytics and adaptive planning beyond the free scope; AI allowance reached. Prompts come from entitlement checks, never from the Coach.

**Payment routes.** Store billing inside the iOS and Android apps. An account-based web checkout on the Astro site for web, desktop and any learner who prefers it, supporting cards and local wallets such as JazzCash and Easypaisa, because many target learners have no international card. Every route feeds one server-side entitlement service (CORE-03). Rules on external payments, and on linking to them, differ by storefront and have changed repeatedly. Confirm them per market before building the paywall.

**Offers.** Free trial on first subscription; pause instead of cancel; coupons; referral credit for both sides; student and group discounts; regional price tiers that are remote-configurable and testable.

**Proposals awaiting owner approval.** 'Until my exam': one payment covering access to the exam date plus 14 days. Pass extension: a free extension for a learner who does not pass and submits proof. Neither may be worded as a guaranteed pass (2.3), and neither is a lifetime plan.

**Growth loops.** Referral and campus-ambassador codes; share cards for score, consistency and league results that never contain question content; duel links that install the app and open the duel through deferred deep links; a free baseline assessment as the main acquisition offer; per-exam landing pages on the Astro site; and localized store listings.

## 27. Delivery, CI and infrastructure constraint

Preserve the owner's explicit GitHub Actions-only rule for development computation. Builds, linting, tests, screenshots, static analysis, performance checks, content-processing validation, model evaluations and artifact creation for the application pipeline run in approved GitHub Actions environments, not the local development computer or the existing production VPS. Do not install self-hosted runners on those excluded machines to evade the rule.

Use GitHub-hosted or separately approved compliant runners, minimal credentials, protected environments, isolated fixtures, signed artifacts and immutable deployment provenance. Real learner or patient data must not be casually copied into CI; use synthetic fixtures and only explicitly authorized datasets.

**Unresolved runtime boundary:** always-on application serving, user-triggered background jobs and real-time AI need an approved runtime. GitHub-hosted Actions jobs have execution limits and are not a sound permanent serving layer. This document proposes separate managed application/inference infrastructure, but does not authorize that exception or reinterpret 'all compute' silently. Production deployment remains blocked until the owner explicitly approves the runtime boundary. [S37]

Changes to a live product use backward-compatible migrations, canary or staged rollout, feature flags, rollback, backup verification and post-release monitoring. No migration or production change is authorized by this plan alone.

### 27.1 Consequences of the Rust and Tauri stack under this rule [NEW]

The rule above stands unchanged. Its practical consequences are recorded so that they are budgeted rather than discovered.

- Runner matrix: Linux for the API, workers, crates, WebAssembly and Android builds; macOS for iOS and macOS builds, signing and notarization; Windows for the desktop build and signing. macOS runners are billed at a higher rate than Linux, and Rust compile times make caching essential.
- Pipeline gates: formatting, lints, dependency and license audit, unit and integration tests against an ephemeral PostgreSQL, compile-time query checks, a WebAssembly build of the shared core, browser end-to-end tests of the web build, desktop end-to-end tests and generated-contract drift checks.
- Mobile feedback loop: because local builds are excluded, device testing runs on CI-built artifacts distributed through TestFlight, Play internal testing or an approved device farm. Whether running a CI-built artifact on a developer-owned simulator or phone is permitted is an open decision in section 32.
- Signing keys, store credentials and updater keys live in protected environments with minimal scope.

## 28. Phased build program and gates

Do not attempt to ship every specialty, every operating system and every advanced simulator at once. Preserve the full architecture while proving an end-to-end learning loop first.

| Phase | Deliverables | Gate to advance |
|---|---|---|
| 0 - Decisions and evidence | Repository/greenfield decision, budget envelope, content rights inventory, pilot exam, official blueprint, runtime approval, architecture records, design tokens, reference-based interaction prototype | No unresolved decision that would invalidate the first implementation slice. |
| 1 - Connected vertical slice | Identity, tenant foundations, content/versioning, QBank, evidence events, basic learner state, Today plan, Coach, rights checks and core billing model | A learner can complete a real reviewed task and see a persisted, justified, reversible plan change. |
| 2 - Intelligent core | SRS, misconceptions, capacity-aware planning, source-grounded tutoring, library/video links, notes, imports, exam simulator and editorial console | Expert-reviewed core loop works on unseen items; no fake readiness; first exam coverage meets the agreed blueprint standard. |
| 3 - Cross-device product | PWA offline, mobile apps, desktop shells, conflict-safe sync, accessibility, purchase reconciliation and approved device matrix | No lost or duplicate attempts under the documented interruption matrix. |
| 4 - Institutional product | SSO, programs/cohorts, faculty authoring, assignments, assessment controls, institutional analytics and LTI/QTI scope | Multiple institutions remain isolated; faculty cannot see private learner data outside policy. |
| 5 - Clinical practice labs | Voice/text OSCE, rubric evidence, CCS, dynamic cases, imaging lab and anatomy integration | Clinician-reviewed scenario behavior and evidence-based formative feedback pass the chosen validation protocol. |
| 6 - Global expansion | Additional country/specialty packs, translations, guideline overlays and cross-exam transition plans | Every pack passes its own rights, clinical review, format, localization and coverage gate. |
| 7 - Advanced intelligence and continuing education | Validated outcome prediction, intervention optimization, deeper longitudinal portfolios and appropriate continuing-education partnerships | Prediction and accreditation claims are supported for the specific intended use. |

Architecture may be prepared early for later modules; this is not permission to show nonfunctional buttons as finished features. Phases may overlap in content/design/engineering lanes, but dependencies and gates remain.

A fixed calendar or budget is intentionally not represented as approved. Team capacity, rights negotiations, required review volume and runtime decisions must determine the schedule. The first pilot can cover a narrow objective set; it must be labeled a pilot, not a complete examination course.

### 28.1 Additions to each phase from the merge [NEW]

The phases and gates in the table above are unchanged. These deliverables join the named phase.

| Phase | Added deliverables | Added gate evidence |
|---|---|---|
| 0 | Tauri mobile go/no-go spike (20.1); shared Rust core proven natively and as WebAssembly; CI runner matrix (27.1); product-analytics taxonomy; store accounts and storefront payment-policy check; decision on pre-medical packs | Spike report against the 30.2 client budgets on the reference devices. |
| 1 | Local-first data layer in every client; touch-first session workspace (7.5); Qbank builder and session tools (11.5-11.6); accounts and sign-in (6.3); daily goal, streak and question of the day (17.2); notification policy (6.4); issue-report SLA; upgrade triggers and entitlement service (26.1); baseline content protection; Astro site with checkout; internal beta builds of the Tauri desktop and mobile apps alongside the web build | No acknowledged answer is lost when any client is terminated mid-session. |
| 2 | Mock builder and results (11.7); community statistics (11.8); question re-test queue and key-point cards (13); pre-generated tutoring (9.5); psychometric screening defaults (11.4); hierarchy analytics (24.1); XP, achievements and weekly recap; editorial console baseline with bulk import (19.5); coupons, referrals and regional price tiers | Expected-score comparison replaces percentiles on self-built sessions. |
| 3 | Offline packs, leases and storage manager (22); full content protection and device attestation (25.1); store compliance checklist (25.2); staged rollout, forced and soft update, two-version API compatibility; public store release of the Tauri apps | A pack cannot be opened on another device; timers cannot be extended by changing the device clock. |
| 4 | Competitions, leagues, duels, friends and groups (17.1); share cards, duel links and ambassador codes (26.1) | The integrity review queue operates within its SLA; nobody appears on a leaderboard without opting in. |
| 5 | No additions. | - |
| 6 | Right-to-left-safe interface localization; pre-medical entrance packs if approved | Each added pack passes the existing Phase 6 gate. |
| 7 | Validated numerical readiness under section 24; selection-policy experiments under 8.5 | As the existing Phase 7 gate. |

## 29. Feature inventory and traceability baseline

Codes below are stable planning IDs. Each implementation ticket must map to at least one ID, an owner, a phase, a source/decision, an API/data contract and acceptance evidence.

| ID | Requirement | Phase |
|---|---|---|
| CORE-01 | One identity with personal and institution contexts | 1 |
| CORE-02 | Versioned goals, exam dates and protected commitments | 1 |
| CORE-03 | Entitlement checks across API, media, retrieval and offline manifests | 1 |
| CORE-04 | Multi-tenant role and audit foundations | 1 |
| CORE-05 | Five-destination learner navigation | 1 |
| CORE-06 | Truthful loading, error, empty and permission states | 1 |
| QB-01 | Immutable published question versions | 1 |
| QB-02 | Question-family and variant identities | 1 |
| QB-03 | Timed/untimed/tutor practice | 1 |
| QB-04 | Confidence and assistance evidence separation | 1 |
| QB-05 | Per-option explanations and source anchors | 1 |
| QB-06 | Targeted, unseen, marked and incorrect filters | 2 |
| QB-07 | Blueprint-balanced session generation | 2 |
| QB-08 | Issue reporting and quarantined-item exclusion | 1 |
| QB-09 | Item statistics and editorial review | 2 |
| QB-10 | Validated advanced calibration when supported | 7 |
| EX-01 | Official-source exam registry with aliases | 1 |
| EX-02 | Date-effective block/timer/break configuration | 2 |
| EX-03 | Frozen assessment forms and versions | 2 |
| EX-04 | Durable answer persistence and submission receipts | 1 |
| EX-05 | Reserved assessment-family protection | 2 |
| EX-06 | Accommodations and assessment-specific AI restrictions | 2 |
| AI-01 | Structured learner-concept state and uncertainty | 1 |
| AI-02 | Cold-start plan with honest sparse-data behavior | 1 |
| AI-03 | Mistake hypotheses, not assumed diagnoses | 2 |
| AI-04 | Time-budgeted next-best-action selection | 1 |
| AI-05 | Bounded event-driven orchestration | 1 |
| AI-06 | Permissioned action tools | 1 |
| AI-07 | Action receipts and undoable plan revisions | 1 |
| AI-08 | Protected tasks and plan-churn controls | 2 |
| AI-09 | Source-grounded contextual tutoring | 2 |
| AI-10 | Socratic, explain-back and contrast modes | 2 |
| AI-11 | Learner-viewable editable memory | 2 |
| AI-12 | Delayed intervention outcome tracking | 2 |
| AI-13 | Cost limits, fallbacks and kill switches | 1 |
| AI-14 | No cross-tenant private-memory access | 1 |
| AI-15 | Evaluated multilingual tutoring | 6 |
| AI-16 | Qualified model routing and regression suites | 2 |
| PLAN-01 | Original/revised/current plan timeline | 1 |
| PLAN-02 | Capacity changes and feasible replanning | 1 |
| PLAN-03 | Review debt recovery and buffer time | 2 |
| PLAN-04 | Exam-switch knowledge-gap report | 6 |
| PLAN-05 | Optional calendar read/write scopes | 4 |
| SR-01 | Deterministic reviewed scheduling engine | 2 |
| SR-02 | New-card and workload limits | 2 |
| SR-03 | Cloze, image and explanatory cards | 2 |
| SR-04 | AI draft versus editorial trust labels | 2 |
| SR-05 | Duplicate/sibling handling | 2 |
| SR-06 | Offline reviews and synchronized history | 3 |
| SR-07 | Authorized import/export compatibility | 3 |
| LIB-01 | Versioned articles and references | 1 |
| LIB-02 | Hybrid search with visibility filters | 2 |
| LIB-03 | Page/figure/timestamp citations | 2 |
| LIB-04 | Guideline country/date overlays | 2 |
| LIB-05 | Source-change propagation | 2 |
| LIB-06 | Rights-checked document imports | 2 |
| LIB-07 | Table/image/extraction completeness reports | 2 |
| LIB-08 | Media player, captions and chapters | 2 |
| LIB-09 | Licensed offline media packages | 3 |
| NOTE-01 | Source-linked private notes | 1 |
| NOTE-02 | Concepts, backlinks and collections | 2 |
| NOTE-03 | Human-controlled revisions and portable export | 2 |
| SIM-01 | Versioned fictional/approved case scripts | 5 |
| SIM-02 | Separate patient and examiner contexts | 5 |
| SIM-03 | Voice/text with transcript uncertainty | 5 |
| SIM-04 | Rubric evidence per criterion | 5 |
| SIM-05 | Authoritative state transitions and timers | 5 |
| SIM-06 | Counterfactual replay and debrief timeline | 5 |
| SIM-07 | Human review/appeal for consequential use | 5 |
| SIM-08 | Team-based cases and handover | 6 |
| IMG-01 | Rights-checked still-image case library | 2 |
| IMG-02 | Stack viewer and reviewed annotations | 5 |
| IMG-03 | DICOM privacy and pixel-integrity workflow | 5 |
| IMG-04 | Anatomy/clinical-image linkage | 5 |
| IMG-05 | Low-device-capability fallbacks | 3 |
| INST-01 | Programs, cohorts and assignments | 4 |
| INST-02 | Distinct faculty and institution workspaces | 4 |
| INST-03 | SSO and scoped enrollment integrations | 4 |
| INST-04 | Curriculum mapping and coverage | 4 |
| INST-05 | Assessment author/reviewer/publisher separation | 4 |
| INST-06 | LTI and selected QTI interoperability | 4 |
| INST-07 | Privacy-preserving cohort analytics | 4 |
| ADMIN-01 | Real cross-tenant owner dashboard | 1 |
| ADMIN-02 | Content and rights operations | 1 |
| ADMIN-03 | AI model/cost/policy administration | 1 |
| ADMIN-04 | Support, incidents and audit trails | 1 |
| ADMIN-05 | Revenue, licenses, royalties and renewals | 2 |
| COMMUNITY-01 | Moderated groups and discussions | 4 |
| COMMUNITY-02 | Permitted shared content and private challenges | 4 |
| COMMUNITY-03 | Optional gamification without coercive defaults | 4 |
| CAREER-01 | Longitudinal learning portfolio | 4 |
| CAREER-02 | Human-supervised feedback/sign-off | 5 |
| CAREER-03 | Continuing-education records and provider workflow | 7 |
| OFF-01 | Signed resource manifests and resumable downloads | 3 |
| OFF-02 | Idempotent event reconciliation | 3 |
| OFF-03 | Note conflicts and versioned plan resolution | 3 |
| OFF-04 | Offline entitlement and freshness disclosure | 3 |
| OPS-01 | GitHub Actions development compute enforcement | 0 |
| OPS-02 | Runtime boundary approval before deployment | 0 |
| OPS-03 | Signed releases and reversible migrations | 1 |
| OPS-04 | Recovery drills and failure-mode monitoring | 2 |
| TRUST-01 | No fake scores, charts, citations or active-agent states | 1 |
| TRUST-02 | Privacy, deletion and export workflows | 1 |
| TRUST-03 | Prompt-injection and tenant-isolation tests | 1 |
| TRUST-04 | Clinically reviewed shared medical content | 1 |
| TRUST-05 | Validated readiness before predictive claims | 7 |
| TRUST-06 | Accessibility and device-matrix testing | 1-3 |
| TRUST-07 | Provenance retained through derived resources | 2 |
| CORE-07 | Sign-in methods, in-app account deletion, device limit and single active session [NEW] | 1 |
| CORE-08 | Notification policy, push and in-app inbox [NEW] | 1 |
| CORE-09 | Guest trial before sign-up (SHOULD) [NEW] | 1 |
| CORE-10 | Navigational hierarchy mapped to concept identities [NEW] | 1 |
| UX-01 | Touch-first session workspace: gestures, tool tray, navigator, Focus Mode [NEW] | 1 |
| UX-02 | Desktop and web keyboard map and fullscreen [NEW] | 1 |
| UX-03 | Client performance budgets on reference devices [NEW] | 1-3 |
| QB-11 | Two to ten options with generated labels [NEW] | 1 |
| QB-12 | Qbank builder: hierarchy multi-select, four pools, counts, availability rule, presets, Quick 10 [NEW] | 1 |
| QB-13 | Session tools baseline: calculator, converter, text size, hint, auto-submit warnings, submission summary [NEW] | 1 |
| QB-14 | Key learning point, exam tip, high-yield flag, authored plus empirical difficulty [NEW] | 1 |
| QB-15 | Community statistics with minimum sample and expected-score comparison [NEW] | 2 |
| QB-16 | Psychometric screening defaults and issue-report SLA [NEW] | 2 |
| QB-17 | Session results with time and answer-change analysis and result actions [NEW] | 1-2 |
| EX-07 | Administrator-configured mock tests, types and results [NEW] | 2 |
| EX-08 | Monotonic client timer, grace windows, integrity signals and per-test policy [NEW] | 2 |
| AI-17 | Transparent baseline estimator, default selection policy and difficulty fallback [NEW] | 1 |
| AI-18 | Pre-generated one-tap tutoring, cached and available offline [NEW] | 2 |
| SR-08 | Automatic question re-test queue with objective grading and family-variant preference [NEW] | 2 |
| SR-09 | Editorial key-point cards [NEW] | 2 |
| PROG-01 | Hierarchy drill-down analytics, difficulty and trend filters, mastery heat-map [NEW] | 2 |
| ENG-01 | Daily goal, streak with freezes, question of the day; all can be disabled [NEW] | 1 |
| ENG-02 | XP, achievements and weekly recap [NEW] | 2 |
| ENG-03 | Widgets and lock-screen mock timer (COULD) [NEW] | 3 |
| COMP-01 | Daily, weekly, monthly and live competitions; same-exam population; opt-in handle [NEW] | 4 |
| COMP-02 | Scoring with guess penalty, capped speed bonus and tie-breaks [NEW] | 4 |
| COMP-03 | Leagues and duels [NEW] | 4 |
| COMP-04 | Anti-cheat, integrity review and prizes only after review [NEW] | 4 |
| COM-01 | Upgrade triggers and free allowance inside the 7C tiers [NEW] | 1 |
| COM-02 | Store billing plus web checkout with local wallets; one server-side entitlement service [NEW] | 1-3 |
| COM-03 | Coupons, referrals, regional price tiers, trials and pause [NEW] | 2 |
| COM-04 | Proposals pending approval: 'until my exam' and pass extension [NEW] | - |
| GROW-01 | Share cards, duel links, deferred deep links and ambassador codes [NEW] | 4 |
| GROW-02 | Astro site: per-exam pages, pricing, checkout, help, legal and app-link association files [NEW] | 1 |
| PROT-01 | Capture protection per platform and text-surface watermark [NEW] | 1-3 |
| PROT-02 | Device attestation, anti-scraping limits, pack encryption and offline lease [NEW] | 3 |
| PROT-03 | Store and platform compliance checklist [NEW] | 1-3 |
| ARCH-01 | Shared Rust core across server, Tauri and WebAssembly [NEW] | 0-1 |
| ARCH-02 | TypeScript contracts generated from Rust types [NEW] | 1 |
| ARCH-03 | Owned native Tauri plugins in Swift and Kotlin [NEW] | 0-3 |
| ADMIN-06 | Editorial console baseline: hierarchy, questions, bulk import with dry run and rollback, settings [NEW] | 1-2 |
| OPS-05 | Product-analytics taxonomy, experimentation, remote config and kill switches [NEW] | 1 |
| OPS-06 | Staged rollout, forced and soft update, two-version API compatibility [NEW] | 3 |
| OPS-07 | Rust and Tauri CI runner matrix under the compute rule [NEW] | 0 |

## 30. Release acceptance criteria

### 30.1 Core release gates

A real reviewed question can be answered, submitted, explained, linked to its source, converted into an appropriate learning intervention, and reflected in the next plan. The entire flow must use real persisted data.

No fabricated analytics, placeholder integrations, dead buttons or permission bypasses. The agent cannot mutate protected tasks, trigger purchases, publish medical content or expose another tenant's records in the release test suite.

A forced retry cannot duplicate an answer, charge, review or plan action. A failed provider call cannot erase submitted work. A cancelled job cannot later apply a stale plan.

All shipped shared medical content has accountable review and a current rights record. Any unresolved critical safety or data-isolation defect blocks release.

### 30.2 Proposed operational targets, not measured achievements

Use an initial target of 99.9% monthly availability for core online study services, API p95 below 500 ms for ordinary non-AI operations under the agreed load, and first usable screens within 2.5 seconds on the agreed mid-range device/network profile. AI and media latency receive separate budgets.

Define and test backup recovery targets before paid production; an initial planning target might be a 15-minute recovery point and four-hour recovery time, subject to infrastructure approval and affordability. Never advertise these targets as achieved before load and recovery testing.

**[NEW] Proposed client budgets, not measured achievements.**

| Metric | Proposed budget |
|---|---|
| Cold start to interactive | <= 2.5 s on the reference mid-range Android device; <= 1.5 s on a recent iPhone and on desktop |
| Next question | <= 100 ms from local data |
| Scrolling and taps | 60 frames per second; no visible delay on option tap in the system WebView |
| Base mobile download | <= 40 MB before packs |
| Memory | Stable on a 2-3 GB RAM Android device through a 200-question mock |
| Battery | <= 8% per hour of active study on the reference device |
| Crash-free sessions | >= 99.8% |
| Android not-responding rate | < 0.2% |
| Sync | 100 queued events uploaded in <= 2 s on a 3G connection |
| Answer and sync endpoints | p95 <= 300 ms, inside the general 500 ms target above |

### 30.3 Learning-quality release tests

Unseen variants should not be misclassified as independent when they share a known content family. Assisted practice must not inflate independent readiness. A suspected ambiguous item triggers investigation rather than automatic learner blame. A changed source propagates to dependent materials. A deactivated license prevents further unauthorized retrieval.

An OSCE result must show evidence for assessed criteria and 'not assessed' for unavailable observations. A cloud-AI outage must leave downloaded non-AI study intact.

**[NEW] Added tests.** A five-option and an eight-option question import, render, score and show community statistics correctly. Changing the device clock never extends a timer, and auto-submit fires offline. A chapter with fewer than ten independent attempts never shows a weakness band. A self-built session shows an expected-score comparison and no percentile. A purchase on one platform unlocks the others, an expired subscription locks paid features at the next sync, and an expired offline lease is enforced. Screenshots are blocked on Android, capture is obscured on iOS, desktop windows are content-protected, the watermark never covers a medical image, and a pack cannot be read on another device. No learner appears on a leaderboard without opting in. No numerical readiness appears anywhere before section 24 validation.

### 30.4 Validation beyond engineering

Use clinical expert review, representative usability testing and a prospective learning evaluation. Compare the adaptive loop with a reasonable fixed-plan baseline using delayed unseen assessment, not just immediate satisfaction or the same practiced questions. Plan appropriate consent and ethics review for research use.

### 30.5 Client test matrix [NEW]

In addition to the synchronization cases in section 22: a device matrix of low-end, mid-range and flagship Android, small and large iPhone, iPad and Android tablet, Windows and macOS, the supported browsers, the oldest and newest supported operating systems and the minimum Android System WebView version; networks that are offline, throttled 2G and 3G, flapping, and behind a captive portal; interruptions by incoming call, termination, reboot, power loss, operating-system background kill, time-zone change and low storage during practice, mock and competition; local database migration across app upgrades; purchase flows for buy, restore, renew, cancel, refund, grace period, coupon and cross-platform entitlement; screen-reader and 200% text runs of every core flow; and a closed beta with real learners before each public release.

## 31. Agentic development execution contract

The implementation agent must inspect actual repository and deployment evidence before changing architecture. It must not infer that an attractive screen proves a working backend.

For each implementation slice, require: requirement IDs -> data contract -> backend behavior -> UI integration -> permissions -> failure states -> tests -> evidence -> documentation. Implement one complete vertical flow before proliferating disconnected modules.

Use specialist agent roles for design, backend, assessment logic, security and tests when useful, with bounded tasks and a single accountable integration owner. No uncontrolled recursive delegation or repeated blind repair loops.

Do not change medical answer keys, delete tests, weaken policy gates, suppress failed checks, or mark a task complete to make CI green. Clinical and rights decisions route to the correct human owner.

All computation must follow the approved CI/runtime boundary. Local file inspection and editing do not authorize local builds or tests. Protect secrets, avoid production data in fixtures, pin dependencies appropriately and preserve audit evidence.

Completion reports must distinguish implemented, tested, partially implemented, blocked and deliberately deferred. A mocked third-party integration is not production integration.

### 31.1 Rust and Tauri guardrails for implementation agents [NEW]

- Rust types are the contract. Never hand-edit generated TypeScript types or the generated API client.
- Business rules for sessions, timers, scoring, scheduling, entitlements and sync live in the shared crates. They are never re-implemented in Svelte components.
- No `unsafe` without a recorded review. No panicking calls in request or sync paths. Errors are typed and mapped to the single API error format.
- SQL goes through compile-time-checked queries and service methods. Every migration has a tested rollback path.
- Tauri commands are narrow, validated and capability-scoped. The webview never receives secrets, pack keys or unreleased answer keys.
- A change to a shared crate must pass native and WebAssembly builds before merge.
- Native plugin code in Swift and Kotlin is owned code with its own tests and review, not copied snippets.
- The Astro site holds no learner data and calls only public or checkout API endpoints.

## 32. Decisions still required before production commitment

These do not undo the approved scope. They are deliberately left open rather than guessed.

| Decision | Why it matters | Proposed handling |
|---|---|---|
| Final brand and operating entity | Contracts, app accounts, payments and trademarks | Resolve in Phase 0. |
| Existing repository versus greenfield | Reuse, compatibility and delivery cost | Audit the actual project before choosing. |
| First fully supported exam pack | Rights, editorial capacity and validation population | Compare demand and available lawful content; activate one complete pilot pathway first. |
| Budget, team and content commitments | Schedule, licensing and operating costs | Set a written envelope before promising dates or subscription prices. |
| Live runtime under the compute policy | The application cannot be a permanent CI job | Obtain explicit approval for a separate serving/inference environment; no silent exception. |
| Licensed providers and actual integration access | Determines what can legally ship | Contract first; use original/open/permitted alternatives otherwise. |
| Pre-medical entrance packs such as MDCAT [NEW] | Brings under-18 learners and non-medical subjects outside decision 1C | Decide in Phase 0. If approved, apply the under-18 privacy defaults before launch. |
| Tauri mobile go/no-go [NEW] | Native plugin maturity and WebView performance on low-end Android are the main technical risks | Decide on the Phase 0 spike report. The fallback changes only the mobile shell. |
| Payment routes per storefront [NEW] | Store rules on external payment differ by country and change | Legal and policy check per market before building the paywall. |
| Free allowance, regional prices, 'until my exam' and pass extension [NEW] | Revenue, trust and compliance with 2.3 | Approve, amend or drop each proposal in section 26.1. |
| Competition prizes [NEW] | Legal, tax and cheating exposure | Propose none at launch; recognition only until integrity review is proven. |
| Running CI-built artifacts on developer-owned devices and simulators [NEW] | Determines the mobile debugging loop under section 27 | Owner to state whether this is inside or outside the compute rule. |
| Desktop distribution channel [NEW] | Direct signed download with updater versus the Microsoft and Mac stores; affects payments and review | Propose direct signed distribution first. |
| Hosting region and data residency [NEW] | Latency for target markets and legal transfer rules | Decide together with the runtime approval above. |
| Companion files [NEW] | AGENT_IMPLEMENTATION_HANDOFF.md and README.md were not supplied to this merge and still describe the version 1.0 stack | Update both to decisions 9-11 before any implementation agent is started. |

**Final product test:** The learner should see a calm, focused experience while the system does the difficult work of connecting evidence, priorities and resources. The complexity belongs in the implementation, not on the home screen.

---

## Appendix A - Merge ledger [NEW]

### A.1 Changes to version 1.0

Only two passages of version 1.0 text were replaced, both by owner decision 9: the section 20.1 stack list and the section 20.4 repository tree. The version line in the header changed from 1.0 to 2.0, and the 20.1 and 20.4 headings gained the [AMENDED] tag. Everything else below is an addition. This was verified by a line-by-line comparison against the version 1.0 file.

| Location | Change | Origin |
|---|---|---|
| Header, How to use | Version 2.0, supersedes line, merge reading note | Merge |
| 1 | Decisions 9-13 appended | Owner, 17 September 2026 |
| 1.1 | Merge defaults | Proposed by the merge; not objected to |
| 2.4 | Health metrics and initial targets | Quiz LMS v2.0 section 2, reconciled with 2.2 |
| 3 | Four benchmark rows | Quiz LMS v2.0 section 3 |
| 4 | Six gap rows | Quiz LMS v2.0 section 1.2 |
| 5.1, 5.2, 5.5 | Quiz LMS pathway families; template fields; navigational hierarchy | Quiz LMS v1.0 sections 1 and 4; v2.0 sections 11 and 15 |
| 6, 6.1, 6.3, 6.4 | Feature placement; onboarding additions; accounts; notification policy | Quiz LMS v2.0 sections 8, 9 and 22 |
| 7.5 | Touch-first session workspace; keyboard map and fullscreen restored | Quiz LMS v2.0 section 12; v1.0 sections 38-56 |
| 8.8 | Estimator, selection defaults, difficulty fallback, mastery index, error tag | Quiz LMS v2.0 section 17; v1.0 sections 26-32 and 72 |
| 9.3, 9.5 | Two autonomy rows; pre-generated tutoring | Quiz LMS v2.0 section 19.4 |
| 11.1-11.4 | Extra question fields; session vocabulary; client timer; screening defaults | Quiz LMS v2.0 sections 11, 13, 28 and 35 |
| 11.5-11.9 | Qbank builder; session tools; mock tests; community statistics; session results | Quiz LMS v1.0 sections 9-19 and 57-62; v2.0 sections 14, 15 and 19 |
| 13 | Question re-test queue; key-point cards; defaults | Quiz LMS v2.0 section 18 |
| 16 | Store review note on calculators | Quiz LMS v2.0 section 40 |
| 17.1, 17.2 | Competition module; engagement mechanics | Quiz LMS v1.0 sections 20-24; v2.0 sections 16 and 22 |
| 18.4 | Product-health views | Quiz LMS v2.0 section 29 |
| 19.3, 19.4, 19.5 | Author is not approver; report SLA; editorial console baseline | Quiz LMS v1.0 sections 76 and 80-87; v2.0 sections 28-29 |
| 20.1, 20.4 | **Replaced** by the Rust, Tauri, SvelteKit and Astro stack | Owner decisions 9-11 |
| 21 | Added domains, API examples, conventions, analytics-stream note | Quiz LMS v2.0 sections 33-34 |
| 22 | Four capability rows; packs; conflict rules; surface differences | Quiz LMS v2.0 sections 25 and 32 |
| 24, 24.1 | Decision 12 note; hierarchy analytics | Owner; Quiz LMS v1.0 sections 63-69 |
| 25.1, 25.2 | Content protection; store compliance | Quiz LMS v2.0 sections 36 and 40 |
| 26.1 | Commercial mechanics and growth | Quiz LMS v2.0 section 6 |
| 27.1 | Stack consequences under the compute rule | New stack |
| 28.1 | Phase additions | Quiz LMS v2.0 section 43, re-sequenced onto Phases 0-7 |
| 29 | 44 new requirement IDs | Merge |
| 30.2, 30.3, 30.5 | Client budgets; added tests; client test matrix | Quiz LMS v2.0 sections 37, 41 and 45 |
| 31.1 | Rust and Tauri guardrails | New stack |
| 32 | Nine decision rows | Merge |
| Source register | S44-S51 | Research of 17 September 2026 |

### A.2 Version 1.0 content withdrawn, by owner decision only

| Item | Reason |
|---|---|
| TypeScript backend | Decision 9: Rust backend and APIs. |
| Capacitor for iOS and Android | Decision 9: Tauri for mobile. Retained only as the named fallback in 20.1. |
| Version 1.0 repository tree | Follows from decisions 9-11. Every package name is carried into the new tree. |

Nothing else from version 1.0 was removed.

### A.3 Quiz LMS v2.0 content deliberately not carried over

| Item | Reason |
|---|---|
| Early predicted score and pass probability | Deferred to Phase 7 by decision 12. |
| Flutter recommendation | Superseded by decision 9. |
| 'Mobile app only' scope and removal of desktop layouts | Superseded by decisions 9-10. |
| Five tabs of Today, Practice, Compete, Progress, Library | Replaced by the section 6 destinations; features relocated, none dropped. |
| Free, Pro and All-Access tiers | Replaced by the 7C tiers. |
| Streak-holding target for weekly active learners | Conflicts with 2.2. Streak length is monitored only. |
| Standalone AI tutor specification | Absorbed by the Coach (sections 8-9). Only pre-generated prompts were added. |
| Separate roles list, content workflow, versioning, psychometrics narrative, offline sync core, study planner and spaced-repetition core | Already present in version 1.0 in stronger form. Only the differences were added. |
| Support or moderator role; AI-assisted authoring rules | Duplicates of 18.1 and 19.3. |
| Flashcard 'study chapter' summaries built from key points | Superseded by the connected library of section 12. |

## Appendix B - Product-analytics event taxonomy [NEW]

Separate from the learning-evidence events of 21.3. Keyed by a pseudonymous ID. User properties: exam, plan, days to exam, cohort.

| Event | Key properties |
|---|---|
| app_open | source, deep_link, surface (web, desktop, ios, android) |
| onboarding_step_completed | step |
| baseline_completed | duration |
| activation_achieved | hours_since_first_use |
| session_started | preset, pool, source (today, builder, smart, review), question_count, offline |
| question_answered | correct, difficulty, seconds, confidence, assisted, offline |
| explanation_viewed | seconds |
| session_submitted | score, duration, auto_submitted |
| review_completed | items, retention |
| plan_revision_viewed, plan_revision_undone | automatic, reason_code |
| coach_turn | prompt_type (pre-generated or free-form), rating |
| daily_goal_met, streak_extended, streak_lost | streak_length |
| mock_completed | template, score, percentile |
| progress_viewed | level |
| paywall_viewed | trigger, variant |
| purchase_completed | product, route, price_tier |
| subscription_renewed, subscription_cancelled | product, tenure |
| pack_downloaded | size, network_type |
| sync_completed | events, duration, failures |
| notification_opened | category, campaign |
| question_reported | reason |
| competition_joined, duel_created, duel_completed | type, result |
| share_card_sent | type, channel |
| integrity_event | type, test_type |

## Appendix C - Quiz LMS v1.0 coverage map [NEW]

Shows where each part of the original 169-section Quiz LMS document now lives, so nothing from it is silently lost.

| Quiz LMS v1.0 sections | Topic | Location in this plan |
|---|---|---|
| 1-3, 165-169 | Overview, vision, shared engines, core product rule, journey, system map | Sections 2.1, 6.2 and 20 |
| 4 | Exam, subject, system, chapter hierarchy | 5.5 |
| 5 | Roles | 18.1 |
| 6-7 | Authentication, onboarding | 6.1, 6.3 |
| 8 | Candidate dashboard | 6 and 7.2 (Today) |
| 9-14 | Random Quiz Generator, four pools, counts, difficulty | 11.5 |
| 15-19 | Mock tests, settings, blueprint, types, results | 5.2, 11.3, 11.7 |
| 20-24 | Competition, types, head-to-head, scoring, leaderboard | 17.1 |
| 25-32, 70-72, 136-137 | Adaptive rules, mastery levels, recommendations, Smart Practice | 8.4-8.8 |
| 33-37 | Exam, Practice, Review and Revision modes | 11.2 |
| 38-41, 56 | Quiz screen, header, navigator, navigation, submission confirmation | 7.5 |
| 42, 55 | Fullscreen, keyboard shortcuts | 7.5 (restored for desktop and web) |
| 43-44 | Medical calculator, unit converter | 11.6, 16 |
| 45-51 | Highlighting, text size, hint, notes, dark mode, elimination, marking | 7.3, 11.2, 11.6 |
| 52-54, 115-117 | Autosave, network recovery, auto-submit, timer | 11.3, 11.6, 22 |
| 57-62 | Results, community statistics, explanations, result actions | 11.1, 11.8, 11.9 |
| 63-69 | Analytics by level, difficulty and trends | 24, 24.1 |
| 73-76 | Question fields, difficulty, status workflow, reporting | 11.1, 19.3, 19.4 |
| 77-79 | Previous quizzes, marked questions, notes library | 6, 12.5 |
| 80-87, 130, 132 | Administration, bulk import, settings | 18.4, 19.5 |
| 88, 138-139 | Community statistics rules, comparison, percentile | 11.8 |
| 89-114 | Data model and APIs | 21 |
| 118 | Quiz security | 11.3, 25.1 |
| 119-123 | Performance, scalability, caching, jobs, security | 20, 25, 30.2 |
| 124-127 | Privacy, accessibility, responsive design, design principles | 7, 25 |
| 128-129 | Navigation | 6 |
| 131 | Notifications | 6.4 |
| 133-135 | Question quality, bad-question detection, revision tracking | 11.4, 13 |
| 140-141 | Reporting, audit log | 19.5 |
| 142-152 | Testing and acceptance criteria | 30 |
| 153-163 | Phases and MVP | 28, 28.1 |
| 164 | Future features | Promoted: spaced repetition, flashcards, AI explanations, institutions, video, learning certificates. Still out of scope: parent dashboards and unreviewed AI-generated questions. |

---

## Source register

All sources below were checked during this research on 17 September 2026. Product pages are evidence of public product descriptions, not independent performance or licensing verification. Official examination/standards documentation takes precedence over competitor pages for formats and requirements.

- **S01** - iMD homepage: https://www.imdmedicaldoctor.com/
- **S02** - iMD resource catalog: https://www.imdmedicaldoctor.com/resources
- **S03** - QBankly public feature page: https://qbankly.app/
- **S04** - Coursology QBank: https://coursologyqbank.app/
- **S05** - StepwiseMD: https://stepwisemd.net/
- **S06** - UWorld USMLE product hub: https://medical.uworld.com/usmle/
- **S07** - AMBOSS product features: https://www.amboss.com/us/features
- **S08** - USMLE-Rx: https://usmle-rx.com/
- **S09** - Osmosis AI: https://www.osmosis.org/features/osmosis-ai
- **S10** - Osmosis flashcards: https://www.osmosis.org/features/flashcards
- **S11** - Lecturio medical AI tutor: https://www.lecturio.com/medical/ai-tutor/
- **S12** - Lecturio AI Educator Suite: https://www.lecturio.com/inst/ai-educator-suite/
- **S13** - USMLE Fighter features: https://www.usmlefighter.com/qbank-features/
- **S14** - StepUp USMLE: https://stepupusmle.com/
- **S15** - MDSteps: https://mdsteps.com/
- **S16** - MedLumen: https://medlumen.io/
- **S17** - TopQBank: https://www.topqbank.com/
- **S18** - Anki deck options and FSRS: https://docs.ankiweb.net/deck-options.html?highlight=preset
- **S19** - Complete Anatomy: https://www.us.elsevierhealth.com/complete-anatomy
- **S20** - Body Interact virtual-patient simulator: https://bodyinteract.com/virtual-patient-simulator/
- **S21** - Geeky Medics virtual-patient simulator: https://geekymedics.com/virtual-patient-simulator/
- **S22** - Geeky Medics virtual examiner: https://support.geekymedics.com/en/article/how-do-i-use-the-virtual-examiner-1glbo3s/
- **S23** - QBankly terms: https://qbankly.app/terms
- **S24** - USMLE 2026 Step 1/2 software update: https://www.usmle.org/test-delivery-software-updates-step-2-ck-and-step-1-coming-may-2026
- **S25** - Current MCCQE overview: https://mcc.ca/examinations-assessments/mccqe/
- **S26** - W3C WCAG 2.2 target-size guidance: https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html
- **S27** - OWASP excessive agency: https://genai.owasp.org/llmrisk/llm062025-excessive-agency/
- **S28** - NBME item-writing guide: https://www.nbme.org/institutions/nbme-item-writing-guide/
- **S29** - ACCME accreditation process: https://accme.org/accredited-providers
- **S30** - LTI Advantage implementation guidance: https://www.imsglobal.org/spec/lti/v1p3/impl/
- **S31** - QTI specifications: https://www.1edtech.org/standards/qti/index
- **S32** - SvelteKit introduction: https://svelte.dev/docs/kit/introduction
- **S33** - Capacitor documentation: https://capacitorjs.com/docs
- **S34** - Tauri and SvelteKit: https://v2.tauri.app/start/frontend/sveltekit/
- **S35** - PostgreSQL row-security policies: https://www.postgresql.org/docs/current/ddl-rowsecurity.html
- **S36** - Apple App Review Guidelines: https://developer.apple.com/app-store/review/guidelines/
- **S37** - GitHub Actions limits: https://docs.github.com/en/actions/reference/limits
- **S38** - MCCQE question structure: https://mcc.ca/examinations-assessments/mccqe/exam-day/multiple-choice-questions/
- **S39** - USMLE Step 3/software transition: https://www.usmle.org/usmle-test-delivery-software-updates-coming-2026
- **S40** - USMLE.Study, limited readable page content in this audit: https://usmle.study/
- **S41** - Medora, limited public-page audit: https://medorasmle.com/
- **S42** - MedicoSpira, limited readable page content in this audit: https://www.medicospira.com/
- **S43** - NextStepMD, limited readable page content in this audit: https://mynextstepmd.com/
- **S44** - UWorld Medical App Store listing and reviews: https://apps.apple.com/us/app/uworld-medical-exam-prep/id991621303
- **S45** - AMBOSS mobile apps: https://www.amboss.com/us/mobile-app
- **S46** - AMBOSS student overview, including the score-predictor description: https://www.amboss.com/us/students/student-life
- **S47** - MedAngle Super App Google Play listing: https://play.google.com/store/apps/details?id=com.medangle.eros&hl=en
- **S48** - PreMed.PK: https://www.premed.pk/
- **S49** - Community Tauri remote-push plugin, noting that the official plugin is local-only: https://github.com/yanqianglu/tauri-plugin-mobile-push
- **S50** - Community Tauri purchases plugin: https://github.com/spicavi/tauri-plugin-purchases
- **S51** - Tauri official plugins workspace and platform-support table: https://github.com/tauri-apps/plugins-workspace

Astro, Axum, SQLx and the Rust FSRS implementation were not audited in this research. They are named as candidates for Phase 0 verification, including license review.
