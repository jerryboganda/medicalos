# LIB-08 — Article media playback

Status: implementation authored; final CI/E2E acceptance pending
Requirement IDs: LIB-08, ARCH-02
Source: `docs/requirements/traceability.md`; `.scratch/lib01-article-authoring/spec.md`

## Problem

Published article responses already include rights-referenced media, caption
cues, and chapter markers. The learner article reader currently renders only
the article text and citations, so the media metadata cannot be used.

## Scope and behavior

- Keep the existing article media table and rights-reference requirement.
- Type media, caption-cue, and chapter-marker DTOs in Rust and export them to
  the client through the existing ARCH-02 `ts-rs` workflow.
- Validate HTTPS media URLs, media kind, duration, caption timing/text, and
  chapter timing/title at the editorial API boundary.
- Render native `<audio>` and `<video>` controls with playback deferred until
  the learner chooses to play. Generate a WebVTT caption track from the
  validated cue list and revoke its object URL when the article changes or the
  reader unmounts.
- Render chapter markers as accessible buttons that seek the associated media
  element. Keep the rights reference visible beside each player.
- Do not cache media bytes or claim media is available offline; the separate
  licensed offline-media package requirement remains open.

## Acceptance

- Valid existing media fixtures serialize in the same article response shape.
- Invalid media/caption/chapter metadata is rejected with a stable 422 code.
- The article reader renders both native media elements, a caption track,
  rights reference, transcript cues, and chapter controls.
- A chapter control seeks only its own media element.
- The article reader remains usable without media and stays within the
  320/375/414/768/1280px viewport widths.
- GitHub Actions passes the API integration, type-export, client, and Playwright
  E2E gates after implementation is complete.

## Verification limits

Synthetic browser tests validate player markup and controls. Actual streaming,
caption synchronization across reference devices, content-rights clearance,
and offline-media licensing require real assets and separate acceptance.

## Implementation record

- Rust now owns the media, caption-cue, and chapter-marker response types and
  exports them through the ARCH-02 contract workflow.
- The editorial endpoint validates credential-free HTTPS URLs, media kind,
  duration, cue text/timing, chapter text/timing, and the required rights
  reference. The reader renders native audio/video controls, generated WebVTT,
  a transcript, seekable chapters, and the visible rights reference.
- API regression cases cover invalid URL credentials, caption timing, and
  duplicate chapter times. Playwright coverage checks both players, generated
  caption contents, seeking, rights labels, and five viewport widths.
- GitHub Actions acceptance remains pending; no real licensed media or device
  streaming evidence is claimed.
