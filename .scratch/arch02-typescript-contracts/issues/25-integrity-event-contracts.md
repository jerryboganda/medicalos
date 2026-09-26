# ARCH-02 issue 25: integrity-event contracts

Status: implementation authored; generated bindings and CI acceptance pending
Requirement IDs: ARCH-02, EX-08
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

The browser's integrity-event request and response types duplicate the server
wire shapes. Response fields are conditional, and an auto-submit receipt is
replayed from durable JSON that can predate newer optional receipt fields.

## Acceptance

- Rust owns the integrity-event request and response DTO declarations.
- Optional session, detail, client-time, away-time, and receipt fields match
  existing omitted/null behavior.
- The signal and action unions include every server-supported value without
  changing runtime validation.
- Receipt stays opaque JSON so older persisted receipt versions remain
  replayable.
- Authenticated integration coverage asserts exact response keys for the
  ordinary, warning, and auto-submitted cases, including the auto-submit
  receipt's stable required envelope.
- The client uses generated types and final export/client/API gates pass in
  GitHub Actions.

## Seams

- `POST /v1/integrity-events`

## Out of scope

Changing signal policy, mock enforcement, stored-receipt compatibility, or
device attestation behavior.
