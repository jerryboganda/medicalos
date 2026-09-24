# CORE-03 tails + EX-01 tails — library entitlements and registry fixtures

Requirements: CORE-03 (media/retrieval entitlements), EX-01 (registry
aliases + official-source fixture assertions), COM-01 (entitlement gates)
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §26.1, §29; ledger rows
CORE-03 and EX-01.

## Scope

Complete the two named tails in the ledger:

1. **CORE-03 media/retrieval entitlements.** The free tier's library
   surface (search queries and article opens) is metered like the question
   allowance: `FREE_DAILY_LIBRARY` (default 10) retrievals per day for
   `tier = 'free'` users, enforced server-side on `GET /v1/library/search`
   and `GET /v1/library/articles/{slug}` with the house 403 + structured
   details (`library_allowance_reached`, limit/used/remaining). Paid tiers
   and admin-token operations are unmetered. Offline manifests and the
   question allowance keep their existing gates.
2. **EX-01 fixture assertions.** The seeded exam fixture carries
   `official_source_url` and `aliases`; the registry list assertions pin
   them so the official-source contract cannot silently regress.

## Acceptance

- Free user: within allowance, search and article opens behave normally and
  count usage; past the allowance, both endpoints return 403 with
  `library_allowance_reached` and honest counter details.
- Paid-tier user (tier ≠ free): no library metering.
- Registry list asserts the fixture exam's official source URL and aliases.
- All existing integration tests stay green; CI fully green before main.

## Out of scope

Media byte-streaming (no playback endpoint exists yet — LIB-08 is
metadata-only), paid-tier feature differences (COM-02), coupons/regional
pricing (COM-03, owner-gated).
