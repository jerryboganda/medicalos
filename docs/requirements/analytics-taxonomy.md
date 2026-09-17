# Product-Analytics Taxonomy (master plan Appendix B)

Product analytics are a **separate stream from learning evidence** (§21.3). Keyed by a pseudonymous ID. **Never feed the learner model.** User properties: exam, plan, days to exam, cohort.

## Events

| Event | Key properties |
|---|---|
| app_open | source, deep_link, surface (web, desktop, ios, android) |
| onboarding_step_completed | step |
| baseline_completed | duration |
| activation_achieved | hours_since_first_use |
| session_started | preset, pool, source (today, builder, smart, review), question_count, offline |
| question_answered | correct, difficulty, seconds, confidence, assisted, offline |
| explanation_viewed | seconds |
| session_submitted | score, duration, auto_submitted |
| review_completed | items, retention |
| plan_revision_viewed, plan_revision_undone | automatic, reason_code |
| coach_turn | prompt_type (pre-generated or free-form), rating |
| daily_goal_met, streak_extended, streak_lost | streak_length |
| mock_completed | template, score, percentile |
| progress_viewed | level |
| paywall_viewed | trigger, variant |
| purchase_completed | product, route, price_tier |
| subscription_renewed, subscription_cancelled | product, tenure |
| pack_downloaded | size, network_type |
| sync_completed | events, duration, failures |
| notification_opened | category, campaign |
| question_reported | reason |
| competition_joined, duel_created, duel_completed | type, result |
| share_card_sent | type, channel |
| integrity_event | type, test_type |

## Rules

1. Pseudonymous key only — no direct identifiers in the analytics stream.
2. Learning evidence (§21.3 events) and product analytics never mix streams; derived analytics state is recomputable from the learning stream alone.
3. The learner model consumes learning evidence only. Nothing in this file may influence recommendations, plans, or mastery estimates.
4. Health metrics built on these events (§2.4) monitor the product; none may become the agent's optimization target.
