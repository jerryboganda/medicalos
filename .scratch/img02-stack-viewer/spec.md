# IMG-02 — Rights-gated image stack viewer and reviewed annotations

Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §15

## Problem

The image-case API historically accepted a non-empty rights reference without checking whether a current grant allowed display. Learners had no ordered still/stack viewer or a reviewed annotation workflow. Findings were included in the learner list response.

## Required behavior

- Require an active, non-revoked `display` grant for every image when creating a case and when serving learner list/detail responses. Hide the complete case if any image grant is unavailable.
- Preserve the order of each case’s image array. Render source files with native `<img>` and contain-fit layout. Zoom may use CSS; never filter, recolor, rewrite, resample, or claim to de-identify image bytes.
- Let learners move through a stack with Previous/Next, the keyboard-operable sequence slider, and the scroll wheel while the pointer is over the image stage.
- Keep findings out of list summaries. Return them only from a rights-checked detail request after the learner chooses to reveal them.
- Represent findings as an ordered list of labeled plain-text sections. Migrate existing single-string findings into one legacy “Findings” section.
- Allow an authorized editor to add a plain-text annotation to an image index and normalized x/y coordinates. Reject HTML-like tags and scripts.
- Store one immutable reviewer decision separately from the annotation. Require a different authenticated admin reviewer and a 1–500 character plain-text review note. Learners see only approved annotations while the case’s display rights remain active.
- Keep the UI responsive and keyboard-operable at 320, 375, 414, and 768 px. State that teaching content and editorial review do not establish clinical validity or authorize patient care.

## Acceptance checks

- API integration coverage exercises missing, non-display, future, expired, and revoked grants; multi-image cases; hidden findings in summaries; ordered structured detail disclosure; bounds validation; plain-text annotations; independent review; and one-time immutable decisions.
- Playwright covers ordered image navigation by buttons, slider, and wheel; slider and zoom keyboard controls; labeled findings disclosure; approved annotation visibility; editor submission; required review notes; and narrow viewport overflow.
- Final formatting, integration, build, and browser verification runs through GitHub Actions. No local build or test run is part of this implementation slice.

## Remaining boundaries

DICOM metadata/burned-in identifier review, consent and clinical provenance, DICOM window/level, normal/abnormal comparison, expert clinical validation, and reference-device acceptance remain separate gates. The viewer does not fetch or proxy publisher media.
