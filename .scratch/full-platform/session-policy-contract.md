Status: ready-for-agent
Requirements: CORE-07, CORE-06, TRUST-04

## Problem Statement

The single-session policy can be changed through the API but cannot be read or
managed in Account. Its setter changes the user before retiring sessions;
failure during retirement leaves a policy change despite the failed request.

## Solution

Commit the owned user's policy, immediate session retirement and audit together,
using the same user lock as login. Preserve immediate take-effect semantics:
enabling signs out every session including this browser; the next login survives.
Expose the stored policy through authenticated GET and the existing Account
card pattern. Clearly explain immediate sign-out before the learner enables it.
Do not change navigation, fonts, themes, tokens or mobile primary tabs.

## User Stories

1. As a learner, I want to see the saved session policy rather than an assumed default.
2. As a learner, I want a failed policy change to preserve policy and access.
3. As a learner, I want to understand and confirm the immediate sign-out effect.
4. As a learner, I want retryable load/save errors and no fabricated saved state.

## Testing Decisions

Use the authorized authenticated HTTP and Playwright seams. A scoped disposable
database constraint rejects session retirement. Remove the constraint before
assertions, then use a new login and existing-session access to observe whether
the failed policy change rolled back. Retain the existing successful single
session test. Browser tests cover persisted state, truthful save, confirmation,
sign-out and error recovery. Synthetic fixtures do not establish provider acceptance.
