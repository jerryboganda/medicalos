# Full-platform batch review

Fixed point: c635e570d0fa9267a84c3b68270b16fe9be61e62.
Specs: spec.md, ui-contract.md, account-contract.md and retest-contract.md.
Design authority: design.md. No primary tab, theme or font redesign.

## Standards

Completed Luna/max standards review covered the earlier security/navigation
snapshot. Final account/spec workers failed with the workspace-credit error;
the required independent final review is incomplete, not passed.

Primary-session integration found and repaired a rights-fixture Clippy error,
missing account device-registration fixture, ambiguous Account heading selector,
unknown surface-token usage and the device-cap recovery dead end. Device
registration keeps captured bearer identity and fails closed for study calls.
Only the explicit device-limit error allows authenticated account controls.
The account page validates unknown wire payloads, uses native date formatting
and JSON downloads, and reuses existing buttons/cards/chips/layout tokens.
The acknowledgement label has a minimum 44px target.

## Spec

Every stable requirement is retained. The Account page covers the supported
partial export and access disablement accurately. It does not satisfy a
complete archive or policy-driven erasure; those remain pending. Clinical
rights records do not establish the authenticity of a license, and extractive
Coach metadata does not implement a remote provider.

Re-test grading now uses submitted answer receipts; its five new regressions
passed in run 36785848450. The family-variant fixture expected the wrong rating;
its assertion now checks the variant's actual key and missing-confidence grade.
Question transitions now share one locked transaction, including review and
audit writes. The original failed-audit defect was confirmed in run 36786239795.
Both repairs still require complete exact-source acceptance.

Confirmed pending tails: runtime question withdrawal after rights expiry/revocation;
article draft publication without independent author/reviewer or rights gates;
incomplete privacy export; generic auth still accepting an unbound bearer;
provider/native acceptance. These must not disappear
behind historical tested labels.

## Ponytail review

No new dependency, framework or speculative account abstraction was added.
Shared device identity reuses the offline key; browser fixtures remain test-only.
Native download/date APIs provide the account interactions. The typed-page
validation is needed at an existing unknown-JSON boundary.

Lean already for the account slice. CI, source review and final independent
review status remain separately recorded; this is not full-platform acceptance.

Editorial repair reuses the existing transition function and transaction, with
no new abstraction or dependency. One row lock protects all decisions, and
failed writes roll back status, provenance and audit together. The CI adds a
targeted regression gate before the retained complete test suite.
