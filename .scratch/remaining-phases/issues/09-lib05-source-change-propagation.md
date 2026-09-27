# LIB-05 — Source-change propagation

Status: resolved
Requirement IDs: LIB-05, TRUST-07, QB-08, AI-18
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §19.4 and §9.5

## Problem Statement

Editors cannot trace which published learning resources depend on a specific
source passage. When guidance changes or a source is withdrawn, they need to
find affected questions, explanations, cards, lessons, simulations and learners
without silently changing the evidence used in historical attempts.

## Solution

Register stable source passages by reference and locator, link them to exact
published resource versions, and record each observed source change as an
auditable case. A case snapshots affected resources into review tasks, marks
unsafe resources unavailable for new use, and applies explicit notification
and score-recalculation policies based on the change type.

## User Stories

1. As an editor, I want to identify a source passage by citation and locator,
   so that a later correction refers to the same external material precisely.
2. As an editor, I want to link a question, lesson or simulation version to the
   passages it uses, so that source changes produce a useful impact graph.
3. As an editor, I want one review task for each directly affected published
   version, so that every dependency is reviewed and closed explicitly.
4. As an editor, I want to classify a change as a minor typo, material change,
   invalid answer key or withdrawn source, so that review and learner actions
   match the risk.
5. As a learner, I want unsafe questions and simulations removed from new
   sessions immediately, so that I do not receive known-invalid content.
6. As an affected learner, I want a clear in-app notice when an answer key is
   invalidated and again when its reviewed correction is applied.
7. As an editor, I want corrected answer keys to recalculate affected practice
   evidence only when the replacement preserves option identity and order, so
   that historical answers are repaired audibly without guessing mappings.
8. As an editor, I want minor spelling fixes to create review work without
   changing scores or notifying learners about a non-material edit.
9. As an editor, I want to see affected learner counts and derived cards for a
   source-change case, so that I can judge the reach of the correction.
10. As a learner, I want authored cards based on unsafe questions suspended
    until the content is reviewed, while retaining my card records.

## Implementation Decisions

- A source passage stores a stable source reference and locator, not copied
  source text. Rights and document-ingestion rules remain owned by LIB-06.
- Source dependencies point to exact question-version, article-version, and
  scenario-version records. Pregen tutoring and learner cards are derived
  through their question-version source, avoiding duplicate graph edges.
- Source changes are append-only events with a source revision, actor,
  classification and concise editorial note. Each event snapshots one review
  task per linked resource version that is currently published; archived and
  quarantined versions remain historically linked but are excluded.
- `minor_typo` creates review tasks without quarantining content, recalculating
  scores or notifying learners. `material_change` creates review tasks.
  `invalid_answer_key` and `source_withdrawn` quarantine linked published
  resources immediately and identify learners with relevant attempt, card,
  article-read or simulation-run history.
- Unsafe question changes suspend derived learner cards and remove pregen
  tutoring from newly generated responses and manifests. Disconnected devices
  may retain already downloaded content until their signed lease expires.
- A corrected answer key must use a newly published version in the same
  question family with identical option text and order. An explicit correction
  ledger records each changed attempt; the API updates effective correctness,
  recomputes affected per-chapter learner state, refreshes stored session score
  receipts, and notifies affected learners. It does not rewrite their selected
  option or question-version reference. It recalculates submitted sessions;
  already-open sessions keep their pinned content. The corrected version
  inherits the source dependency so future source changes include it.
- Article reading impact uses one first-read record per user and article
  version, only for authenticated reads. Simulation runs record the exact
  scenario version used.
- Admin APIs register passages and dependencies, record source changes, list
  impact/review tasks, and resolve a task as reviewed-current, corrected or
  retired. Each task reports affected learner IDs and derived-card counts.
  Every write is admin-gated, validated, transactional and audited.
- Regression seam: the authenticated HTTP API. Integration cases cover source
  registration/linking, minor versus unsafe change behavior, quarantine,
  impacted-user identification, correction audit/recalculation, and task
  resolution. A browser regression verifies the learner's source-update
  notification preference roundtrip. Existing editor/API fixtures provide
  prior art. Tests are added before implementation and executed in GitHub
  Actions at the final CI gate.

## Testing Decisions

- Test user-visible API behavior and persisted outcomes through HTTP requests;
  do not test private helpers or mirror SQL implementation details.
- Verify immutable attempt identity, explicit correction audit rows, exact
  learner-state recomputation, and idempotent review-task resolution.
- Verify a minor typo leaves current content, scores, cards and notifications
  unchanged; verify an invalid key quarantines content and notifies only
  learners with a recorded dependency impact.
- Keep client builds, browser E2E, database tests, formatting and rollback
  verification in GitHub Actions, consistent with the repository compute rule.

## Out of Scope

- Copying or ingesting external source content, license acquisition and rights
  verification (LIB-06).
- Reversing XP, competition points or other engagement rewards after a score
  correction; these remain separately recorded participation outcomes.
- Immediate remote deletion of content already downloaded to a disconnected
  device; signed offline lease expiry remains the revocation boundary.
- Automated medical judgment, automatic publication, or replacing human
  editorial review with an AI decision.

## Further Notes

- `question_versions.source_ref` and `article_versions.source_ref` are currently
  free text. They remain display citations; only explicit dependency links
  participate in the change graph.
- API and browser regression cases, formatting, builds, tests, and migration
  verification passed in GitHub Actions run
  [36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978).
- The later full CI run
  [36245924351](https://github.com/jerryboganda/medicalos/actions/runs/36245924351)
  passed on `63e5cd5`, an ancestor of the current branch. The source-change API
  has no changes since that tested revision; the later library-page edit only
  updates the private-import generated type name. A branch-wide rerun remains
  pending because GitHub Actions currently rejects jobs before runner startup
  for account payment/spending-limit reasons.
- Result actions and due retests now exclude quarantined or archived question
  versions; the retest queue cannot fall back to unsafe stored content.
- Open sessions retain their pinned key after quarantine. Answer replay avoids
  regenerating tutoring for unavailable versions, submitted sessions adopt a
  correction only after review resolves, and misses recorded after quarantine
  create retained but suspended key-point cards.
- Resolving an older safe source review as current no longer republishes its
  resource, so it cannot undo a later quarantine. API regressions cover this
  ordering and the in-flight card case.
