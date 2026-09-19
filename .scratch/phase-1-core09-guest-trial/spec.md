# CORE-09 — Guest trial before sign-up

## Problem Statement

Visitors currently have to create and verify an account before they can experience the question workflow. The Phase 1 plan calls for an optional guest trial that lets a visitor answer a few safe sample questions first, while preserving any answered progress when they later create an account.

## Solution

Expose a tightly bounded unauthenticated guest-trial seam that serves at most three deterministic published, non-quarantined sample questions. The server issues a short-lived opaque token, stores only its hash, enforces first-answer-wins/idempotent replay, and never exposes arbitrary question-bank selection. Registration may include that guest token; answered guest items are then migrated into the existing practice-session and attempt records for the new learner in the same transaction.

The login surface gains a compact “Try sample questions” path. Guest answer feedback follows the existing tutor semantics: answer keys and rationales are hidden before answering and revealed only for the answered item.

## User Stories

1. As a visitor, I want to try a few questions without an account, so that I can judge the learning experience before signing up.
2. As a visitor, I want sample questions to show only the vignette, lead-in, difficulty, and option text before I answer, so that answer keys are not leaked.
3. As a visitor, I want immediate feedback after answering, so that the trial demonstrates the normal tutor experience.
4. As a visitor, I want refreshing or repeated answer submission with the same idempotency key to be safe, so that network retries do not duplicate progress.
5. As a visitor, I want a second different answer for an already-answered sample to be rejected, so that first-answer-wins semantics match normal practice.
6. As a visitor, I want at most three sample items, so that the trial is useful without becoming a question-bank scraping route.
7. As a visitor, I want repeated trial starts to expose the same bounded deterministic sample set, so that creating many trial tokens cannot enumerate the full question bank.
8. As a visitor, I want quarantined or unpublished content excluded, so that unsafe editorial content is never used as a sample.
9. As a visitor, I want the trial token to expire, so that abandoned guest state is not indefinitely usable.
10. As a visitor, I want my answered trial questions attached to my learning record when I create an account, so that signing up does not discard my work.
11. As a new learner, I want only questions I actually answered migrated, so that untouched samples are not recorded as skipped attempts.
12. As a new learner, I want migrated progress stored in the same practice/attempt model as ordinary study, so that downstream progress features do not need a guest-specific analytics path.
13. As a new learner, I want registration to remain atomic with guest migration, so that I cannot end up with an account created but partially migrated trial data.
14. As a security owner, I want raw guest tokens kept out of durable storage, so that a database read does not reveal reusable trial credentials.
15. As a security owner, I want guest tokens to authorize only guest-trial operations, so that a visitor cannot call authenticated learner APIs.
16. As a product owner, I want this feature implemented without a second quiz engine or new dependency, so that Phase 1 remains maintainable.

## Implementation Decisions

- Add one guest-trial route module with start and answer endpoints; no guest authentication extractor and no guest access to authenticated routes.
- Persist trial metadata and the fixed item set in dedicated tables because normal practice tables require a real user account.
- Use the existing opaque-token generator and SHA-256 token hashing; guest tokens live for 24 hours.
- Select a stable set of at most three published, non-quarantined questions ordered deterministically, preferring high-yield content when available.
- Store chosen answer, correctness, and idempotency key on the guest item row; first answer wins.
- Reuse the existing question option type and tutor-feedback payload shape rather than creating a parallel feedback contract.
- Extend registration with an optional guest trial token. If valid and unconverted, copy only answered guest items into one submitted ordinary practice session and ordinary attempts, then mark the trial converted, all within the registration transaction.
- Invalid, expired, or already-converted guest tokens supplied during registration fail closed rather than silently dropping progress.
- Do not add IP fingerprinting, CAPTCHA, a scheduler, or a rate-limit service in this slice. The bounded deterministic three-question set prevents token farming from exposing additional bank content; infrastructure-level request throttling remains an operational concern.
- Keep deployment out of scope.

## Testing Decisions

- Test at the HTTP API seam, matching the repository’s existing integration-test convention.
- RED contract proves: unauthenticated start, maximum three pre-answer-safe items, answer feedback, idempotent replay, first-answer-wins, no arbitrary fourth item, and registration-time migration acknowledgment.
- Migration correctness is additionally checked through persisted ordinary practice/attempt records because downstream learner features consume those records as their source of truth.
- Lightweight local verification is limited to formatting/diff/static checks; heavy database/browser acceptance belongs in GitHub Actions.

## Out of Scope

- Social login, production email delivery, CAPTCHA, WAF/IP rate limiting, device fingerprinting, marketing analytics, experimentation, deployment, and a guest-accessible version of the full qbank builder/session UI.

## Further Notes

- CORE-09 is a SHOULD requirement, but once implemented it must obey the same answer-secrecy and quarantined-content rules as ordinary practice.
- The trial is intentionally a product sample, not an anonymous learner account.
