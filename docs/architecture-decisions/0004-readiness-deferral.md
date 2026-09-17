# ADR 0004 — No numerical readiness until validated (owner decision 12)

Status: Accepted (owner decision 12, 17 September 2026) · Date: 2026-09-18

## Context

The Quiz LMS baseline proposed an early predicted score / pass probability. §24 sets the validation bar: defined target outcome, consented outcome data, representative sampling, leakage controls, held-out period, external validation where feasible, calibration, subgroup checks. Showing a precise-looking invented number earlier is a trust violation (§2.3, TRUST-05).

## Decision

No numerical readiness, predicted score, or pass probability ships before Phase 7 validation. Progress shows observed measures only (accuracy, independent-attempt accuracy, coverage, retention trends, uncertainty; §24). Mock results are observations about a form (§11.7). The early-estimate feature is deferred.

## Consequences

Consented exam-outcome collection starts in Phase 1 (`POST /v1/outcomes`, modest reward that never biases toward passes) so Phase 7 validation is possible. The UI must show honest sparse-data messages ('Not enough evidence') from day one (AI-02). Any percentage that could read as a pass probability is a TRUST-01 defect.
