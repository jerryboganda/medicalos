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

## Re-test and editorial regression evidence

- Run 36785848450 passed format, migrations and Clippy and 143 of 144 API
  integration tests. All five receipt-grading cases and the rights cases passed.
  The remaining assertion incorrectly expected "again" for a correctly answered
  family variant without confidence; the correct rating is "hard", passes=0.
- Run 36786239795 confirmed the new editorial regression: a failed submission
  audit returned an internal error but left the draft in_review. The candidate
  now locks the version and commits status, review and audit in one transaction.
  Review failure, independent provenance and competing-decision cases are added.
- The next exact-source run must pass the complete suite, client builds and
  browser checks. Gallery artifacts will be inspected after they are generated.

## Session-policy atomicity evidence

- Run 36810363234 failed the new regression against the old implementation.
  A database check rejected retirement of one existing session. The setting
  update had already committed; a later sign-in therefore revoked the remaining
  sessions. The test observed 401 where the preserved sessions must return 200.
- The setter now takes the same user-row lock as login and commits the setting,
  all-session retirement and audit as one transaction. An authenticated read
  route and Account controls were added with honest loading/error states and an
  explicit warning/confirmation for the immediate sign-out.
- Run 36810848227 reran the same red regression before the setter fix was in
  the branch; it confirmed the same 401-after-failed-retirement defect. The
  transaction and UI changes, including device-limit recovery, still need
  exact-source GitHub Actions.

## Candidate 527f06f

- Run 36788089142 passed Rust, client, site and Zitadel jobs. All 148 API
  integration tests passed, including all 20 critical launch regressions.
  All six backup cases and the disposable full-schema restore passed. The
  checked-in TypeScript contracts passed drift validation (362 export tests).
- Browser result: 77 passed, two OIDC failures, one retry-passing flaky check.
  OIDC fixtures issued synthetic tokens without intercepting the new device
  registration seam; only those named fixture tokens now use the existing
  test helper. Genuine tokens continue to the API.
- Session integrity callbacks now stop before leaving the route and ignore
  late responses. The cleanup check waits for the next page's rendered heading
  and rejects newly created blur signals, excluding earlier queued requests.
- Inspected Account PNGs at 1280px desktop and 390px phone in both themes.
  Existing type, tokens, cards, rail/mobile menu and five tabs are preserved.
- The SQLx offline cache was synchronized from this run's Rust artifact,
  not generated locally. Follow-up exact-source acceptance remains required.
- The final Luna/max editorial inspection again returned "Your workspace is
  out of credits. Add credits to continue." Independent review is incomplete.

## Browser registration and cleanup follow-up

- Run 36810247581 passed all five jobs at source commit
  5b11037c8812b13e6a45ac8566c2b8be4a0488c7. The Rust suite, client/site builds,
  Zitadel provisioning and browser E2E passed after the OIDC fixture
  registration and late session-integrity callback repairs. This source
  predates session-policy UI and the stricter offline-cache and no-retry
  cleanup gates added afterward.
- Runs 36810844176 and 36810848227 stopped at the still-red session-policy
  regression. They do not accept the later transaction fix; the next full run
  must validate the corrected source and both stricter release checks.

## Session-policy API and cache refresh

- Run 36813649924 at source `87b1e69dd4ed15156702e782092f78d07d8000a8` passed Rust, client, site, Zitadel and browser jobs. Critical/full API tests, browser E2E and the no-retry session-cleanup repeat passed.
- Its offline compile failed only because the committed 437-file cache lacked the getter and active-account update queries. The Rust job's CI artifact contains 438 query files; compared with the checked-in set it adds those two queries, removes the obsolete update query, and leaves the other 436 unchanged. The local cache now exactly matches that artifact.
- The offline job in run 36813649924 checked the immutable pre-sync tree, so a later exact-source offline pass is required. The same commit also predates the follow-up Account focus-return assertion; its updated browser behavior and current gallery need exact-source acceptance.
- Inspected the four Account images from that run's gallery: desktop and phone in dark and light themes. Session security uses the existing card, chip and button patterns; the five primary mobile tabs and layout remain unchanged. The later focus change adds no visual styling.

## Current rights and offline receipt regression

- PR run 36921326889 at source `142f74c295336addf3f96e1ce6df804cb2423978` passed format, migration, Clippy, dependency, critical-regression, site and Zitadel checks. The Rust integration suite passed 164/165; `reserved_family_form_session_and_ai_gate` failed because its synthetic rights grant allowed display but omitted the newly enforced `offline` use while the test explicitly downloaded a pack. No browser job ran, so the signed-receipt regression remains unverified.
- The fixture now explicitly grants `offline` for this pack scenario. This keeps the server's fail-closed rights rule and the separate reserved-assessment AI restriction intact. The retest passed the full Rust suite and reached the browser regression recorded below.

## Signed offline-receipt regression

- PR run 36923455791 at source `a9bcdecb938c6f0fe793dfcaa48ef12c051a0dd9` passed Rust, client, site and Zitadel. The offline compile failed because the committed SQLx cache lacked current query records. Its Rust artifact contains 438 query records; eight were absent locally, and each artifact record had an exact filename and SHA-256 match after syncing. Existing cache-only records were retained.
- The browser run passed 83 tests and reproduced the invalid-receipt regression: a 64-byte zero signature was accepted, the UI reported success, and all 26 questions were saved. The regression requires signature verification to fail before batch persistence. The client now verifies the receipt's Ed25519 signature against the already-verified manifest key and rejects malformed signature encodings. Exact-source GitHub Actions is still required for both changes.
