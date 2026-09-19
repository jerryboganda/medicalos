# 01: LIB-01 versioned articles and references

Status: blocked-on-final-ci-acceptance

Requirement ID: LIB-01.

Implement `../spec.md` through the existing authenticated HTTP and Learn seams.

- [x] RED HTTP contract exists before implementation.
- [x] Stable library identities and immutable numbered published versions are modeled.
- [x] Phase-1 kinds are limited to article/reference.
- [x] Latest list returns only the newest published version per item.
- [x] Explicit historical version lookup is supported.
- [x] Version metadata includes provenance, source, date, and jurisdiction.
- [x] Library versions link to concept versions and question versions.
- [x] Only synthetic reviewed seed content is added.
- [x] Learn UI exposes a compact accessible Library section without a new primary destination.
- [x] No new dependency or search/ingestion abstraction is introduced.
- [x] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [x] Traceability is updated truthfully.
- [x] Heavy verification uses GitHub Actions; deployment is skipped.

## Comments

- The HTTP seam is reused because it is already the repository's highest-value integration boundary.
- Matt Spec review found 0 findings. Matt Standards review found 0 hard violations and one non-blocking tuple-clarity smell; Ponytail review intentionally kept the current simple shape rather than extracting a premature abstraction.
- Hallmark closeout found no blocking UI/accessibility issue after library failures were isolated from the existing spaced-review queue.
- Local `cargo fmt --all -- --check` and `git diff --check` passed.
- GitHub Actions run 35446948317 failed before runner steps because recent account payments failed or the spending limit needs to be increased. No retry was attempted. Final heavy acceptance remains blocked externally.
- Deployment remains skipped per owner instruction.
