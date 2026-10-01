# Enforce content rights for offline packs and cached study material

Status: needs-triage

## Outcome

Offline question packs, browser caches, and downloaded derivatives obey the
grant and learner/device scope under an explicitly accepted offline-access
policy.

## Acceptance criteria

- Inventory pack creation, manifests, downloads, receipts, leases, local
  IndexedDB storage, sync, and derived material that can remain on a device.
- Bind every pack to current display rights, audience, asset scope, device, and
  an expiry that the client enforces without relying on its wall clock.
- Specify the revocation behavior while offline and the maximum unavoidable
  access window; do not imply that server revocation can erase a disconnected
  device's copy.
- Reject stale or mismatched receipts and prevent renewed access after a rights
  check fails.
- Require owner acceptance of the offline license policy and exact-source
  GitHub Actions coverage before marking this issue complete.

## Spec

See `../spec.md`.

## Comments
