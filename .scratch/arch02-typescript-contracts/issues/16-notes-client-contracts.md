# ARCH-02 — Learner notes client contracts

Status: in-progress (Rust DTOs, generated declarations, and authenticated wire assertions authored; final GitHub Actions acceptance pending)
Requirement IDs: ARCH-02, NOTE-01, NOTE-02, NOTE-03
Source: `.scratch/arch02-typescript-contracts/spec.md`; existing authenticated notes routes

## Problem Statement

The client manually declares the learner note list and response payloads. Those
shapes can drift from the authenticated Rust routes without a compiler error.

## Solution

Generate the request and response declarations for the client-consumed note
create, update, list, and delete endpoints from the Rust wire DTOs. Keep
authentication and route construction in the existing client API module.

## User Stories

1. As a learner, I want my note list and source links to match the server
   response so that notes render reliably across Learn and the session page.
2. As a learner, I want create, update, and delete responses typed from the
   server so that note actions handle their receipts consistently.
3. As a client maintainer, I want note request and response shapes generated
   from Rust so that API changes are caught by contract export and CI.

## Implementation Decisions

- Reuse the existing note request structure and represent the response shapes
  for client-consumed routes with serializable Rust DTOs.
- Preserve current HTTP methods, paths, authentication, JSON field names,
  nullable source IDs, and timestamps.
- Generate declarations under the existing client generated-contract folder.
- Leave note-link and portable-export transport contracts out of this slice
  because the browser client does not call those routes.

## Testing Decisions

- Use the existing authenticated HTTP integration test as the single public
  seam for create, update, list, and delete response shapes.
- Assert exact public keys and representative values, including note source
  nullability and backlink entries; retain its user-isolation checks.
- Use the existing Rust type-export and client build gates to verify generated
  bindings and client consumption. Defer execution until all implementation
  slices are authored, as directed by the user.

## Out of Scope

- Changing note privacy, persistence, conflict resolution, link behavior,
  export behavior, or the user interface.
- Completing the ARCH-02 requirement; other client-consumed API contract
  families remain handwritten.
