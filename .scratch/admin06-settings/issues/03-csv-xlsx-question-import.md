# ADMIN-06 — CSV and Excel question import

Status: ready-for-human
Requirement IDs: ADMIN-06, ADMIN-02, TRUST-07
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §19.5
Implementation state: scoped CSV/XLSX import, active rights checks, metadata persistence, and API/browser regressions are authored; final GitHub Actions verification and external contract evidence remain.

## Problem

The Editorial Console accepts JSON rows only. Editors preparing question banks
in spreadsheets cannot use the supported CSV/Excel workflow, and the current
question record does not retain tags or multiple source and media references.

## Solution

Keep the existing authenticated admin import and batch lifecycle. Add CSV and
`.xlsx` upload using one documented column template, parse to the same question
validation and transactional dry-run/apply path, and retain all supported
editorial metadata on the immutable question version. Imported questions stay
drafts and still require the existing independent review and publish workflow.

## Template columns

Required: `chapter_id`, `difficulty`, `vignette`, `lead_in`, `option_1`,
`rationale_1`, `option_2`, `rationale_2`, `correct_option`,
`key_learning_point`, `source_ref`, and `rights_ref`. `correct_option` is
one-based. Add paired `option_3`/`rationale_3` through
`option_10`/`rationale_10` as needed.

Optional: `exam_tip`, `hint`, `high_yield`, `tags`, `references`, and
`media_refs`. List values use `|` separators. A blank option pair is ignored;
an option without its paired rationale is reported as a row error. CSV and the
first worksheet of `.xlsx` use identical headers and semantics.

Every `rights_ref` must resolve to a currently active rights-ledger record that
permits both `display` and `derivatives`. Its explicit `asset_refs` must cover
the row's primary source, every `references` entry, and every `media_refs`
entry. The import transaction locks the matching grant through the batch write
so a concurrent revocation cannot pass validation and then be saved.

## Acceptance

- The admin console downloads a CSV template and accepts `.csv` and `.xlsx`.
- The API accepts only those formats, enforces the upload and 1–500 row limits,
  and reports malformed headers, cells, and question fields with spreadsheet
  row numbers.
- Dry run creates no questions; apply is atomic, records one import batch and
  audit receipt, and creates draft versions. Existing rollback and independent
  review rules continue to apply.
- Variable option counts and all rationale, source, tag, media-reference, and
  `rights_ref` values are retained. A later version keeps them when duplicated.
- Integration coverage rejects missing, inactive or revoked, under-permissioned,
  and under-scoped rights records; accepted imports retain the ledger reference.
- Integration tests cover quoted CSV values, workbook parsing, invalid rows,
  dry run, apply, metadata persistence, and rollback. Playwright covers template
  download and file selection through the existing console.

## Seams

- Authenticated HTTP import endpoint, reusing the existing question importer.
- Existing `/admin` Editorial Console, with the design-system tokens and form
  controls already used by the page.

## Out of scope

Legacy `.xls`, macro-enabled files, arbitrary workbook formulas, direct media
upload/extraction, permission to publish imported content, and external legal
verification of the rights contract. This slice enforces the configured ledger;
it cannot establish that external contract evidence is genuine or sufficient.

## Implementation record

- The import transaction accepts JSON, CSV, and `.xlsx`, validates row/header
  data and the active rights ledger, and preserves tags, sources, media refs,
  and `rights_ref` on each immutable question version.
- A replay-safe migration adds question metadata and the rights-reference
  field. The console offers a CSV template, file preview/apply actions, and
  retains the existing JSON importer.
- HTTP integration and Playwright coverage are authored, including rights
  permission and asset-scope failures. No build or test has run locally; final
  GitHub Actions verification is deferred until the full implementation batch.
- Imported values remain drafts. External rights evidence and the remaining
  ADMIN-06 policy settings still require separate work or owner decisions.
