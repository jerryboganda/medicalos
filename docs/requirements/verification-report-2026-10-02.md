# Medical OS verification report — 2026-10-02

## Verdict

Medical OS is not fully developed or demonstrated to be production ready.
The checked-out requirement ledger contains 152 IDs: 120 are `tested`, one
is `resolved`, 15 are `in-progress`, 14 are `not-started`, and two are
`blocked`. The 121 tested/resolved IDs represent 79.6% of ledger entries;
this is a status count, not a measure of production readiness or feature depth.

This report audits branch `codex/inst06-launch-handoff-ui`, based on main
`0a9793f`. It covers source and GitHub Actions evidence. No deployment,
Orca Desktop runtime acceptance, real campus LMS certification, or reference
device acceptance was performed in this development slice.

## Completed in this slice

- Browser LTI launch responses negotiate HTML while API callers retain JSON.
  A nonce-protected, non-cacheable document restores the verified application
  token without adding it to the navigation URL.
- Resource-link launches continue to an existing library article, with a safe
  fallback and a readable browser-storage error.
- Instructors can search published editorial articles, select according to the
  LMS's single/multiple settings, and post a signed deep-link response. Private
  documents are excluded. Cancellation submits an empty signed response.
- Nginx applies HTTPS framing only to the launch endpoint and LTI pages;
  ordinary app routes retain frame denial. The SPA fallback retains its LTI
  location policy. Configuration validation runs at container startup, after
  the API service hostname can resolve.
- Focused API and browser tests cover negotiation, script escaping, cancellation,
  article continuation, selection constraints, LMS return forms, and widths of
  320, 375, 414, and 768 pixels.

The changes reuse existing application and design-system surfaces and add no
dependencies. Direct Ponytail and standards/spec reviews found no remaining
actionable issue in the scoped change.

The Nginx fallback uses a file candidate before `=404`, keeping processing in
the LTI location; a final URI would cause an internal redirect. See the
[official try_files documentation](https://nginx.org/en/docs/http/ngx_http_core_module.html#try_files).

## Automated verification

Implementation CI acceptance: **passed all five jobs** in
[run 37038719199](https://github.com/jerryboganda/medicalos/actions/runs/37038719199)
on source commit `8f586d106ca178822b4bd33f0efba9b33148e686`.

The previous run
[37018375483](https://github.com/jerryboganda/medicalos/actions/runs/37018375483)
passed Rust, site, Zitadel, and browser E2E. It failed the new Nginx frame-policy
check, exposing the SPA fallback issue addressed in the final source commit.

| Gate | Evidence / scope |
|---|---|
| Rust workspace | 156 tests passed, including 128 existing API integration tests and the mocked-campus LTI integration test |
| Type export | 364 tests passed with the type-export feature; generated TypeScript drift checked |
| Database | All 63 migration pairs passed rollback/reapply; SQLx offline cache generated |
| Rust quality | Formatting, Clippy with warnings denied, dependency advisory/license checks |
| Shared core | WASM builds and nine calculation parity cases, zero failures |
| Browser | 77 Playwright tests passed against the CI app/fixtures |
| Client | Svelte build; Docker startup, LTI/API framing, ordinary-route denial; TRUST-01 audit |
| Site | Astro build |
| Identity provisioning | Zitadel provisioned twice; second pass created nothing |

The LTI browser tests initialize a synthetic launch session and intercept the
picker's API/LMS responses. The Rust LTI integration test verifies the server
against a mocked campus platform. Together they validate the scoped contracts;
they do not constitute a real LMS browser launch or 1EdTech certification.
The browser cases run as top-level pages; LMS iframe storage partitioning,
blocked storage, and navigation/reload behavior need campus-browser acceptance.
The container header test checks the launch location even when its test
upstream is unavailable; it is not a live campus launch test.

## All open ledger IDs

These are the 31 entries whose full requirement acceptance remains open. The
details and prior evidence are maintained in [traceability.md](traceability.md).
The owner decisions and external dependencies are maintained in
[the input sheet](../../.scratch/owner-inputs-requested.md).

| ID | Status | Remaining requirement or acceptance |
|---|---|---|
| ARCH-01 | in-progress | Shared-core native/WASM parity is verified; native Tauri shell and reference-device spike remain open |
| CORE-08 | in-progress | Remote push through an owned native plugin |
| ADMIN-02 | in-progress | Real content-license verification |
| TRUST-04 | not-started | Named clinical reviewers and reviewed shared content |
| UX-01 | in-progress | Native gesture polish |
| ENG-01 | in-progress | Remote push delivery; in-app reminder work has prior evidence |
| OPS-03 | in-progress | Native installer signing and deployment-side image verification |
| EX-08 | in-progress | Native clock resistance and competition-specific policy |
| SR-08 | in-progress | Retest SLA product definition and timers |
| LIB-06 | in-progress | Binary extraction, real license verification, retention approval |
| LIB-07 | in-progress | Trusted parsers, malware scanning, OCR, sandboxing |
| COM-03 | not-started | Coupons, referrals, regional tiers, trials, pause |
| ADMIN-05 | not-started | Revenue, royalties, licenses, renewals |
| LIB-09 | not-started | Licensed offline media packages and rights |
| IMG-05 | in-progress | Low-end reference-device performance acceptance |
| UX-03 | not-started | Reference-device performance budgets |
| TRUST-06 | not-started | Accessibility/device matrix acceptance |
| PROT-01 | in-progress | Native capture controls and device acceptance |
| PROT-02 | in-progress | Device attestation, anti-scraping, encrypted packs |
| PROT-03 | not-started | Store/platform compliance and developer accounts |
| COM-02 | not-started | Payment-provider/store billing integration and checkout |
| ENG-03 | not-started | Native widgets and lock-screen timer |
| ARCH-03 | not-started | Owned Swift/Kotlin Tauri plugins and shells |
| INST-06 | in-progress | Real campus LMS acceptance and 1EdTech certification |
| PLAN-05 | not-started | Calendar read/write scopes |
| IMG-03 | not-started | DICOM privacy and pixel-integrity workflow |
| CAREER-02 | not-started | Human-supervised feedback/sign-off |
| AI-15 | not-started | Evaluated multilingual tutoring |
| QB-10 | blocked | Externally validated readiness/calibration record |
| CAREER-03 | in-progress | Accredited provider workflow |
| COM-04 | blocked | Owner decision on exam/pass-extension business model |

Additional operational decisions include email delivery, offsite backup and a
measured restore drill, telemetry, site origin, voice provider, native app IDs,
and the owner-reviewed learner token transport. Existing browser sessions still
use localStorage; this feature does not implement an httpOnly-cookie migration.

## Release boundary

Passing this CI suite accepts the implemented branch scope. The open ledger
features, external acceptance, and Orca runtime verification prevent a claim
that every feature is complete or that the whole project is production ready.
The historical completion plan contains older snapshots; the canonical ledger
and this dated report identify the scope audited here.
