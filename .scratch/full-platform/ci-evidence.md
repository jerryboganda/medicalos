# Full-platform verification evidence

## Readiness regression: red

- Source commit: 31d595972c28b0134872c2bc0c93f69da1c18ee0.
- GitHub Actions run: https://github.com/jerryboganda/medicalos/actions/runs/36765780450.
- Result: the 128 existing API integration tests passed; the new
  `full_platform_readiness::readiness_requires_database_and_preserves_liveness`
  failed because `/readyz` returned 404. Rust formatting, migration checks and
  compilation had already passed. Site and Zitadel jobs passed.
- Green verification of the implementation is still pending. This deliberate
  red run is regression evidence, not a release candidate.

## Release acceptance

No deployment, real-provider acceptance, clinical approval, device-matrix run,
student beta or production disaster-recovery drill is claimed by this record.
CI restore testing uses the disposable database and synthetic data only.
