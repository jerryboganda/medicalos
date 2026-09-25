# QB-13 — Session tools and local-first workspace

Status: ready-for-human
Requirement IDs: QB-13, QB-04, QB-06, NOTE-01, OFF-02, EX-08
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§7.5, 8.3, 11.2–11.6
Implementation state: API/browser and shared WASM CI acceptance passed in run 36159484978.

## Scope

Complete the existing session page with the §11.6 tools: deterministic
medical calculators, common-unit conversion, four text sizes, tutor-only
reasoning hints recorded as assisted evidence, question-linked autosaved
notes, persistent question marks, local-first selection/navigation/elimination/
highlight/timing state, and 10/5/1-minute expiry warnings with local
auto-submit. Reuse `calc-engine`, `question_marks`, notes, and the existing
session API.

## Acceptance

- Calculator results use the shared Rust calculation engine and always carry
  the exam-practice / not-for-clinical-use label. Invalid or missing inputs
  fail with a stable 422 response.
- A hint is returned only after an authenticated learner requests it during
  an open tutor session. The hint is never included in the initial session
  response; its use remains assisted even if the client sends `assisted=false`.
- A question note is linked to its question version, immediately persisted on
  the device, autosaved through the existing conflict-aware note API, and
  visible in the learner's notebook. Mark/unmark uses the existing private
  marks API and remains in the marked-question pool.
- Current question, selected and pending answers, marks, notes, highlights,
  eliminated options, and timer anchor are saved locally before each request.
  Pending answers replay with their original idempotency key after reconnect.
- Pending offline answers can be navigated across the full session without
  exposing unconfirmed correctness or fabricated feedback.
- Timers use a monotonic elapsed clock anchored to the server deadline. A
  locally expired session stops accepting new input, presents the 10/5/1
  minute warning when each threshold is crossed, and queues its submission
  while offline.
- The session tool tray works at narrow mobile widths with existing Medical OS
  colors, type, spacing, and card styles.

## Verification boundary

API integration and Playwright coverage passed in GitHub Actions run
[36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978).
Offline writes remain pending and visibly unsynced until the existing server
sync seam confirms them; never show an invented score.
