# ENG-01 — Share the daily question by exam

Status: complete
Requirement IDs: ENG-01
Triage label: complete
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §17.2; `.scratch/eng01-shared-qotd/spec.md`

## Acceptance

- Learners can select their QOTD exam from exams in their curriculum.
- Each exam receives one persisted eligible question per database calendar day, shared by every learner who selects that exam.
- The question does not change during that day if the pool changes.
- If the selected question becomes ineligible, the API fails closed without choosing a replacement that day.
- A learner can change exam before answering, but cannot change after answering until the next database day.
- No exam and no eligible question have explicit UI states.
- API integration and Playwright E2E verify the behavior through the existing HTTP and Today seams.

## Comments

- 2026-09-26: The previous picker was user-specific across all published questions. The project has no global current-exam setting, so this slice adds an explicit QOTD-only exam selector rather than inferring from plan or session history.
- 2026-09-26: Verified in [GitHub Actions run 36211024073](https://github.com/jerryboganda/medicalos/actions/runs/36211024073) at `8641ba2e36b3ed78379d1feb8c023ae1127d9154`: all four jobs passed, including 107 Rust integration tests, 54 Playwright browser E2E tests, migration rollback/replay, and wasm32 shared-core build. This closes the shared-per-exam QOTD slice; ENG-01 still has the declared-time goal and scheduled-push work open.
