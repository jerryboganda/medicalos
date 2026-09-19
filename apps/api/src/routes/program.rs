//! Batch: AI-18 pre-generated tutoring, OPS-06 feature flags, INST-01/02/04
//! institutions/cohorts/assignments, CAREER-01 portfolio, CAREER-03 CE
//! records, SIM-01/02/05 deterministic scenario engine, Phase 6 translation
//! scaffold. API-first; surfaces attach in the next client iterations.

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::seed::QuestionOption;
use crate::state::AppState;

fn admin(state: &AppState, headers: &axum::http::HeaderMap) -> ApiResult<()> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)
}

// ---- AI-18: pre-generated tutoring ------------------------------------------

/// Generate the five one-tap tutoring cards for a published question from
/// its own reviewed material (extractive — §9.5, no model, no hallucination
/// surface). Cached by (version, prompt type); a new version regenerates.
pub async fn generate_pregen(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(vid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let qv = sqlx::query!(
        r#"SELECT correct_index, key_learning_point, exam_tip, options, source_ref
           FROM question_versions WHERE id = $1 AND status = 'published'"#,
        vid
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    let options: Vec<QuestionOption> =
        serde_json::from_value(qv.options).map_err(|_| ApiError::internal())?;
    let key = qv.correct_index as usize;
    let key_option = options.get(key).ok_or_else(|| ApiError::internal())?;
    let distractor = options
        .get((key + 1) % options.len())
        .ok_or_else(|| ApiError::internal())?;

    let cards = json!({
        "explain": format!("Simple version: {} — because {}.", key_option.text, key_option.rationale),
        "why_wrong": format!(
            "The tempting wrong answer is \"{}\": {}. The reviewed position is \"{}\": {}.",
            distractor.text, distractor.rationale, key_option.text, key_option.rationale
        ),
        "compare": format!(
            "\"{}\" vs \"{}\": the difference that matters is {} vs {}.",
            key_option.text, distractor.text, key_option.rationale, distractor.rationale
        ),
        "mnemonic": format!("Mnemonic cue: {} (from the reviewed key point).", qv.key_learning_point),
        "test_me": "A fresh unseen variant will be scheduled by the re-test queue (SR-08).".to_string(),
    });

    for (ptype, content) in cards.as_object().unwrap() {
        sqlx::query!(
            "INSERT INTO pregen_tutoring (id, question_version_id, prompt_type, content)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (question_version_id, prompt_type)
             DO UPDATE SET content = $4, generated_by = 'extractive', created_at = now()",
            Uuid::new_v4(),
            vid,
            ptype,
            content
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(
        json!({ "generated": cards.as_object().unwrap().len() }),
    ))
}

/// Cached one-tap cards for a question. Offline-includable in packs (§9.5).
pub async fn pregen_for_question(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(vid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        "SELECT prompt_type, content FROM pregen_tutoring
         WHERE question_version_id = $1 ORDER BY prompt_type",
        vid
    )
    .fetch_all(&state.pool)
    .await?;
    let cards: serde_json::Value = rows
        .iter()
        .map(|r| json!({"prompt_type": r.prompt_type, "content": r.content}))
        .collect();
    Ok(Json(json!({ "cards": cards })))
}

// ---- OPS-06: feature flags / remote config ----------------------------------

pub async fn list_flags(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!("SELECT key, value, rollout_percent FROM feature_flags ORDER BY key")
        .fetch_all(&state.pool)
        .await?;
    let flags: serde_json::Value = rows
        .iter()
        .map(|r| json!({"key": r.key, "value": r.value, "rollout_percent": r.rollout_percent}))
        .collect();
    Ok(Json(json!({ "flags": flags })))
}

#[derive(Deserialize)]
pub struct FlagReq {
    pub key: String,
    pub value: serde_json::Value,
    pub rollout_percent: Option<i32>,
}

pub async fn set_flag(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<FlagReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&state, &headers)?;
    if req.key.is_empty() || req.key.len() > 100 {
        return Err(ApiError::unprocessable(
            "invalid_key",
            "flag key 1-100 chars",
        ));
    }
    sqlx::query!(
        "INSERT INTO feature_flags (key, value, rollout_percent)
         VALUES ($1, $2, COALESCE($3, 100))
         ON CONFLICT (key) DO UPDATE SET
           value = $2, rollout_percent = COALESCE($3, feature_flags.rollout_percent),
           updated_at = now()",
        req.key,
        req.value,
        req.rollout_percent
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "set": req.key })))
}

// ---- INST-01/02/04: institutions, cohorts, assignments ----------------------

#[derive(Deserialize)]
pub struct CreateInstitutionReq {
    pub name: String,
}

pub async fn create_institution(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CreateInstitutionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_name",
            "institution name must be 1-200 characters",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO institutions (id, name) VALUES ($1, $2)",
        id,
        name
    )
    .execute(&state.pool)
    .await?;
    // Creator becomes the institution admin (§18.1 role seed).
    sqlx::query!(
        "INSERT INTO institution_members (institution_id, user_id, role)
         VALUES ($1, $2, 'admin')",
        id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "institution_id": id })))
}

#[derive(Deserialize)]
pub struct JoinReq {
    pub user_id: Uuid,
    pub role: Option<String>,
}

pub async fn add_member(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(institution_id): Path<Uuid>,
    Json(req): Json<JoinReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&state, &headers)?;
    let role = req.role.unwrap_or_else(|| "learner".into());
    if !matches!(role.as_str(), "admin" | "instructor" | "learner") {
        return Err(ApiError::unprocessable(
            "invalid_role",
            "role must be admin, instructor, or learner",
        ));
    }
    let exists = sqlx::query!("SELECT 1 AS one FROM users WHERE id = $1", req.user_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("user_not_found"))?;
    let _ = exists;
    sqlx::query!(
        "INSERT INTO institution_members (institution_id, user_id, role)
         VALUES ($1, $2, $3)
         ON CONFLICT (institution_id, user_id) DO UPDATE SET role = $3",
        institution_id,
        req.user_id,
        role
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "member": req.user_id, "role": role })))
}

#[derive(Deserialize)]
pub struct CreateCohortReq {
    pub name: String,
    pub member_ids: Option<Vec<Uuid>>,
}

pub async fn create_cohort(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(institution_id): Path<Uuid>,
    Json(req): Json<CreateCohortReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_name",
            "cohort name must be 1-200 characters",
        ));
    }
    let admin_member = sqlx::query!(
        "SELECT 1 AS one FROM institution_members
         WHERE institution_id = $1 AND user_id = $2 AND role IN ('admin','instructor')",
        institution_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ApiError::forbidden(
            "instructor_required",
            "only institution staff create cohorts",
        )
    })?;
    let _ = admin_member;

    let cohort_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO cohorts (id, institution_id, name) VALUES ($1, $2, $3)",
        cohort_id,
        institution_id,
        name
    )
    .execute(&state.pool)
    .await?;
    for member in req.member_ids.unwrap_or_default() {
        sqlx::query!(
            "INSERT INTO cohort_members (cohort_id, user_id) VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
            cohort_id,
            member
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(json!({ "cohort_id": cohort_id })))
}

#[derive(Deserialize)]
pub struct AssignmentReq {
    pub title: String,
    pub due_at: Option<DateTime<Utc>>,
}

pub async fn create_assignment(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(cohort_id): Path<Uuid>,
    Json(req): Json<AssignmentReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let title = req.title.trim();
    if title.is_empty() || title.len() > 300 {
        return Err(ApiError::unprocessable(
            "invalid_title",
            "title must be 1-300 characters",
        ));
    }
    let staff = sqlx::query!(
        r#"SELECT 1 AS one FROM institution_members im
           JOIN cohorts c ON c.institution_id = im.institution_id
           WHERE c.id = $1 AND im.user_id = $2 AND im.role IN ('admin','instructor')"#,
        cohort_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ApiError::forbidden(
            "instructor_required",
            "only cohort staff create assignments",
        )
    })?;
    let _ = staff;
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO assignments (id, cohort_id, title, due_at, created_by)
         VALUES ($1, $2, $3, $4, $5)",
        id,
        cohort_id,
        title,
        req.due_at,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "assignment_id": id })))
}

// ---- CAREER-01 portfolio + CAREER-03 CE --------------------------------------

#[derive(Deserialize)]
pub struct PortfolioReq {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub detail: String,
    pub occurred_on: Option<chrono::NaiveDate>,
}

pub async fn add_portfolio_entry(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<PortfolioReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !matches!(
        req.kind.as_str(),
        "rotation" | "case_reflection" | "procedure_observation" | "certificate"
    ) {
        return Err(ApiError::unprocessable(
            "invalid_kind",
            "kind must be rotation, case_reflection, procedure_observation, or certificate",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO portfolio_entries (id, user_id, kind, title, detail, occurred_on)
         VALUES ($1, $2, $3, $4, $5, $6)",
        id,
        user.user_id,
        req.kind,
        req.title.trim(),
        req.detail,
        req.occurred_on
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "entry_id": id })))
}

pub async fn list_portfolio(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT id, kind, title, detail, occurred_on, created_at
           FROM portfolio_entries WHERE user_id = $1 ORDER BY created_at DESC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let entries: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "entry_id": r.id, "kind": r.kind, "title": r.title,
                "detail": r.detail, "occurred_on": r.occurred_on,
                "created_at": r.created_at,
            })
        })
        .collect();
    Ok(Json(json!({ "entries": entries })))
}

#[derive(Deserialize)]
pub struct CeActivityReq {
    pub activity: String,
    pub hours: f64,
}

pub async fn add_ce_activity(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CeActivityReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !(0.0..=100.0).contains(&req.hours) {
        return Err(ApiError::unprocessable(
            "invalid_hours",
            "hours must be 0-100",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO ce_activities (id, user_id, activity, hours) VALUES ($1, $2, $3, $4)",
        id,
        user.user_id,
        req.activity.trim(),
        req.hours
    )
    .execute(&state.pool)
    .await?;
    // §16: records only — never labelled accredited until accreditation exists.
    Ok(Json(json!({
        "activity_id": id,
        "note": "Recorded as activity. Not an accredited credit."
    })))
}

// ---- SIM-01/02/05: deterministic scenario engine -----------------------------

#[derive(Deserialize)]
pub struct CreateScenarioReq {
    pub slug: String,
    pub title: String,
    pub state_machine: serde_json::Value,
}

pub async fn create_scenario(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateScenarioReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&state, &headers)?;
    let slug = req.slug.trim();
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(ApiError::unprocessable(
            "invalid_slug",
            "slug must be alphanumeric/dashes",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO scenarios (id, slug, title, state_machine) VALUES ($1, $2, $3, $4)",
        id,
        slug,
        req.title.trim(),
        req.state_machine
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "scenario_id": id })))
}

/// Resolve one transition deterministically from the authored machine. The
/// machine shape is {initial, transitions: [{from, on, to}]}; anything not
/// covered is honestly refused instead of improvised (§14.3).
fn next_state(machine: &serde_json::Value, current: &str, event: &str) -> Option<String> {
    let transitions = machine.get("transitions")?.as_array()?;
    for t in transitions {
        if t.get("from")?.as_str()? == current && t.get("on")?.as_str()? == event {
            return t.get("to").and_then(|v| v.as_str()).map(String::from);
        }
    }
    None
}

#[derive(Deserialize)]
pub struct StartScenarioReq {
    pub scenario_slug: String,
}

pub async fn start_scenario(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<StartScenarioReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let scenario = sqlx::query!(
        "SELECT id, state_machine FROM scenarios
         WHERE slug = $1 AND status = 'published'",
        req.scenario_slug
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("scenario_not_found"))?;
    let initial = scenario
        .state_machine
        .get("initial")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::internal())?
        .to_string();
    let run_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO scenario_runs (id, scenario_id, user_id, current_state)
         VALUES ($1, $2, $3, $4)",
        run_id,
        scenario.id,
        user.user_id,
        initial
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "run_id": run_id, "current_state": initial })))
}

#[derive(Deserialize)]
pub struct ScenarioEventReq {
    pub event: String,
}

pub async fn scenario_event(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
    Json(req): Json<ScenarioEventReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let run = sqlx::query!(
        "SELECT r.current_state, r.transcript, s.state_machine
           FROM scenario_runs r JOIN scenarios s ON s.id = r.scenario_id
         WHERE r.id = $1 AND r.user_id = $2 AND r.finished_at IS NULL",
        run_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    let next = next_state(&run.state_machine, &run.current_state, &req.event).ok_or_else(|| {
        ApiError::unprocessable(
            "no_transition",
            format!(
                "event {} is not valid from state {}",
                req.event, run.current_state
            ),
        )
    })?;
    let mut transcript = run.transcript.clone();
    transcript.as_array_mut().map_or((), |a| {
        a.push(json!({"from": run.current_state, "on": req.event, "to": next}));
    });
    sqlx::query!(
        "UPDATE scenario_runs SET current_state = $2, transcript = $3 WHERE id = $1",
        run_id,
        next,
        transcript
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "run_id": run_id, "current_state": next })))
}
