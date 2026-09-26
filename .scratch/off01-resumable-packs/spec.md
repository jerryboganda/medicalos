# OFF-01 browser pack downloads

Status: implementation in progress
Requirement IDs: OFF-01, OFF-04, PROT-02 (browser-storage slice)
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §22

## User outcome

Paid learners can choose chapters, download their published practice material,
resume an interrupted download, and reopen the saved pack while disconnected.
The browser surface is explicitly best effort: the browser may evict storage,
the offline lease expires, and institutional/high-stakes assessments remain
online-only.

## Contract

- Keep `/v1/packs/*` response behavior unchanged for installed clients.
- Upgrade the lease-bound `/v2/packs/{exam_id}/manifest` to manifest version 4.
  Sign the canonical exam/device/chapter/item listing with Ed25519. Derive the
  32-byte signing seed from the configured server secret with a versioned,
  domain-separated SHA-256 derivation. Return the raw verification key and its
  key ID with the signature; the authenticated TLS response establishes the
  initial browser trust anchor. The browser verifies the signature before it
  stores the manifest and its verification key with the pack.
- Bind each manifest item to a checksum of its complete offline practice
  resource (question text, options and rationales, answer key, learning point,
  exam tip, and only eligible pre-generated tutoring cards).
- Add an authenticated, lease-checked batch resource endpoint. It accepts at
  most 50 unique IDs and returns only published resources within the leased
  exam/chapter scope. A resource's recomputed checksum must match its signed
  manifest entry.
- Persist manifest, encrypted resources, per-device non-extractable AES-GCM
  key, and lease metadata in IndexedDB. Commit complete validated batches
  atomically; retries fetch only missing or changed items. Keep one active
  browser pack per exam/device and union chapter scope on additions.
- Bound browser packs to 500 questions. Check browser quota before every
  batch, expose saved bytes/expiry, offer storage persistence when available,
  and let learners remove local packs. Browser storage remains evictable.
  - Cache SvelteKit build/static assets and the `/offline` route. For navigation,
  try the network first and fall back to the cached `/offline` route; never cache
  authenticated API replies.

## Acceptance

- Legacy v1 manifest remains metadata-only and unchanged.
- API tests prove Ed25519 verification, tamper failure, key identity, complete
  resource checksums, entitlement/device/chapter scope, batch limits, and
  revocation refusal.
- Browser E2E covers chapter selection, lease creation, verified download,
  interruption/retry without duplicate writes, reload and offline reopen,
  answer reveal from cached content without recording a local attempt, quota
  refusal, expired/revoked status, pack deletion, and widths
  320/375/414/768/1280.
- No local builds, tests, screenshots, or generated artifacts during this
  implementation batch. GitHub Actions is the final acceptance gate.

## Explicit boundary

Browser AES-GCM is per-device at-rest protection, not hardware-backed key
storage. The PWA is best effort and can be evicted. Native encrypted SQLite,
attestation, rights-verified receipts, device-matrix validation, and
institutional/high-stakes offline exams remain separate acceptance work.

## Implementation record

- Rust serves lease- and device-scoped Ed25519 manifests and bounded question
  batches whose hashes cover every returned practice field. The legacy v1
  manifest remains metadata-only.
- The browser verifies the manifest and each complete resource, encrypts
  batches with a per-device AES-GCM key in IndexedDB, resumes only missing
  items, and exposes storage quota, expiry, and removal behavior.
- The offline reader is review-only. Scored practice starts through the
  existing authenticated server session API, and the service worker excludes
  API responses from its caches.
- Authored API and Playwright cases cover signed-resource integrity, lease
  scope, resumable batches, offline reopen, review-only behavior, synced
  session start, storage limits, removal, and five viewport widths.
- GitHub Actions acceptance remains pending; browser storage, device
  attestation, signed download receipts, and licensed offline media remain
  separate acceptance boundaries.
