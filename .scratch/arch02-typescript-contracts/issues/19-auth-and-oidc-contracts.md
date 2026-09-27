# ARCH-02 — Authentication and institution OIDC contracts

Status: in-progress (Rust request/response DTOs and authenticated wire assertions authored; final type-export and GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, CORE-01, INST-03
Source: `.scratch/arch02-typescript-contracts/spec.md`; authentication and institution OIDC routes

## Problem Statement

The browser manually declares sign-in and institution OIDC request and response
shapes. These flows exchange bearer tokens and handle provider secrets, so the
wire types must track the server without exposing stored credentials.

## Solution

Generate registration, login, OIDC provider, authorization-start, and ticket
completion contracts from the Rust route DTOs.

## User Stories

1. As a learner, I want the sign-in client to send and receive the server's
   exact shapes so authentication errors are not caused by client drift.
2. As an institution administrator, I want provider configuration responses
   to expose whether a secret exists without returning its value.

## Implementation Decisions

- Add generated request and response DTOs to the existing auth and OIDC route
  modules; consume them from `api.ts`.
- Preserve request validation, admin authorization, encrypted credential
  storage, secret redaction, PKCE, and one-use ticket behavior.
- Do not change route paths, status codes, or authentication behavior.

## Testing Decisions

- Extend the existing authenticated HTTP integration flows with exact response
  key assertions for register/login, provider configure/view, authorization
  start, and ticket completion.
- Keep the secret-redaction assertion and SSO one-use assertions.
- Defer type export, formatting, builds, integration tests, and browser E2E
  execution until implementation slices are complete, as directed by the user.

## Out of Scope

- Adding new identity providers or changing institutional enrollment policy.
- Completing ARCH-02; other client-consumed DTO families remain handwritten.
