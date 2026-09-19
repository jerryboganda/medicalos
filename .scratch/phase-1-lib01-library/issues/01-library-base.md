# 01: LIB-01 versioned articles and references

Status: ready-for-agent

Requirement ID: LIB-01.

Implement `../spec.md` through the existing authenticated HTTP and Learn seams.

- [ ] RED HTTP contract exists before implementation.
- [ ] Stable library identities and immutable numbered published versions are modeled.
- [ ] Phase-1 kinds are limited to article/reference.
- [ ] Latest list returns only the newest published version per item.
- [ ] Explicit historical version lookup is supported.
- [ ] Version metadata includes provenance, source, date, and jurisdiction.
- [ ] Library versions link to concept versions and question versions.
- [ ] Only synthetic reviewed seed content is added.
- [ ] Learn UI exposes a compact accessible Library section without a new primary destination.
- [ ] No new dependency or search/ingestion abstraction is introduced.
- [ ] Matt Standards/Spec review, Ponytail review, and Hallmark closeout are completed.
- [ ] Traceability is updated truthfully.
- [ ] Heavy verification uses GitHub Actions; deployment is skipped.

## Comments

- The HTTP seam is reused because it is already the repository's highest-value integration boundary.
- Known GitHub Actions billing/spending-limit failure is not a reason to run heavy checks locally.
