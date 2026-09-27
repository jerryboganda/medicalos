# ARCH-02 — JSON numeric wire types

Status: implementation authored; Rust-owned regeneration and GitHub Actions acceptance pending
Requirement IDs: ARCH-02
Source: `.scratch/arch02-typescript-contracts/spec.md`

## Problem

Four client-consumed DTOs contain Rust `i64`/`u64` properties that currently
export as TypeScript `bigint`, while the JSON API serializes them as JSON
numbers and JavaScript's JSON parser provides them as `number`. This leaves
the generated declarations different from the values clients receive at
runtime.

## Acceptance

- `AdminDashboard` aggregate counts, media caption/chapter timestamps, and
  `LibrarySearchResult.score` export as TypeScript `number` from their Rust DTOs.
- API JSON field names, numeric values, and endpoint behavior remain unchanged.
- GitHub Actions regenerates the bindings from Rust, checks generated-file
  drift, builds the client, and passes the existing API and browser suites.
- Generated TypeScript files are produced by the CI exporter, not edited by
  hand.

## Testing seam

Use the existing `type-export` generation and drift gate, plus the current
authenticated API and Playwright coverage for admin counts, media metadata,
and library search. Do not add a parallel generator or change the HTTP
transport.
