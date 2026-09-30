# Full-platform verification evidence

## Readiness regression: red

- Source commit: 31d595972c28b0134872c2bc0c93f69da1c18ee0.
- GitHub Actions run: https://github.com/jerryboganda/medicalos/actions/runs/36765780450.
- Result: the 128 existing API integration tests passed; the new
  `full_platform_readiness::readiness_requires_database_and_preserves_liveness`
  failed because `/readyz` returned 404. Rust formatting, migration checks and
  compilation had already passed. Site and Zitadel jobs passed.
- This deliberate red run is regression evidence, not a release candidate.

## Operational implementation: green

- Source commit: e70ba4543a84b80b0f00a24c022bc92ab53da8fb.
- GitHub Actions run: https://github.com/jerryboganda/medicalos/actions/runs/36767943398.
- Result: all five jobs passed (site, Zitadel, Rust, client and browser E2E).
  Browser verification passed 72 tests. The new readiness regression passed;
  the backup harness passed success, missing configuration, upload failure,
  corrupt remote copy, dump failure and invalid retention checks. The restore
  drill restored the complete schema into a disposable database and verified
  synthetic learner relationships, answer receipts and notes.
- This evidence covers this source commit only. Subsequent security, UI and
  content-rights changes require a new CI run.

## Session, LTI and mobile navigation: green

- Source commit: 400ed80ec0d32f5746de12d07f3c5369717c6df7.
- GitHub Actions run: https://github.com/jerryboganda/medicalos/actions/runs/36772428670.
- All five jobs passed. This covers device-session binding/revocation, tenant-owned
  LTI configuration, guarded JWKS addresses, honest Coach engine metadata, and
  mobile menu keyboard focus containment. Later publication/account changes
  are not covered by this run.

## Publication and browser registration: lint repair awaiting acceptance

- Runs 36774254899 and 36774746446 stopped at Clippy because the new rights
  fixture helper exceeded the default argument-count lint. The repair scopes
  the existing repository test-helper allowance to that one function.
- The account page and device-limit recovery have been added to browser
  coverage and the account page to the existing desktop/phone theme gallery.
  Their acceptance requires the follow-up exact-source CI run.
- Luna/max implementation workers completed the prior security/content
  changes. The final account/spec review workers stopped with the explicit
  workspace-credit error. Their unfinished review is not counted as passed.
  Primary-session source integration continues; no Astra-Gemini worker is used.

## Release acceptance

- The account candidate run 36783972136 passed format, migration and Clippy,
  then stopped with two rights-test method errors (POST instead of PATCH) and
  a pre-existing QOTD fixture dependent on daytime UTC. Fixes retain actual
  revocation and quiet-hour assertions and configure eligible fixture users
  with an explicitly empty quiet window.
- The re-test regression commit 66bf2cf in run 36784150909 proved a genuine
  grading defect: HTTP 200 and passes=1 for a correctness claim with no answer.
  The expected status was 422. Receipt-based grading, atomic scheduling and
  single-use/replay cases require the follow-up implementation run.

No deployment, real-provider acceptance, clinical approval, device-matrix run,
student beta or production disaster-recovery drill is claimed by this record.
CI restore testing uses the disposable database and synthetic data only.
