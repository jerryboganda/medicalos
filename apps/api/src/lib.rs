//! Phase 1 slice 1: the connected question loop (see
//! .scratch/phase-1-slice-1/spec.md). The HTTP API is the tested seam.

pub mod agent;
pub mod auth;
pub mod error;
pub mod routes;
pub mod schema;
pub mod seed;
pub mod state;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;

pub fn router(state: Arc<state::AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        // Same-origin production prefix: nginx serves the API under /api/
        // (medicalos.polytronx.com/api/* -> medicalos-api:8080/*).
        .route("/api/healthz", get(health_json))
        .route("/api/version.json", get(version))
        .route("/v1/auth/register", post(routes::auth::register))
        .route("/v1/auth/login", post(routes::auth::login))
        .route(
            "/v1/auth/oidc/callback/{institution_id}",
            get(routes::oidc::callback),
        )
        .route("/v1/auth/oidc/complete", post(routes::oidc::complete))
        .route(
            "/v1/me/devices",
            post(routes::accounts::register_device).get(routes::accounts::list_devices),
        )
        .route(
            "/v1/me/devices/{device_id}",
            delete(routes::accounts::revoke_device),
        )
        .route("/v1/me/account", delete(routes::accounts::delete_account))
        .route("/v1/exams", get(routes::exams::list_exams))
        .route(
            "/v1/admin/exams/{exam_id}/specs",
            post(routes::exams::create_exam_spec),
        )
        .route(
            "/v1/admin/exam-specs/{spec_id}/forms",
            post(routes::exams::create_assessment_form),
        )
        .route(
            "/v1/me/accommodations",
            post(routes::exams::set_accommodation),
        )
        .route("/v1/me/outcomes", post(routes::exams::add_outcome))
        .route("/v1/me/readiness", get(routes::exams::readiness))
        .route(
            "/v1/calculators/{kind}",
            post(routes::calculators::calculate),
        )
        .route(
            "/v1/calculators/convert",
            post(routes::calculators::convert),
        )
        .route("/v1/me/today", get(routes::today::today))
        .route("/v1/me/plan/next-action", get(routes::today::next_action))
        .route(
            "/v1/me/engagement",
            get(routes::engagement::engagement_status),
        )
        .route(
            "/v1/me/engagement/settings",
            axum::routing::put(routes::engagement::update_engagement_settings),
        )
        .route("/v1/qotd", get(routes::engagement::qotd))
        .route("/v1/me/qotd", get(routes::engagement::qotd))
        .route("/v1/me/qotd/answers", post(routes::engagement::answer_qotd))
        .route(
            "/v1/practice/sessions",
            post(routes::practice::create_session),
        )
        .route(
            "/v1/practice/sessions/{sid}",
            get(routes::practice::get_session),
        )
        .route(
            "/v1/practice/sessions/{sid}/answers",
            post(routes::practice::answer),
        )
        .route(
            "/v1/practice/sessions/{sid}/items/{item_index}/hint",
            get(routes::practice::hint),
        )
        .route(
            "/v1/practice/sessions/{sid}/submit",
            post(routes::practice::submit),
        )
        .route(
            "/v1/plans/{pid}/revisions/{rid}/undo",
            post(routes::today::undo_revision),
        )
        .route(
            "/v1/plans/{pid}/tasks/{task_id}/protection",
            put(routes::today::set_task_protection),
        )
        .route(
            "/v1/questions/versions/{vid}/reports",
            post(routes::reports::report).get(routes::reports::my_reports),
        )
        .route("/v1/reports/{rid}/resolve", post(routes::reports::resolve))
        .route(
            "/v1/mocks",
            post(routes::mock::create_mock).get(routes::mock::list_mocks),
        )
        .route("/v1/mocks/{mid}/start", post(routes::mock::start_mock))
        .route(
            "/v1/questions/versions/{vid}/community-stats",
            get(routes::practice::community_stats),
        )
        .route(
            "/v1/admin/hierarchy",
            post(routes::admin::create_node).get(routes::admin::list_nodes),
        )
        .route(
            "/v1/admin/hierarchy/{node_id}",
            axum::routing::patch(routes::admin::update_node),
        )
        .route(
            "/v1/admin/concepts",
            get(routes::concepts::list).post(routes::concepts::create),
        )
        .route(
            "/v1/admin/concepts/{concept_id}/versions",
            post(routes::concepts::create_version),
        )
        .route(
            "/v1/admin/hierarchy/{node_id}/concepts",
            get(routes::concepts::node_mappings).put(routes::concepts::set_node_mappings),
        )
        .route(
            "/v1/admin/questions",
            post(routes::admin::create_question).get(routes::admin::search_questions),
        )
        .route("/v1/admin/import", post(routes::admin::import))
        .route(
            "/v1/admin/import/{batch_id}/rollback",
            post(routes::admin::rollback_import),
        )
        .route("/v1/admin/audit", get(routes::admin::audit_log))
        .route("/v1/admin/reports", get(routes::reports::review_queue))
        .route(
            "/v1/admin/assessment-workflow",
            post(routes::admin::assessment_workflow),
        )
        .route(
            "/v1/assessments/{form_id}/sessions",
            post(routes::exams::start_assessment_session),
        )
        .route(
            "/v1/community/profile",
            post(routes::community::create_profile),
        )
        .route("/v1/community/me", get(routes::community::my_profile))
        .route(
            "/v1/community/groups",
            post(routes::community::create_group).get(routes::community::list_groups),
        )
        .route(
            "/v1/community/groups/{group_id}/join",
            post(routes::community::join_group),
        )
        .route(
            "/v1/community/groups/{group_id}/posts",
            post(routes::community::create_post).get(routes::community::list_posts),
        )
        .route(
            "/v1/community/groups/{group_id}/posts/{post_id}",
            axum::routing::delete(routes::community::remove_post),
        )
        .route("/v1/community/duels", post(routes::community::create_duel))
        .route(
            "/v1/community/duels/by-token/{token}",
            get(routes::community::duel_by_token),
        )
        .route(
            "/v1/community/duels/{duel_id}",
            get(routes::community::duel_state),
        )
        .route(
            "/v1/community/duels/{duel_id}/accept",
            post(routes::community::accept_duel),
        )
        .route(
            "/v1/community/duels/{duel_id}/decline",
            post(routes::community::decline_duel),
        )
        .route(
            "/v1/competitions/{comp_id}/leaderboard",
            get(routes::community::competition_leaderboard),
        )
        .route(
            "/v1/competitions/{comp_id}/claim",
            post(routes::community::claim_prize),
        )
        .route(
            "/v1/admin/competitions/{comp_id}/prize-review",
            post(routes::community::prize_review),
        )
        .route("/v1/me/plan/replan", post(routes::program::replan_plan))
        .route(
            "/v1/me/exam-switch/{to_exam_id}/gap-report",
            get(routes::program::exam_switch_gap_report),
        )
        .route(
            "/v1/me/selection-policy",
            get(routes::program::selection_policy),
        )
        .route("/v1/me/review-debt", get(routes::review::review_debt))
        .route(
            "/v1/me/session-policy",
            axum::routing::patch(routes::accounts::set_session_policy),
        )
        .route(
            "/v1/admin/articles/{article_id}/media",
            post(routes::library::attach_media),
        )
        .route(
            "/v1/admin/image-cases",
            post(routes::library::create_image_case),
        )
        .route("/v1/me/image-cases", get(routes::library::list_image_cases))
        .route(
            "/v1/analytics/events",
            post(routes::analytics::receive_events),
        )
        .route("/v1/admin/dashboard", get(routes::admin::dashboard))
        .route(
            "/v1/admin/content-rights",
            post(routes::admin::create_content_rights).get(routes::admin::list_content_rights),
        )
        .route(
            "/v1/admin/content-rights/{rights_id}/revoke",
            axum::routing::patch(routes::admin::revoke_content_rights),
        )
        .route(
            "/v1/admin/library/extraction-reports",
            get(routes::admin::list_extraction_reports)
                .post(routes::admin::create_extraction_report)
                .layer(DefaultBodyLimit::max(3 * 1024 * 1024)),
        )
        .route(
            "/v1/admin/library/extraction-reports/{report_id}",
            get(routes::admin::get_extraction_report),
        )
        .route(
            "/v1/admin/library/extraction-reports/{report_id}/review",
            post(routes::admin::review_extraction_report),
        )
        .route("/v1/admin/ai-admin", get(routes::admin::ai_admin))
        .route(
            "/v1/admin/incidents",
            post(routes::admin::create_incident).get(routes::admin::list_incidents),
        )
        .route(
            "/v1/admin/incidents/{incident_id}",
            axum::routing::patch(routes::admin::update_incident),
        )
        .route("/v1/client-update", get(routes::config::client_update))
        .route("/v1/me/curriculum", get(routes::today::my_curriculum))
        .route(
            "/v1/community/profiles/{handle}",
            get(routes::community::profile_by_handle),
        )
        .route("/v1/me/duels", get(routes::community::my_duels))
        .route("/v1/me/trends", get(routes::insights::accuracy_trends))
        .route(
            "/v1/admin/questions/{question_id}/versions",
            post(routes::admin::create_variant),
        )
        .route(
            "/v1/admin/recovery-drills/run",
            post(routes::admin::run_recovery_drill),
        )
        .route(
            "/v1/admin/recovery-drills",
            get(routes::admin::list_recovery_drills),
        )
        .route(
            "/v1/admin/coach-regression/run",
            post(routes::admin::run_coach_regression),
        )
        .route(
            "/v1/admin/coach-regression",
            get(routes::admin::list_coach_regression),
        )
        .route(
            "/v1/admin/qti/packages/{exam_id}",
            get(routes::exams::qti_export),
        )
        .route("/v1/me/institutions", get(routes::program::my_institutions))
        .route(
            "/v1/admin/settings",
            axum::routing::patch(routes::settings::update_settings)
                .get(routes::settings::get_settings),
        )
        .route(
            "/v1/admin/psychometrics/{vid}",
            get(routes::admin::psychometric_screening),
        )
        .route("/v1/coach/turns", post(routes::coach::coach_turn))
        .route("/v1/coach/history", get(routes::coach::history))
        .route("/v1/me/coach-memory", get(routes::coach::list_memory))
        .route(
            "/v1/me/coach-memory/{key}",
            axum::routing::put(routes::coach::put_memory).delete(routes::coach::delete_memory),
        )
        .route(
            "/v1/me/interventions",
            post(routes::coach::create_intervention),
        )
        .route(
            "/v1/me/interventions/{id}",
            axum::routing::patch(routes::coach::measure_intervention),
        )
        .route(
            "/v1/coach/answerable-questions",
            get(routes::coach::answerable_questions),
        )
        .route(
            "/v1/notes",
            post(routes::notes::create_note).get(routes::notes::list_notes),
        )
        .route(
            "/v1/notes/{note_id}",
            axum::routing::patch(routes::notes::update_note).delete(routes::notes::delete_note),
        )
        .route("/v1/notes/link", post(routes::notes::link_notes))
        .route("/v1/notes/export", get(routes::notes::export_notes))
        .route("/v1/library/search", get(routes::library::search))
        .route(
            "/v1/library/articles/{slug}",
            get(routes::library::get_article),
        )
        .route(
            "/v1/me/library/import-rights",
            get(routes::library::private_import_rights),
        )
        .route(
            "/v1/me/library/imports",
            get(routes::library::list_private_imports)
                .post(routes::library::create_private_import)
                .layer(DefaultBodyLimit::max(3 * 1024 * 1024)),
        )
        .route(
            "/v1/me/library/imports/{document_id}",
            get(routes::library::get_private_import).delete(routes::library::delete_private_import),
        )
        .route(
            "/v1/admin/source-passages",
            post(routes::source_changes::register_passage),
        )
        .route(
            "/v1/admin/source-passages/{passage_id}/dependencies",
            post(routes::source_changes::link_dependency),
        )
        .route(
            "/v1/admin/source-passages/{passage_id}/changes",
            post(routes::source_changes::record_change),
        )
        .route(
            "/v1/admin/source-changes/{event_id}",
            get(routes::source_changes::get_change),
        )
        .route(
            "/v1/admin/source-change-tasks/{task_id}",
            axum::routing::put(routes::source_changes::resolve_task),
        )
        .route(
            "/v1/me/notifications",
            get(routes::inbox::inbox).patch(routes::inbox::update_preferences),
        )
        .route(
            "/v1/me/notifications/{note_id}/read",
            post(routes::inbox::mark_read),
        )
        .route(
            "/v1/me/goals",
            post(routes::goals::create_goal).get(routes::goals::list_goals),
        )
        .route("/v1/me/commitments", post(routes::goals::add_commitment))
        .route(
            "/v1/me/commitments/{id}",
            axum::routing::delete(routes::goals::remove_commitment),
        )
        .route("/guest/trial/start", post(routes::guest::start))
        .route(
            "/guest/trial/next-question",
            post(routes::guest::next_question),
        )
        .route(
            "/v1/questions/versions/{vid}/pregen-tutoring",
            post(routes::program::generate_pregen),
        )
        .route("/v1/config/flags", get(routes::program::list_flags))
        .route("/v1/admin/flags", post(routes::program::set_flag))
        .route(
            "/v1/institutions",
            post(routes::program::create_institution),
        )
        .route(
            "/v1/institutions/{institution_id}/members",
            post(routes::program::add_member),
        )
        .route(
            "/v1/institutions/{institution_id}/external-enrollments",
            post(routes::program::bind_external_enrollment),
        )
        .route(
            "/v1/institutions/{institution_id}/sso/oidc/start",
            get(routes::oidc::start_login),
        )
        .route(
            "/v1/admin/institutions/{institution_id}/sso/oidc",
            get(routes::oidc::get_provider).put(routes::oidc::configure_provider),
        )
        .route(
            "/v1/institutions/{institution_id}/cohorts",
            get(routes::program::list_cohorts).post(routes::program::create_cohort),
        )
        .route(
            "/v1/institutions/{institution_id}/programs",
            get(routes::program::list_programs).post(routes::program::create_program),
        )
        .route(
            "/v1/institutions/{institution_id}/programs/{program_id}/curriculum",
            put(routes::program::set_program_curriculum),
        )
        .route(
            "/v1/institutions/{institution_id}/programs/{program_id}/coverage",
            get(routes::program::program_curriculum_coverage),
        )
        .route(
            "/v1/institutions/{institution_id}/interop",
            post(routes::program::record_interop),
        )
        .route(
            "/v1/institutions/{institution_id}/audit",
            get(routes::program::institution_audit),
        )
        .route(
            "/v1/institutions/{institution_id}/analytics",
            get(routes::program::institution_analytics),
        )
        .route(
            "/v1/cohorts/{cohort_id}/assignments",
            post(routes::program::create_assignment),
        )
        .route(
            "/v1/me/portfolio",
            post(routes::program::add_portfolio_entry).get(routes::program::list_portfolio),
        )
        .route(
            "/v1/me/ce-activities",
            post(routes::program::add_ce_activity),
        )
        .route(
            "/v1/scenarios",
            get(routes::program::list_scenarios).post(routes::program::create_scenario),
        )
        .route(
            "/v1/admin/scenarios/{scenario_id}/versions",
            post(routes::program::create_scenario_version),
        )
        .route("/v1/scenarios/runs", post(routes::program::start_scenario))
        .route(
            "/v1/scenarios/runs/{run_id}",
            get(routes::program::get_scenario_run),
        )
        .route(
            "/v1/scenarios/runs/{run_id}/events",
            post(routes::program::scenario_event),
        )
        .route(
            "/v1/scenarios/runs/{run_id}/team",
            get(routes::scenario_team::list_team),
        )
        .route(
            "/v1/scenarios/runs/{run_id}/team/invites",
            post(routes::scenario_team::create_invite),
        )
        .route(
            "/v1/scenario-team-invites/join",
            post(routes::scenario_team::join_invite),
        )
        .route(
            "/v1/scenarios/runs/{run_id}/handovers",
            get(routes::scenario_team::list_handovers).post(routes::scenario_team::create_handover),
        )
        .route(
            "/v1/scenarios/runs/{run_id}/handovers/{handover_id}/ack",
            post(routes::scenario_team::acknowledge_handover),
        )
        .route(
            "/v1/scenarios/runs/{run_id}/counterfactual",
            post(routes::sim::counterfactual_replay),
        )
        .route(
            "/v1/scenarios/runs/{run_id}/appeals",
            post(routes::sim::appeal_scenario_assessment),
        )
        .route(
            "/v1/admin/scenarios/runs/{run_id}/assessment",
            get(routes::sim::admin_assessment).post(routes::sim::record_assessment),
        )
        .route(
            "/v1/admin/scenarios/runs/pending-assessment",
            get(routes::sim::pending_assessments),
        )
        .route(
            "/v1/admin/scenario-assessment-appeals",
            get(routes::sim::list_scenario_assessment_appeals),
        )
        .route(
            "/v1/admin/scenario-assessment-appeals/{appeal_id}",
            get(routes::sim::get_scenario_assessment_appeal),
        )
        .route(
            "/v1/admin/scenario-assessment-appeals/{appeal_id}/review",
            post(routes::sim::review_scenario_assessment_appeal),
        )
        .route(
            "/v1/note-collections",
            post(routes::retest::create_collection).get(routes::retest::list_collections),
        )
        .route(
            "/v1/note-collections/{collection_id}/notes",
            post(routes::retest::add_note_to_collection),
        )
        .route(
            "/v1/notes/{note_id}/concepts",
            post(routes::retest::tag_note_concept),
        )
        .route(
            "/v1/concepts/{concept}/notes",
            get(routes::retest::notes_by_concept),
        )
        .route("/v1/me/export", get(routes::packs::export_account))
        .route(
            "/v1/packs/{exam_id}/manifest",
            get(routes::packs::legacy_pack_manifest),
        )
        .route(
            "/v2/packs/{exam_id}/manifest",
            get(routes::packs::pack_manifest),
        )
        .route("/v1/packs/lease", post(routes::packs::create_lease))
        .route("/v1/me/packs", get(routes::packs::list_leases))
        .route(
            "/v1/packs/lease/{lease_id}",
            delete(routes::packs::revoke_lease),
        )
        .route("/v1/sync/events", post(routes::sync::sync_events))
        .route("/v1/me/decks/export", get(routes::review::export_decks))
        .route("/v1/me/decks/import", post(routes::review::import_decks))
        .route(
            "/v1/scenarios/runs/{run_id}/debrief",
            get(routes::sim::debrief),
        )
        .route("/v1/appeals", post(routes::sim::submit_appeal))
        .route("/v1/me/retests", get(routes::retest::due_retests))
        .route("/v1/me/retests/result", post(routes::retest::retest_result))
        .route("/v1/config", get(routes::config::config))
        .route("/v1/me/xp", get(routes::engagement::my_xp))
        .route("/v1/me/weekly-recap", get(routes::engagement::weekly_recap))
        .route(
            "/v1/me/mistake-hypotheses",
            get(routes::insights::mistake_hypotheses),
        )
        .route("/v1/me/heatmap", get(routes::insights::mastery_heatmap))
        .route("/v1/me/marks", get(routes::marks::my_marks))
        .route(
            "/v1/questions/{qid}/mark",
            post(routes::marks::mark_question).delete(routes::marks::unmark_question),
        )
        .route(
            "/v1/admin/psychometrics",
            get(routes::admin::psychometric_queue),
        )
        .route(
            "/v1/competitions",
            post(routes::engagement::create_competition).get(routes::engagement::list_competitions),
        )
        .route(
            "/v1/competitions/{comp_id}/entry",
            post(routes::engagement::submit_competition_entry),
        )
        .route(
            "/v1/institutions/{inst_id}/coverage",
            get(routes::engagement::institution_coverage),
        )
        .route(
            "/v1/integrity-events",
            post(routes::integrity::record_integrity_event),
        )
        .route(
            "/v1/practice/sessions/{sid}/action",
            post(routes::actions::session_action),
        )
        .route("/v1/decks", post(routes::review::create_deck))
        .route("/v1/decks/{deck_id}/cards", post(routes::review::add_card))
        .route("/v1/reviews/queue", get(routes::review::queue))
        .route("/v1/reviews/events", post(routes::review::review_event))
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route(
            "/api/v1/auth/oidc/callback/{institution_id}",
            get(routes::oidc::callback),
        )
        .route("/api/v1/auth/oidc/complete", post(routes::oidc::complete))
        .route(
            "/api/v1/me/devices",
            post(routes::accounts::register_device).get(routes::accounts::list_devices),
        )
        .route(
            "/api/v1/me/devices/{device_id}",
            delete(routes::accounts::revoke_device),
        )
        .route(
            "/api/v1/me/account",
            delete(routes::accounts::delete_account),
        )
        .route("/api/v1/exams", get(routes::exams::list_exams))
        .route(
            "/api/v1/admin/exams/{exam_id}/specs",
            post(routes::exams::create_exam_spec),
        )
        .route(
            "/api/v1/admin/exam-specs/{spec_id}/forms",
            post(routes::exams::create_assessment_form),
        )
        .route(
            "/api/v1/me/accommodations",
            post(routes::exams::set_accommodation),
        )
        .route("/api/v1/me/outcomes", post(routes::exams::add_outcome))
        .route("/api/v1/me/readiness", get(routes::exams::readiness))
        .route(
            "/api/v1/calculators/{kind}",
            post(routes::calculators::calculate),
        )
        .route(
            "/api/v1/calculators/convert",
            post(routes::calculators::convert),
        )
        .route("/api/v1/me/today", get(routes::today::today))
        .route(
            "/api/v1/me/plan/next-action",
            get(routes::today::next_action),
        )
        .route(
            "/api/v1/me/engagement",
            get(routes::engagement::engagement_status),
        )
        .route(
            "/api/v1/me/engagement/settings",
            axum::routing::put(routes::engagement::update_engagement_settings),
        )
        .route("/api/v1/qotd", get(routes::engagement::qotd))
        .route("/api/v1/me/qotd", get(routes::engagement::qotd))
        .route(
            "/api/v1/me/qotd/answers",
            post(routes::engagement::answer_qotd),
        )
        .route(
            "/api/v1/practice/sessions",
            post(routes::practice::create_session),
        )
        .route(
            "/api/v1/practice/sessions/{sid}",
            get(routes::practice::get_session),
        )
        .route(
            "/api/v1/practice/sessions/{sid}/answers",
            post(routes::practice::answer),
        )
        .route(
            "/api/v1/practice/sessions/{sid}/items/{item_index}/hint",
            get(routes::practice::hint),
        )
        .route(
            "/api/v1/practice/sessions/{sid}/submit",
            post(routes::practice::submit),
        )
        .route(
            "/api/v1/plans/{pid}/revisions/{rid}/undo",
            post(routes::today::undo_revision),
        )
        .route(
            "/api/v1/plans/{pid}/tasks/{task_id}/protection",
            put(routes::today::set_task_protection),
        )
        .route(
            "/api/v1/questions/versions/{vid}/reports",
            post(routes::reports::report).get(routes::reports::my_reports),
        )
        .route(
            "/api/v1/reports/{rid}/resolve",
            post(routes::reports::resolve),
        )
        .route(
            "/api/v1/mocks",
            post(routes::mock::create_mock).get(routes::mock::list_mocks),
        )
        .route("/api/v1/mocks/{mid}/start", post(routes::mock::start_mock))
        .route(
            "/api/v1/questions/versions/{vid}/community-stats",
            get(routes::practice::community_stats),
        )
        .route(
            "/api/v1/admin/hierarchy",
            post(routes::admin::create_node).get(routes::admin::list_nodes),
        )
        .route(
            "/api/v1/admin/hierarchy/{node_id}",
            axum::routing::patch(routes::admin::update_node),
        )
        .route(
            "/api/v1/admin/concepts",
            get(routes::concepts::list).post(routes::concepts::create),
        )
        .route(
            "/api/v1/admin/concepts/{concept_id}/versions",
            post(routes::concepts::create_version),
        )
        .route(
            "/api/v1/admin/hierarchy/{node_id}/concepts",
            get(routes::concepts::node_mappings).put(routes::concepts::set_node_mappings),
        )
        .route(
            "/api/v1/admin/questions",
            post(routes::admin::create_question).get(routes::admin::search_questions),
        )
        .route("/api/v1/admin/import", post(routes::admin::import))
        .route(
            "/api/v1/admin/import/{batch_id}/rollback",
            post(routes::admin::rollback_import),
        )
        .route("/api/v1/admin/audit", get(routes::admin::audit_log))
        .route("/api/v1/admin/reports", get(routes::reports::review_queue))
        .route(
            "/api/v1/admin/assessment-workflow",
            post(routes::admin::assessment_workflow),
        )
        .route(
            "/api/v1/assessments/{form_id}/sessions",
            post(routes::exams::start_assessment_session),
        )
        .route(
            "/api/v1/community/profile",
            post(routes::community::create_profile),
        )
        .route("/api/v1/community/me", get(routes::community::my_profile))
        .route(
            "/api/v1/community/groups",
            post(routes::community::create_group).get(routes::community::list_groups),
        )
        .route(
            "/api/v1/community/groups/{group_id}/join",
            post(routes::community::join_group),
        )
        .route(
            "/api/v1/community/groups/{group_id}/posts",
            post(routes::community::create_post).get(routes::community::list_posts),
        )
        .route(
            "/api/v1/community/groups/{group_id}/posts/{post_id}",
            axum::routing::delete(routes::community::remove_post),
        )
        .route(
            "/api/v1/community/duels",
            post(routes::community::create_duel),
        )
        .route(
            "/api/v1/community/duels/by-token/{token}",
            get(routes::community::duel_by_token),
        )
        .route(
            "/api/v1/community/duels/{duel_id}",
            get(routes::community::duel_state),
        )
        .route(
            "/api/v1/community/duels/{duel_id}/accept",
            post(routes::community::accept_duel),
        )
        .route(
            "/api/v1/community/duels/{duel_id}/decline",
            post(routes::community::decline_duel),
        )
        .route(
            "/api/v1/competitions/{comp_id}/leaderboard",
            get(routes::community::competition_leaderboard),
        )
        .route(
            "/api/v1/competitions/{comp_id}/claim",
            post(routes::community::claim_prize),
        )
        .route(
            "/api/v1/admin/competitions/{comp_id}/prize-review",
            post(routes::community::prize_review),
        )
        .route("/api/v1/me/plan/replan", post(routes::program::replan_plan))
        .route(
            "/api/v1/me/exam-switch/{to_exam_id}/gap-report",
            get(routes::program::exam_switch_gap_report),
        )
        .route(
            "/api/v1/me/selection-policy",
            get(routes::program::selection_policy),
        )
        .route("/api/v1/me/review-debt", get(routes::review::review_debt))
        .route(
            "/api/v1/me/session-policy",
            axum::routing::patch(routes::accounts::set_session_policy),
        )
        .route(
            "/api/v1/admin/articles/{article_id}/media",
            post(routes::library::attach_media),
        )
        .route(
            "/api/v1/admin/image-cases",
            post(routes::library::create_image_case),
        )
        .route(
            "/api/v1/me/image-cases",
            get(routes::library::list_image_cases),
        )
        .route(
            "/api/v1/analytics/events",
            post(routes::analytics::receive_events),
        )
        .route("/api/v1/admin/dashboard", get(routes::admin::dashboard))
        .route(
            "/api/v1/admin/content-rights",
            post(routes::admin::create_content_rights).get(routes::admin::list_content_rights),
        )
        .route(
            "/api/v1/admin/content-rights/{rights_id}/revoke",
            axum::routing::patch(routes::admin::revoke_content_rights),
        )
        .route(
            "/api/v1/admin/library/extraction-reports",
            get(routes::admin::list_extraction_reports)
                .post(routes::admin::create_extraction_report)
                .layer(DefaultBodyLimit::max(3 * 1024 * 1024)),
        )
        .route(
            "/api/v1/admin/library/extraction-reports/{report_id}",
            get(routes::admin::get_extraction_report),
        )
        .route(
            "/api/v1/admin/library/extraction-reports/{report_id}/review",
            post(routes::admin::review_extraction_report),
        )
        .route("/api/v1/admin/ai-admin", get(routes::admin::ai_admin))
        .route(
            "/api/v1/admin/incidents",
            post(routes::admin::create_incident).get(routes::admin::list_incidents),
        )
        .route(
            "/api/v1/admin/incidents/{incident_id}",
            axum::routing::patch(routes::admin::update_incident),
        )
        .route("/api/v1/client-update", get(routes::config::client_update))
        .route("/api/v1/me/curriculum", get(routes::today::my_curriculum))
        .route(
            "/api/v1/community/profiles/{handle}",
            get(routes::community::profile_by_handle),
        )
        .route("/api/v1/me/duels", get(routes::community::my_duels))
        .route("/api/v1/me/trends", get(routes::insights::accuracy_trends))
        .route(
            "/api/v1/admin/questions/{question_id}/versions",
            post(routes::admin::create_variant),
        )
        .route(
            "/api/v1/admin/recovery-drills/run",
            post(routes::admin::run_recovery_drill),
        )
        .route(
            "/api/v1/admin/recovery-drills",
            get(routes::admin::list_recovery_drills),
        )
        .route(
            "/api/v1/admin/coach-regression/run",
            post(routes::admin::run_coach_regression),
        )
        .route(
            "/api/v1/admin/coach-regression",
            get(routes::admin::list_coach_regression),
        )
        .route(
            "/api/v1/admin/qti/packages/{exam_id}",
            get(routes::exams::qti_export),
        )
        .route(
            "/api/v1/me/institutions",
            get(routes::program::my_institutions),
        )
        .route("/api/v1/coach/turns", post(routes::coach::coach_turn))
        .route("/api/v1/coach/history", get(routes::coach::history))
        .route("/api/v1/me/coach-memory", get(routes::coach::list_memory))
        .route(
            "/api/v1/me/coach-memory/{key}",
            axum::routing::put(routes::coach::put_memory).delete(routes::coach::delete_memory),
        )
        .route(
            "/api/v1/me/interventions",
            post(routes::coach::create_intervention),
        )
        .route(
            "/api/v1/me/interventions/{id}",
            axum::routing::patch(routes::coach::measure_intervention),
        )
        .route(
            "/api/v1/coach/answerable-questions",
            get(routes::coach::answerable_questions),
        )
        .route(
            "/api/v1/notes",
            post(routes::notes::create_note).get(routes::notes::list_notes),
        )
        .route(
            "/api/v1/notes/{note_id}",
            axum::routing::patch(routes::notes::update_note).delete(routes::notes::delete_note),
        )
        .route("/api/v1/notes/link", post(routes::notes::link_notes))
        .route("/api/v1/notes/export", get(routes::notes::export_notes))
        .route("/api/v1/library/search", get(routes::library::search))
        .route(
            "/api/v1/library/articles/{slug}",
            get(routes::library::get_article),
        )
        .route(
            "/api/v1/me/library/import-rights",
            get(routes::library::private_import_rights),
        )
        .route(
            "/api/v1/me/library/imports",
            get(routes::library::list_private_imports)
                .post(routes::library::create_private_import)
                .layer(DefaultBodyLimit::max(3 * 1024 * 1024)),
        )
        .route(
            "/api/v1/me/library/imports/{document_id}",
            get(routes::library::get_private_import).delete(routes::library::delete_private_import),
        )
        .route(
            "/api/v1/admin/source-passages",
            post(routes::source_changes::register_passage),
        )
        .route(
            "/api/v1/admin/source-passages/{passage_id}/dependencies",
            post(routes::source_changes::link_dependency),
        )
        .route(
            "/api/v1/admin/source-passages/{passage_id}/changes",
            post(routes::source_changes::record_change),
        )
        .route(
            "/api/v1/admin/source-changes/{event_id}",
            get(routes::source_changes::get_change),
        )
        .route(
            "/api/v1/admin/source-change-tasks/{task_id}",
            axum::routing::put(routes::source_changes::resolve_task),
        )
        .route(
            "/api/v1/me/notifications",
            get(routes::inbox::inbox).patch(routes::inbox::update_preferences),
        )
        .route(
            "/api/v1/me/notifications/{note_id}/read",
            post(routes::inbox::mark_read),
        )
        .route(
            "/api/v1/me/goals",
            post(routes::goals::create_goal).get(routes::goals::list_goals),
        )
        .route(
            "/api/v1/me/commitments",
            post(routes::goals::add_commitment),
        )
        .route(
            "/api/v1/me/commitments/{id}",
            axum::routing::delete(routes::goals::remove_commitment),
        )
        .route(
            "/api/v1/admin/psychometrics/{vid}",
            get(routes::admin::psychometric_screening),
        )
        .route(
            "/api/v1/questions/versions/{vid}/pregen-tutoring",
            post(routes::program::generate_pregen),
        )
        .route("/api/v1/config/flags", get(routes::program::list_flags))
        .route("/api/v1/admin/flags", post(routes::program::set_flag))
        .route(
            "/api/v1/institutions",
            post(routes::program::create_institution),
        )
        .route(
            "/api/v1/institutions/{institution_id}/members",
            post(routes::program::add_member),
        )
        .route(
            "/api/v1/institutions/{institution_id}/external-enrollments",
            post(routes::program::bind_external_enrollment),
        )
        .route(
            "/api/v1/institutions/{institution_id}/sso/oidc/start",
            get(routes::oidc::start_login),
        )
        .route(
            "/api/v1/admin/institutions/{institution_id}/sso/oidc",
            get(routes::oidc::get_provider).put(routes::oidc::configure_provider),
        )
        .route(
            "/api/v1/institutions/{institution_id}/cohorts",
            get(routes::program::list_cohorts).post(routes::program::create_cohort),
        )
        .route(
            "/api/v1/institutions/{institution_id}/programs",
            get(routes::program::list_programs).post(routes::program::create_program),
        )
        .route(
            "/api/v1/institutions/{institution_id}/programs/{program_id}/curriculum",
            put(routes::program::set_program_curriculum),
        )
        .route(
            "/api/v1/institutions/{institution_id}/programs/{program_id}/coverage",
            get(routes::program::program_curriculum_coverage),
        )
        .route(
            "/api/v1/institutions/{institution_id}/interop",
            post(routes::program::record_interop),
        )
        .route(
            "/api/v1/institutions/{institution_id}/audit",
            get(routes::program::institution_audit),
        )
        .route(
            "/api/v1/institutions/{institution_id}/analytics",
            get(routes::program::institution_analytics),
        )
        .route(
            "/api/v1/cohorts/{cohort_id}/assignments",
            post(routes::program::create_assignment),
        )
        .route(
            "/api/v1/me/portfolio",
            post(routes::program::add_portfolio_entry).get(routes::program::list_portfolio),
        )
        .route(
            "/api/v1/me/ce-activities",
            post(routes::program::add_ce_activity),
        )
        .route(
            "/api/v1/scenarios",
            get(routes::program::list_scenarios).post(routes::program::create_scenario),
        )
        .route(
            "/api/v1/admin/scenarios/{scenario_id}/versions",
            post(routes::program::create_scenario_version),
        )
        .route(
            "/api/v1/scenarios/runs",
            post(routes::program::start_scenario),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}",
            get(routes::program::get_scenario_run),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}/events",
            post(routes::program::scenario_event),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}/team",
            get(routes::scenario_team::list_team),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}/team/invites",
            post(routes::scenario_team::create_invite),
        )
        .route(
            "/api/v1/scenario-team-invites/join",
            post(routes::scenario_team::join_invite),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}/handovers",
            get(routes::scenario_team::list_handovers).post(routes::scenario_team::create_handover),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}/handovers/{handover_id}/ack",
            post(routes::scenario_team::acknowledge_handover),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}/counterfactual",
            post(routes::sim::counterfactual_replay),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}/appeals",
            post(routes::sim::appeal_scenario_assessment),
        )
        .route(
            "/api/v1/admin/scenarios/runs/{run_id}/assessment",
            get(routes::sim::admin_assessment).post(routes::sim::record_assessment),
        )
        .route(
            "/api/v1/admin/scenarios/runs/pending-assessment",
            get(routes::sim::pending_assessments),
        )
        .route(
            "/api/v1/admin/scenario-assessment-appeals",
            get(routes::sim::list_scenario_assessment_appeals),
        )
        .route(
            "/api/v1/admin/scenario-assessment-appeals/{appeal_id}",
            get(routes::sim::get_scenario_assessment_appeal),
        )
        .route(
            "/api/v1/admin/scenario-assessment-appeals/{appeal_id}/review",
            post(routes::sim::review_scenario_assessment_appeal),
        )
        .route(
            "/api/v1/note-collections",
            post(routes::retest::create_collection).get(routes::retest::list_collections),
        )
        .route(
            "/api/v1/note-collections/{collection_id}/notes",
            post(routes::retest::add_note_to_collection),
        )
        .route(
            "/api/v1/notes/{note_id}/concepts",
            post(routes::retest::tag_note_concept),
        )
        .route(
            "/api/v1/concepts/{concept}/notes",
            get(routes::retest::notes_by_concept),
        )
        .route("/api/v1/me/export", get(routes::packs::export_account))
        .route(
            "/api/v1/packs/{exam_id}/manifest",
            get(routes::packs::legacy_pack_manifest),
        )
        .route(
            "/api/v2/packs/{exam_id}/manifest",
            get(routes::packs::pack_manifest),
        )
        .route("/api/v1/packs/lease", post(routes::packs::create_lease))
        .route("/api/v1/me/packs", get(routes::packs::list_leases))
        .route(
            "/api/v1/packs/lease/{lease_id}",
            delete(routes::packs::revoke_lease),
        )
        .route("/api/v1/sync/events", post(routes::sync::sync_events))
        .route("/api/v1/me/decks/export", get(routes::review::export_decks))
        .route(
            "/api/v1/me/decks/import",
            post(routes::review::import_decks),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}/debrief",
            get(routes::sim::debrief),
        )
        .route("/api/v1/appeals", post(routes::sim::submit_appeal))
        .route("/api/v1/me/retests", get(routes::retest::due_retests))
        .route(
            "/api/v1/me/retests/result",
            post(routes::retest::retest_result),
        )
        .route("/api/v1/config", get(routes::config::config))
        .route("/api/v1/me/xp", get(routes::engagement::my_xp))
        .route(
            "/api/v1/me/weekly-recap",
            get(routes::engagement::weekly_recap),
        )
        .route(
            "/api/v1/me/mistake-hypotheses",
            get(routes::insights::mistake_hypotheses),
        )
        .route("/api/v1/me/heatmap", get(routes::insights::mastery_heatmap))
        .route("/api/v1/me/marks", get(routes::marks::my_marks))
        .route(
            "/api/v1/questions/{qid}/mark",
            post(routes::marks::mark_question).delete(routes::marks::unmark_question),
        )
        .route(
            "/api/v1/admin/psychometrics",
            get(routes::admin::psychometric_queue),
        )
        .route(
            "/api/v1/competitions",
            post(routes::engagement::create_competition).get(routes::engagement::list_competitions),
        )
        .route(
            "/api/v1/competitions/{comp_id}/entry",
            post(routes::engagement::submit_competition_entry),
        )
        .route(
            "/api/v1/institutions/{inst_id}/coverage",
            get(routes::engagement::institution_coverage),
        )
        .route(
            "/api/v1/me/settings-public",
            get(routes::settings::public_settings),
        )
        .route(
            "/api/v1/integrity-events",
            post(routes::integrity::record_integrity_event),
        )
        .route(
            "/api/v1/practice/sessions/{sid}/action",
            post(routes::actions::session_action),
        )
        .route("/api/v1/decks", post(routes::review::create_deck))
        .route(
            "/api/v1/decks/{deck_id}/cards",
            post(routes::review::add_card),
        )
        .route("/api/v1/reviews/queue", get(routes::review::queue))
        .route("/api/v1/reviews/events", post(routes::review::review_event))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn health_json() -> axum::Json<serde_json::Value> {
    // JSON twin for the plain-text healthz (lets the deploy probe parse it).
    axum::Json(serde_json::json!({"status": "ok"}))
}

async fn version() -> axum::Json<serde_json::Value> {
    // Served at /api/version.json in production so the deploy pipeline can
    // prove the exact commit landed. SHA stamped at image build time via
    // MEDICALOS_BUILD_SHA (Docker build-arg); option_env! keeps local/CI
    // builds compiling without it.
    axum::Json(serde_json::json!({
        "sha": option_env!("MEDICALOS_BUILD_SHA").unwrap_or("dev"),
    }))
}
