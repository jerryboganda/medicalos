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

## Release acceptance

No deployment, real-provider acceptance, clinical approval, device-matrix run,
student beta or production disaster-recovery drill is claimed by this record.
CI restore testing uses the disposable database and synthetic data only.
