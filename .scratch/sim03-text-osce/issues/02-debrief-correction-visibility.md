# SIM-03 — Show corrected transcript segments in the learner debrief

Status: ready-for-agent
Requirement IDs: SIM-03, SIM-04
Source: `.scratch/sim03-text-osce/spec.md`; `.scratch/remaining-phases/issues/11-sim04-criterion-evidence.md`
Implementation state: implemented and statically reviewed; browser acceptance is deferred to the final GitHub Actions gate.

## Problem Statement

The debrief API returns append-only transcript corrections, but the learner
scenario page does not render them. Learners therefore cannot see a correction
that the examiner's uncertainty handling and appeal flow already use.

## Solution

Add a compact, read-only correction list to the existing scenario debrief.
Show each corrected event number, original event, and corrected text while
stating that the original transcript and timeline remain unchanged.

## Implementation Decisions

- Render the section only when `transcript_corrections` is non-empty.
- Keep corrections separate from the immutable timeline and rubric evidence.
- Do not expose internal user IDs or add edit/delete controls.
- Reuse the owner-approved app shell tokens; no new palette, font, motion, or
  interactive states.

## Acceptance

- A learner with corrections sees the numbered event, original event, and
  corrected text in the debrief.
- A debrief without corrections remains unchanged.
- Existing scenario E2E coverage verifies the correction display and checks
  horizontal overflow at 320, 375, 414, and 768 px.
- Final GitHub Actions browser and client jobs pass.

## Testing Decisions

- Extend `tests/e2e/sim06-scenario-flow.spec.ts` with a corrected-event fixture
  and visible-text assertions, then reload with no corrections to verify the
  section stays hidden and the original timeline remains visible.
- Defer browser execution and client builds until all implementation slices
  are complete, as directed by the user.

## Out of Scope

- Editing or deleting transcript corrections.
- Changes to assessment, correction submission, clinical validation, or
  consequential-use policy.
