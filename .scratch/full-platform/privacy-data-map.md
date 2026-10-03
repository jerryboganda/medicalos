# Account archive implementation map

Status: source inventory; export implementation pending.
Current HTTP seam: GET /v1/me/export in routes/packs.rs.
Current export includes only profile email/tier/created_at, selected attempt
fields, note text, review-event fields and portfolio fields. Account UI describes
this as partial. These are source findings, not privacy/legal acceptance.

| Data category | Existing storage requiring an explicit export decision |
|---|---|
| Account preferences and accessibility | users policy fields, learner_accommodations, notification_preferences, engagement_settings |
| Study sessions and learning evidence | practice_sessions, session_items, attempts, mock_attempts, learner_concept_state, question_marks, question_reports, retest_cards, retest_history |
| Recall materials | decks, cards, review_events; owned notes, note_links, note_collections and note_collection_items |
| Planning and interventions | goals, protected_commitments, plans, plan_tasks, plan_revisions, intervention_outcomes |
| Coach | coach_turns and coach_memory; preserve grounding and contribution metadata without exporting provider secrets |
| Notifications and engagement | notifications, engagement_days, qotd_answers, xp_ledger, achievements |
| Library and private imports | article_reads, source_change_task_learners, private_documents; legal scope of licensed binary/text copies requires an explicit decision |
| Simulations and professional evidence | scenario_runs and owned event/assessment records, scenario_team_members, appeals, supervised_feedback, portfolio_entries, ce_activities, exam_outcomes |
| Community and competitions | community_profiles, owned posts/reports, group memberships, owned duel participation, competition_entries/attempts, league memberships; do not include other learners' private rows |
| Identity and device history | user_devices, external_identities, lti_identities, pack_leases, pack_download_receipts; export safe metadata only |
| Access and commercial records | entitlement_usage, referral ownership and applicable institutional membership/assignment/receipt records |

This table follows the migration definitions, including 0001, 0004, 0008,
0009, 0010, 0013-0019, 0023, 0026, 0037, 0039, 0044, 0050-0052,
0058 and 0063. It is an implementation starting map, not a claim that every
indirect relationship or legal export right is already resolved.

## Archive acceptance contract

1. Version the archive and enumerate included and excluded categories honestly.
2. Select every row through owned foreign keys or the authenticated user's
   explicit membership/contribution. Test two-user and cross-institution denial.
3. Keep passwords, bearer/session hashes, one-use tickets, shared duel/invite
   tokens, provider credentials and signing material out of the archive.
4. Use one consistent read snapshot so relationships remain reconstructable.
   Preserve stable ids, version/provenance, dispositions and timestamps.
5. Preserve shared-content rights: export learner-owned metadata/annotations
   without granting a new license to protected content or other users' text.
6. Define size limits and asynchronous delivery for large accounts; no silent
   truncation or partial-success claim. Stream or bound memory and failure paths.
7. Verify complete category coverage through authenticated HTTP and browser
   download seams in disposable CI. Add future tables to this inventory.
8. Account erasure, retention exceptions, shared contributions, audit retention,
   backups and operational/legal time limits need the approved owner policy.
   Access disablement is not proof of deletion from those stores.
