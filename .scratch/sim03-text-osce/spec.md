# SIM-03 — text-mode transcript uncertainty

Requirements: SIM-03 (voice/text with transcript uncertainty), SIM-04
(criterion evidence linking), TRUST-01
Source: master plan §15 ("Display uncertain transcript segments and allow
correction before final feedback"; "Every criterion-level judgment should
link to transcript or action evidence"; "report 'not assessed' rather than
inventing").

## Scope

Text-mode runs gain authored transcript uncertainty and the correction +
evidence workflow around it. Voice arrives behind replaceable adapters later
(owner model decision); in text mode the uncertainty markers are authored
into the scenario fixture and labelled as the drill they are — no fake ASR
output.

1. **Authored uncertainty.** A scenario version's state machine may mark
   states with `transcript_uncertain: true` (plus an optional
   `transcript_uncertainty_reason`). Advancing into such a state appends a
   transcript event carrying `uncertain: true` and the reason, so the
   uncertainty is visible in the run, the timeline, and the debrief.
2. **Correction before final feedback.** The run owner (or an operator)
   may correct an uncertain event via
   `POST /v1/scenarios/runs/{run_id}/transcript-corrections`. Corrections
   are separate append-only records (unique per run + event) that keep the
   original; the transcript itself is never rewritten.
3. **Uncertainty-aware examiner evidence.** When an examiner's criterion
   judgment cites an uncorrected uncertain transcript event, the evidence
   must carry `transcript_uncertain: true` — otherwise the assessment is
   refused with `uncertain_transcript_evidence`. Not-assessed results keep
   their existing no-score/no-evidence semantics.
4. **Debrief surfacing.** The debrief returns the run's transcript
   corrections alongside the timeline.

## Acceptance

- Advancing into an uncertain state produces an `uncertain` transcript
  event with its authored reason.
- A correction by the run owner succeeds once; a second correction or a
  correction of a non-uncertain event fails; non-owners are refused.
- Recording an assessment that cites an uncorrected uncertain event without
  `transcript_uncertain` is refused; with the flag it is accepted and the
  evidence row stores the flag.
- The debrief lists the correction.
