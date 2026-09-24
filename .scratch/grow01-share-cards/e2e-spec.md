# GROW-01 — share-card client e2e coverage

Extends `tests/e2e/share-cards.spec.ts` (Playwright, real API) to assert
the community page renders share cards from real data:

1. Fresh learner: the page shows the honest unavailability reasons
   (score/consistency/league) and no cards.
2. After two API-driven answers: the page's score card renders exactly the
   headline the share-cards API returns (page == API — the real-data
   proof), and each consistency/league surface matches the API state
   (card or honest unavailable note).
