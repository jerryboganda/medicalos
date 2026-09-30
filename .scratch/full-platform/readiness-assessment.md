# Production readiness assessment — 2026-10-01

The platform is not yet accepted for unrestricted real-student use. The full
152-ID scope is retained in requirements.md. That baseline has 31 explicitly
open IDs and 121 historically tested/resolved IDs, but the latter also contain
unfinished work and regressions. These counts are not a readiness percentage.
No defensible calendar estimate follows from equally weighting these rows.

The old 2026-09-27 completion plan is a historical snapshot. Its 75–90%
estimate, deployed-host assertions and older test totals are not current
acceptance evidence for this worktree or its intended Orca Desktop runtime.

## Verified improvements and limits

The exact 400ed80 commit passed all five GitHub Actions jobs: API tests against
Postgres, client/site builds, identity-provider provisioning and browser tests.
It includes session/device revocation, LTI configuration ownership, guarded
JWKS transport, accurate Coach engine labels and mobile keyboard navigation.
Readiness and verified backup/restore changes passed on e70ba45. Later content
publication, browser device registration and Account controls need the newer
exact-source run. See ci-evidence.md for successful and unsuccessful runs.

Recovery testing uses a disposable CI database and synthetic data. Software
tests do not establish lawful content, clinical correctness, vendor contracts,
native-device behavior or student acceptance. No deployment or live VPS
inspection has been performed in this work scope.

## Engineering still required

| Workstream | Remaining work | Acceptance needed |
|---|---|---|
| Learning evidence | Replace client-reported re-test correctness with single-use submitted answer receipts; finish queue delivery/SLA behavior | Real API grading, replay, race, variant, assistance and confidence cases |
| Content safety | Complete article review/rights workflow, source propagation, trusted ingestion and licensed media coverage; verify all publication transitions atomically preserve review state | Independent review provenance, denied invalid grants, extraction quality and quarantine cases |
| Identity/privacy | Complete the account archive and policy-driven retention/erasure; finish configured identity methods and device-cap recovery | Every user-owned data category, account isolation, session retirement and approved policy |
| Institutional work | Complete LTI browser/deep-link/content handoff and integration acceptance; check remaining enrollment and least-privilege application-pool work | Real tenant/provider handoff and denial of foreign data |
| AI/planning | Implement real provider routing, bounded cost/policy controls, evaluations and supported languages; complete calendar scopes | Provider failure/fallback, evidence grounding, permission and evaluation gates |
| Native clients | Complete owned desktop/mobile shells, push, attestation, secure/offline storage, billing and background behavior | Signed builds and supported real-device matrix |
| Imaging/imports | Complete binary parsing, malware/OCR isolation, DICOM privacy/pixel integrity and low-capability fallbacks | Hostile-file tests and lawful reference fixtures |
| Commercial/career | Complete approved payment route, pricing/referrals, revenue/license operations, accreditation and provider workflow | Genuine vendor transactions, renewals, records and external calibration |
| Operations | Finish exact-source release acceptance, least privilege, secrets/provider configuration, signed releases and actual runtime recovery evidence | Orca deployment verification plus approved backup/recovery and support procedures |
| Design/accessibility | Continue every screen within design.md; verify account and remaining features in existing themes, phone/desktop sizes and keyboard flows | CI gallery inspection, accessibility and supported-device performance evidence |

Each workstream includes existing historical-ID tails; nothing is removed
because a ledger row says tested. The re-test endpoint's correctness claim and
the export's partial category coverage demonstrate why source and real flows
must be checked beyond the label.

## External inputs and acceptance

Named reviewers and an approved first exam/content corpus, genuine rights,
provider accounts/credentials, store identities/signing, reference devices,
retention policy and external calibration/accreditation remain necessary.
Email was explicitly deferred in the September owner decision; this is not
evidence of an implemented self-service reset or email delivery service.
Engineering independent of these inputs continues.

Luna/max was used for completed inspection/implementation workers. Later
review workers failed with "Your workspace is out of credits"; those reviews
are incomplete. No Astra–Gemini/Antigravity substitute is used.

## Completion order

1. Close confirmed security, grading and recovery regressions with exact CI.
2. Complete the remaining web/API workflows, clinical provenance and privacy
   behavior, retaining each full requirement and its acceptance boundaries.
3. Finish provider/native/commercial features with genuine credentials and
   owned identities, then evaluate the supported device/content matrix.
4. Verify the accepted source on the authorized Orca runtime and conduct an
   approved student beta, clinical review, recovery and operational acceptance.

The project reaches production acceptance only after those gates are met.
Green CI for one batch does not close the full platform.
