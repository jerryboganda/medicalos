## Problem Statement

Question and explanation text in the web practice session has no account-linked watermark, so copied text cannot be traced to the account that viewed it.

## Solution

Expose the authenticated owner ID in that owner's practice-session detail response and tile a faint, non-interactive identifier over question and explanation text in the session view. Use the session ID as a clearly labelled fallback for older offline drafts. Keep diagnostic images and anatomy free of overlays. The watermark does not claim to prevent browser capture.

## User Stories

1. As a learner, I want my account identifier faintly repeated over question text so that copied text can be traced without making study harder.
2. As a learner, I want the same identifier on revealed rationales and learning points so that explanations retain their provenance when copied.
3. As a learner using an older offline session, I want a traceable session identifier when no account ID was cached so that the content still carries provenance.
4. As a learner using a screen reader, keyboard, zoom, or text scaling, I want the watermark to remain decorative and non-interactive so that it does not obstruct learning or controls.
5. As a learner viewing diagnostic images or anatomy, I want those images to remain unobscured.

## Implementation Decisions

- The authenticated session-detail response includes the owner UUID only after the existing owner-scoped lookup succeeds.
- The existing practice-session route renders the account identifier as a faint repeated overlay on question prose and revealed explanation prose.
- Older cached session details fall back to a session UUID and label it as a session identifier.
- The watermark is hidden from assistive technology, does not receive pointer events, and uses existing design tokens.
- Browser capture prevention, native secure-window controls, screenshot detection, and device attestation remain separate platform work.

## Testing Decisions

- API integration coverage verifies that an authenticated learner receives their own account ID in session detail.
- The existing learner-loop browser test verifies the watermark on question and revealed explanation text and checks that its identifier is UUID-shaped.
- Tests observe the HTTP and rendered browser surfaces, not internal rendering helpers.
- Existing imaging routes remain outside the modified surface; no image watermarking is introduced.

## Out of Scope

- Preventing screenshots or screen recording in browsers.
- Android, iOS, Windows, or macOS capture controls and device attestation.
- Watermarking diagnostic images, anatomy, or licensed media.
- Native/reference-device acceptance.

## Further Notes

This implements the web text-watermark portion of PROT-01. It cannot complete PROT-01's platform capture controls or device acceptance.
