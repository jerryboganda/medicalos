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

use axum::routing::{delete, get, post};
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
        .route("/v1/me/today", get(routes::today::today))
        .route(
            "/v1/me/engagement",
            get(routes::engagement::engagement_status),
        )
        .route(
            "/v1/me/engagement/settings",
            axum::routing::put(routes::engagement::update_engagement_settings),
        )
        .route("/v1/me/qotd", get(routes::engagement::engagement_status))
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
            "/v1/practice/sessions/{sid}/submit",
            post(routes::practice::submit),
        )
        .route(
            "/v1/plans/{pid}/revisions/{rid}/undo",
            post(routes::today::undo_revision),
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
            "/v1/admin/questions",
            post(routes::admin::create_question).get(routes::admin::search_questions),
        )
        .route("/v1/admin/import", post(routes::admin::import))
        .route(
            "/v1/admin/import/{batch_id}/rollback",
            post(routes::admin::rollback_import),
        )
        .route("/v1/admin/audit", get(routes::admin::audit_log))
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
            get(routes::program::pregen_for_question).post(routes::program::generate_pregen),
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
            "/v1/institutions/{institution_id}/cohorts",
            post(routes::program::create_cohort),
        )
        .route(
            "/v1/institutions/{institution_id}/programs",
            post(routes::program::create_program),
        )
        .route(
            "/v1/institutions/{institution_id}/interop",
            post(routes::program::record_interop),
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
        .route("/v1/scenarios", post(routes::program::create_scenario))
        .route("/v1/scenarios/runs", post(routes::program::start_scenario))
        .route(
            "/v1/scenarios/runs/{run_id}/events",
            post(routes::program::scenario_event),
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
        .route("/api/v1/me/today", get(routes::today::today))
        .route(
            "/api/v1/me/engagement",
            get(routes::engagement::engagement_status),
        )
        .route(
            "/api/v1/me/engagement/settings",
            axum::routing::put(routes::engagement::update_engagement_settings),
        )
        .route(
            "/api/v1/me/qotd",
            get(routes::engagement::engagement_status),
        )
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
            "/api/v1/practice/sessions/{sid}/submit",
            post(routes::practice::submit),
        )
        .route(
            "/api/v1/plans/{pid}/revisions/{rid}/undo",
            post(routes::today::undo_revision),
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
            "/api/v1/admin/questions",
            post(routes::admin::create_question).get(routes::admin::search_questions),
        )
        .route("/api/v1/admin/import", post(routes::admin::import))
        .route(
            "/api/v1/admin/import/{batch_id}/rollback",
            post(routes::admin::rollback_import),
        )
        .route("/api/v1/admin/audit", get(routes::admin::audit_log))
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
            get(routes::program::pregen_for_question).post(routes::program::generate_pregen),
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
            "/api/v1/institutions/{institution_id}/cohorts",
            post(routes::program::create_cohort),
        )
        .route(
            "/api/v1/institutions/{institution_id}/programs",
            post(routes::program::create_program),
        )
        .route(
            "/api/v1/institutions/{institution_id}/interop",
            post(routes::program::record_interop),
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
        .route("/api/v1/scenarios", post(routes::program::create_scenario))
        .route(
            "/api/v1/scenarios/runs",
            post(routes::program::start_scenario),
        )
        .route(
            "/api/v1/scenarios/runs/{run_id}/events",
            post(routes::program::scenario_event),
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
