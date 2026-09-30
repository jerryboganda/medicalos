# Owner inputs and remaining external gates — 2026-09-30 (supersedes the 2026-09-25 sheet)

The engineering-ready, code-addressable tail is empty: every open ledger ID
is gated on one of the decisions below or an external party. Each decision
lists its option set, the safe default already shipped, and exactly what
answering it unlocks. Answer in one pass (a letter per row is enough); each
answer converts directly into a scoped slice.

## Decision sheet

| # | Decision | Options | Safe default while unanswered (shipped) | Unlocks when answered |
|---|---|---|---|---|
| 1 | Production IdP hosting (Zitadel) | Zitadel Cloud / on-VPS container / dedicated host | Code deployed, `ZITADEL_*` deploy plumbing live, endpoint honestly all-false | Provision production Zitadel, set `VPS_ZITADEL_*` secrets, platform sign-in goes live (CORE-01/CORE-07 tails) |
| 2 | Email provider (delivery seam) | Provider choice + account | Password accounts work; operator-assisted reset shipped; self-service reset/verification deferred | Self-service password reset + email verification against the delivery adapter (CORE-07 tail, auth-hardening spec) |
| 3 | Off-site backup destination + restore-drill approval | Any rclone backend (S3/B2/rsync-over-ssh); approve running the drill | `backup-offsite.sh` + written drill in the runbook, honestly marked not executed | Configure the remote, schedule the timer, execute the drill — its measured time becomes the real RTO (ops tails) |
| 4 | HSTS enablement | Enable at 15552000 now / wait for pilot stability | Deliberately unset (sticky; can lock visitors out on cert failure); runbook documents the steps + raise path | One NPM proxy-host patch (docs/deployment/vps-medicalos.md) |
| 5 | Telemetry collector | Collector choice + endpoint | OTLP export coded behind `OTEL_EXPORTER_OTLP_ENDPOINT`; production stays fmt-only | Set the env in deploy plumbing; spans flow; alerting becomes collector-side work |
| 6 | Payment route / provider for web checkout | Per-market route + provider account (§26.1) | No checkout anywhere; pricing page honest "to be announced" | COM-02 build (checkout API + entitlement service link), COM-03 |
| 7 | Store developer accounts | Apple/Google accounts + signing identities | Store route blocked (PROT-03), OPS-03 native installer signing deferred | Store build + compliance work alongside the Tauri shells |
| 8 | Clinical reviewers | Named reviewers + review workflow sign-off | TRUST-04 / CAREER-02 blocked | First reviewed exam pack + review-assignment workflow (TRUST-04, CAREER-02) |
| 9 | Reference test devices | Device list + budget | UX-03 / TRUST-06 blocked | Device-budget + accessibility matrix execution (UX-03, TRUST-06) |
| 10 | Site deployment origin | Subdomain on the existing VPS (recommended) / separate host | Site built + CI-gated, unserved | Serve `apps/site` at the chosen origin (GROW-02 tail) |
| 11 | SIM-03 voice model | Model-provider decision | Text-mode OSCE shipped; voice adapters owner-gated | Voice adapters for scenario runs (SIM-03 tail) |
| 12 | Tauri shells + app IDs | Shell work authorization, app identifiers, deep-link domains | ARCH-03 not started; UX-01/EX-08 native tails, ENG-01/ENG-03 remote push, PROT-01/02 native controls deferred behind it | The native workstream (ARCH-03, UX-01, EX-08, ENG-01/03 push, CORE-08 push tokens, PROT-01/02 native, OPS-03 installer signing) |
| 13 | **UI authorization for new surfaces** | Lift the standing no-UI constraint for specific scoped surfaces / keep frozen | LTI launch handoff page + content-selection picker deferred (launches answer JSON; server/API complete) | LTI browser handoff page and the deep-linking content picker, built against the owner-locked design system |
| 14 | COM-04 business model | Per §32 / §26.1 | Blocked by owner decision | Tier/monetization definition feeding COM-02/03 detail |
| 15 | Learner token transport (owner review) | httpOnly-cookie migration (touches every client request path + CSRF) / keep localStorage | localStorage, documented in the readiness assessment | The httpOnly-cookie migration as a scoped, owner-approved slice |

## Non-decision gates (external parties, not answerable by the owner alone)

- **1EdTech certification** for the LTI 1.3 launch (needs a real campus LMS
  platform to certify against; the tool side is built and mocked-platform
  tested — INST-06).
- **QB-10 calibration record** — externally validated readiness/calibration
  evidence.
- **LIB-06/07 tails** — real license verification, malware scanning, OCR
  sandboxing (third-party services).
- **SR-08 "SLA timers"** — needs a written product definition (what an SLA
  means for retests) before it can be built; currently undefined anywhere.
- **CAREER-03 accreditation** — provider/accreditor workflow (Phase 7 gate).
- **CORE-04 tail** — moving the production pool onto
  `medos_tenant_viewer`-style least-privilege credentials (DBA step on the
  VPS; owner-visible production change).

## Historical verification log (2026-09-25/26) — superseded by the ledger

The Codex-lane notes and per-run verification records previously kept here
have moved to `docs/requirements/traceability.md`, the single source of
truth for evidence runs and current statuses. The 2026-09-25 counting note
in this file (38 non-complete, 22 narrative rows) described a stale
checkout; the current strict count is 121/152 tested/resolved with every
status in the declared vocabulary.
