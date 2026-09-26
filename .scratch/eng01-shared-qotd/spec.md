# ENG-01 — Shared question of the day per exam

Status: complete (shared-per-exam QOTD slice; ENG-01 remains in-progress)
Requirement: ENG-01

## Problem

The current question picker hashes the learner ID and date across every published question. Learners on the same exam can therefore receive different questions, and the API has no explicit exam-selection seam.

## Design

Use a learner-selected QOTD exam in the existing engagement settings and Today card. Reuse the existing curriculum endpoint for exam choices. Persist one question version per exam and database calendar day so concurrent requests, newly published questions, and different learners cannot change that day's selection. Choose deterministically from published, unreserved questions linked through `question_versions.chapter_id` to the selected exam.

Learners may change their selection before answering. After answering, the selected exam is locked until the next database calendar day because the existing QOTD contract allows one answer per learner per day. The server rejects any mismatched answer and returns no question when no exam is selected or no eligible question is available.

Alternatives considered: infer the exam from the most recent practice session or plan (ambiguous for learners studying multiple exams); silently choose the first available exam (may be the wrong exam); or add an explicit QOTD exam selection (chosen because it makes the rule visible and reversible before answering).

## Acceptance

- Today lets the learner choose from exams present in their curriculum.
- The authenticated API returns the selected QOTD exam and one stable daily question shared by all learners selecting that exam.
- Concurrent first reads create and return one shared daily row.
- The daily selection stays stable if the eligible question pool changes after first selection.
- If the pinned question becomes ineligible during the day, QOTD is unavailable until reset rather than silently switching questions.
- Different exams have separate daily selections.
- No selected exam and no eligible question are explicit, non-error states.
- Answering remains one-per-learner-per-day; changing the selected exam afterward returns a conflict until the next database calendar day.
- The existing question-answer mismatch protection, hidden answer key, per-learner disable, and global disable remain enforced.
- API integration and browser E2E pass in GitHub Actions, including migration rollback.

## Verification boundary

Use the authenticated HTTP integration seam for selection, stability, exam isolation, concurrency-safe persistence, answer locking, and mismatch rejection. Use the real Today browser flow to choose an exam, answer the displayed item, then continue the existing learner loop. GitHub Actions is the only build/test authority for this repository.

## Acceptance evidence

GitHub Actions run [36211024073](https://github.com/jerryboganda/medicalos/actions/runs/36211024073), commit `8641ba2e36b3ed78379d1feb8c023ae1127d9154`: all four jobs passed; 107 Rust integration tests and 54 Playwright browser E2E tests passed; migration down/up replay and wasm32 shared-core build passed.
