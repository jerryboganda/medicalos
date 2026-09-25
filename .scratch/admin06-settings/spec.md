# ADMIN-06 — Validated live settings

Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §19.5

This spec defines issue 01's five baseline settings. Offline lease duration,
CSV/Excel question import, daily review caps, and competition scoring settings
are tracked in issues 02–05.

## Problem statement

The admin settings endpoint stores JSON overrides, but some request handlers continue reading startup-only `AppState` values. Operators also have no settings form, updates are not validated or atomic, and changes have no old/new audit receipt.

## Required behavior

- Admins can view and update the five settings already represented by the API: mastery bands, community minimum sample, free daily questions, free daily Coach turns, and re-test intervals.
- Each setting is validated at the API boundary. One invalid value rejects the complete update.
- A multi-setting update is atomic and writes one audit event with the actor and previous/new override values.
- Reads show effective values: stored override first, then the existing runtime default.
- Request handlers that consume these settings read the effective value so an update applies without restarting the service.
- Learner-visible public settings expose only the effective free daily question limit.
- The admin form uses the existing Editorial Console, design-system tokens, and admin-token gate.

## Test seams

- Authenticated HTTP integration: update, invalid batch rejection, effective readback, public learner-safe setting, community sample gate, free allowance enforcement, re-test scheduling, and audit receipt.
- Playwright: load the console settings, save values, and show validation/server errors accessibly.

## Acceptance checks

- Invalid thresholds, negative allowances, zero community sample, and malformed re-test schedules return 422 without persisting any part of the request.
- Valid settings are returned as effective values and used by request handlers immediately.
- The audit event records the operator, settings entity, prior overrides, and requested values.
- Current formatting, integration, build, and browser verification runs through GitHub Actions after the implementation batch.

## Scope boundaries

Integrity policies and role-aware operator permissions remain separate ADMIN-06
work. Issues 02–05 cover offline lease duration, rights-checked CSV/Excel
question import, adjustable daily review caps, and competition scoring controls;
this settings slice does not claim the complete §19.5 settings catalog.
