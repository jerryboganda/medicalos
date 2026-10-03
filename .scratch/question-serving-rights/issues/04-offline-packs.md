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

- Exact-source Actions run [36990426238](https://github.com/jerryboganda/medicalos/actions/runs/36990426238) passed all six jobs on `7dfd41e25df75485e37bab5affad69603b966b4a`. Resource-batch receipts now bind a fresh request challenge into the server signature, and the browser rejects stale/replayed challenges before saving content.
- The trusted offline expiry/revocation window and owner-approved policy are still unresolved; this issue remains open.

## Comments
