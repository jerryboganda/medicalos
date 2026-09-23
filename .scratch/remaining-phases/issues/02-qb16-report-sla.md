Status: ready-for-human
Requirements: QB-08, QB-16, ADMIN-06
Implementation state: complete; final CI and browser verification pending.

# Report resolution, visible SLAs, and reporter feedback

## Acceptance

- A successful report is acknowledged immediately, and the reporter can see the acknowledgement time and the 24-hour acknowledgement / 72-hour resolution deadlines.
- Admins can list unresolved reports as mobile-friendly grouped cards with question context, vote count, age, SLA state, and up to 20 anonymized feedback notes per group.
- A report group resolves atomically as rejected or fixed. A fixed resolution requires a newer published version of the same question and a separate public correction changelog; the old version remains out of new pools.
- Resolution stores the actor and private reporter note, audits the change, updates every open/quarantined report for that question version, and sends each reporter an in-app notification respecting their report preference.
- A fixed decision also stores a separate public correction changelog on the resolution record; it never reuses or exposes the private reporter note. A reporter can read their own note and corrected version while other reporters remain private.
- Repeated resolution attempts and invalid decisions fail with stable API errors; unresolved items remain quarantined.

## Seams

- HTTP integration tests for report, review queue, resolution, SLA fields, privacy, and notifications.
- The existing admin Svelte page for queue and resolution controls; Playwright mocks exercise loading, empty, error, and success states.

## Out of scope

Automated clinical review, guaranteeing staff meet the operational target, report-email delivery, and source-change propagation beyond refusing to restore an old version as current content.
