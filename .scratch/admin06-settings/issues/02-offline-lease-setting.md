# ADMIN-06 — Configurable offline lease length

Status: ready-for-agent
Type: task
Requirement IDs: ADMIN-06, PROT-02, OFF-04
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §§19.5, 22

## Problem

The admin settings surface cannot change the offline lease window. Pack leases
always expire after 14 days, even when an operator needs a shorter policy.

## Scope

Add `offline_lease_days` to the existing settings API and Editorial Console.
Keep the current 14-day value as the default and allow bounded values from 1
through 30 days. Lease issuance reads the current override and stores the
resulting server-authoritative expiry. Existing leases keep their recorded
expiry; the setting affects newly created or renewed leases only.

## Acceptance

- Admin get/update supports `offline_lease_days`, validates 1–30, and includes
  its old/new values in the existing transactional audit record.
- Invalid or empty batches do not write any setting or audit row.
- New and renewed device-bound leases use the current setting; existing lease
  rows are not silently rewritten when an admin changes the setting.
- The admin form loads, edits, saves, and displays API validation errors for
  the value using existing design tokens and states.
- API integration coverage proves bounds, audit behavior, runtime lease expiry,
  and existing-lease preservation. Playwright covers the settings control.
- The migration registry and schema remain unchanged because `app_settings`
  already stores generic JSON values.

## Verification boundary

Author API and Playwright regressions. Run formatting, builds, tests, and
rollback verification passed in GitHub Actions run 36159484978. Do not run
heavy builds or test suites on local or production hosts.

## Out of scope

Pack encryption, resumable downloads, native device attestation, rights-contract
verification, and changing already-issued lease expiries.

## Implementation record

- Added the setting to the existing admin settings contract and form with a
  14-day default, 1–30 day validation, and old/new audit values. No schema
  change is needed.
- New leases and renewals read the current override; existing lease expiries
  stay unchanged. Public learner config continues to expose only the free
  question allowance.
- API integration and Playwright regressions passed in GitHub Actions run
  [36159484978](https://github.com/jerryboganda/medicalos/actions/runs/36159484978).
