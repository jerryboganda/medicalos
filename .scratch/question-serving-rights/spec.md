# Runtime question-content rights

## Goal

Published question content must stop being served when its display grant is
missing, revoked, expired, outside the learner audience, seat-limited without
seat allocation, or missing any source/media asset used by the question.
Publication-time validation is not a permanent display entitlement.

## Shared display rule

A question is currently displayable only when its `rights_ref` resolves to a
rights record that is active today, permits `display`, permits the `learners`
or `all` audience (or has no audience restriction), has no unsupported seat
limit, and covers the question's source, source references, and media
references. Missing or malformed provenance fails closed. The same database
predicate must be used by student-facing question paths so expiry and
revocation have one definition.

This display rule is separate from external confirmation that a license is
authentic. Production content still requires documented, reviewed grants and
the applicable licensor/territory/contract acceptance.

## Delivery slices

1. **Self-directed practice and Coach** — filter new practice/retry/revision
   pools; stop session reads, hints, answer feedback/replays, and Coach
   responses/history when a grant is no longer active. Preserve learner-owned
   attempt and aggregate result evidence; never include question text or keys
   in a rights-denied response.
2. **Assessments, mocks, and guest access** — apply the rule to question
   selection, launch, detail, feedback, and public/demo paths.
3. **Study planning, review, and derived content** — cover daily recommendations,
   QOTD, retests, marks, notes, tutoring cards, and question-linked insights;
   ensure derived copies cannot outlive their source display permission.
4. **Offline packs** — bind pack manifests, downloads, receipts, and local
   access leases to the grant and define revocation behavior for devices that
   are temporarily offline.

Each slice must inventory every endpoint in its domain, add route-level
regressions for active/revoked/expired rights, and pass exact-source GitHub
Actions. A green code run does not attest to real license provenance or
student acceptance.

## Current slice acceptance

- Current eligibility is centralized in a database function and agrees with
  the existing publication checks, including audience, seat, and asset scope.
- Practice and retry/revision paths do not select newly ineligible questions.
- A previously created session cannot return question text, options, hints,
  answer feedback, or Coach-derived content after its grant is revoked or
  expires. This includes idempotent response replays and Coach history.
- Submitting a previously started session can preserve aggregate learner
  evidence without returning licensed question content.
- Missing rights fail closed; existing synthetic test fixtures receive
  explicit synthetic grants rather than a null-rights exception.
- The existing Today eligibility query uses the shared predicate.
