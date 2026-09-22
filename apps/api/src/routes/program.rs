//! Batch: AI-18 pre-generated tutoring, OPS-06 feature flags, INST-01/02/04
//! institutions/cohorts/assignments, CAREER-01 portfolio, CAREER-03 CE
//! records, SIM-01/02/05 deterministic scenario engine, Phase 6 translation
//! scaffold. API-first; surfaces attach in the next client iterations.

use axum::extract::{Path, State};
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
    let key_option = options.get(key).ok_or_else(ApiError::internal)?;
    let distractor = options
        .get((key + 1) % options.len())
        .ok_or_else(ApiError::internal)?;

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
            content.as_str().unwrap_or_default()
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
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(institution_id): Path<Uuid>,
    Json(req): Json<JoinReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&state, &headers)?;
    let role = req.role.unwrap_or_else(|| "learner".into());
    if !matches!(
        role.as_str(),
        "admin" | "instructor" | "author" | "reviewer" | "learner"
    ) {
        return Err(ApiError::unprocessable(
            "invalid_role",
            "role must be admin, instructor, author, reviewer, or learner (§18.1)",
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
    crate::routes::admin::audit_scoped(
        &state.pool,
        user.user_id,
        institution_id,
        "membership_set",
        "institution_member",
        req.user_id,
        json!({ "role": role }),
    )
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
    crate::routes::admin::audit_scoped(
        &state.pool,
        user.user_id,
        institution_id,
        "cohort_created",
        "cohort",
        cohort_id,
        json!({ "name": name }),
    )
    .await?;
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

#[derive(Deserialize)]
pub struct CreateProgramReq {
    pub name: String,
}

async fn require_institution_staff(
    state: &AppState,
    institution_id: Uuid,
    user_id: Uuid,
) -> ApiResult<()> {
    let row = sqlx::query!(
        r#"SELECT 1 AS one FROM institution_members
           WHERE institution_id = $1 AND user_id = $2
             AND role IN ('admin', 'instructor')"#,
        institution_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    if row.is_none() {
        return Err(ApiError::forbidden(
            "instructor_required",
            "institution staff access required",
        ));
    }
    Ok(())
}

pub async fn create_program(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(institution_id): Path<Uuid>,
    Json(req): Json<CreateProgramReq>,
) -> ApiResult<Json<serde_json::Value>> {
    require_institution_staff(&state, institution_id, user.user_id).await?;
    let name = req.name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_name",
            "program name must be 1-200 characters",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO institution_programs (id, institution_id, name) VALUES ($1, $2, $3)",
        id,
        institution_id,
        name
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "program_id": id })))
}

#[derive(Deserialize)]
pub struct InteropReq {
    pub standard: String,
    pub direction: String,
    pub external_id: Option<String>,
    pub payload: serde_json::Value,
}

pub async fn record_interop(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(institution_id): Path<Uuid>,
    Json(req): Json<InteropReq>,
) -> ApiResult<Json<serde_json::Value>> {
    require_institution_staff(&state, institution_id, user.user_id).await?;
    if !matches!(req.standard.as_str(), "lti" | "qti") {
        return Err(ApiError::unprocessable(
            "invalid_standard",
            "standard must be lti or qti",
        ));
    }
    if !matches!(req.direction.as_str(), "import" | "export") {
        return Err(ApiError::unprocessable(
            "invalid_direction",
            "direction must be import or export",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO interoperability_receipts
           (id, institution_id, standard, direction, external_id, payload, status, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, 'recorded', $7)"#,
        id,
        institution_id,
        req.standard,
        req.direction,
        req.external_id,
        req.payload,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "receipt_id": id, "status": "recorded" })))
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
        req.hours as f32
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
        .ok_or_else(ApiError::internal)?
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
    if let Some(a) = transcript.as_array_mut() {
        a.push(json!({"from": run.current_state, "on": req.event, "to": next}));
    }
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

// ---- CORE-04: institution-scoped audit export (§18.3) -----------------------

/// Staff-only tenant audit trail: every institution-scoped mutation recorded
/// by this institution's staff actions. Aggregates never; this IS the log.
pub async fn institution_audit(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(institution_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_institution_staff(&state, institution_id, user.user_id).await?;
    let rows = sqlx::query!(
        r#"SELECT action, entity, entity_id, new_value, created_at
           FROM audit_events WHERE institution_id = $1
           ORDER BY created_at DESC LIMIT 200"#,
        institution_id
    )
    .fetch_all(&state.pool)
    .await?;
    let events: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "action": r.action,
                "entity": r.entity,
                "entity_id": r.entity_id,
                "new_value": r.new_value,
                "at": r.created_at,
            })
        })
        .collect();
    Ok(Json(json!({ "events": events })))
}

// ---- INST-07: privacy-preserving cohort analytics (§18.3) -------------------
//
// Aggregate-only per-chapter accuracy for one cohort. §18.3: cohort
// aggregates require minimum group size — below MIN_COHORT_SIZE the report
// is suppressed, never partially disclosed.

/// ponytail: fixed k=5 per §18.3 "minimum group-size"; per-deployment
/// configuration only if an institution ever asks.
const MIN_COHORT_SIZE: i64 = 5;

pub async fn institution_analytics(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(institution_id): Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<AnalyticsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    require_institution_staff(&state, institution_id, user.user_id).await?;
    let cohort_id = q.cohort_id.ok_or_else(|| {
        ApiError::unprocessable("cohort_required", "cohort_id query parameter is required")
    })?;
    // The cohort must belong to THIS institution — no cross-tenant reads.
    let cohort = sqlx::query!(
        "SELECT id FROM cohorts WHERE id = $1 AND institution_id = $2",
        cohort_id,
        institution_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("cohort_not_found"))?;
    let _ = cohort;
    let size: i64 = sqlx::query!(
        r#"SELECT COUNT(*) AS "n!" FROM cohort_members WHERE cohort_id = $1"#,
        cohort_id
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    if size < MIN_COHORT_SIZE {
        return Ok(Json(json!({
            "cohort_id": cohort_id,
            "cohort_size": size,
            "suppressed": true,
            "reason": "minimum_group_size",
            "minimum": MIN_COHORT_SIZE,
        })));
    }
    let rows = sqlx::query!(
        r#"SELECT c.name AS chapter,
                  COUNT(*) AS "attempts!",
                  COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS "correct!",
                  COUNT(DISTINCT a.user_id) AS "learners!"
           FROM attempts a
           JOIN cohort_members cm ON cm.user_id = a.user_id AND cm.cohort_id = $1
           JOIN question_versions qv ON qv.id = a.question_version_id
           LEFT JOIN curriculum_nodes c ON c.id = qv.chapter_id
           GROUP BY c.name ORDER BY c.name"#,
        cohort_id
    )
    .fetch_all(&state.pool)
    .await?;
    let chapters: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let accuracy = if r.attempts > 0 {
                r.correct as f64 / r.attempts as f64
            } else {
                0.0
            };
            json!({
                "chapter": r.chapter,
                "attempts": r.attempts,
                "correct": r.correct,
                "accuracy": accuracy,
                "learners": r.learners,
            })
        })
        .collect();
    Ok(Json(json!({
        "cohort_id": cohort_id,
        "cohort_size": size,
        "suppressed": false,
        "chapters": chapters,
    })))
}

#[derive(Deserialize)]
pub struct AnalyticsQuery {
    pub cohort_id: Option<Uuid>,
}

// ---- PLAN-02: capacity-driven replanning (§8.6) -----------------------------

#[derive(Deserialize)]
pub struct ReplanReq {
    /// The learner's real daily budget in minutes (5-480).
    pub daily_minutes: i32,
}

/// The learner (or a capacity change) sets a new daily budget; today's plan
/// is forked into a new version whose pending tasks fit the budget. Each
/// task costs question_count minutes (disclosed, deterministic). Done work
/// is never touched; trimmed tasks are reported, never hidden.
pub async fn replan_plan(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<ReplanReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !(5..=480).contains(&req.daily_minutes) {
        return Err(ApiError::unprocessable(
            "invalid_capacity",
            "daily_minutes must be 5-480",
        ));
    }
    let (old_plan_id, from_version) =
        crate::agent::get_or_create_today(&state.pool, user.user_id).await?;
    let tasks = sqlx::query!(
        "SELECT id, title, question_count, status FROM plan_tasks
         WHERE plan_id = $1 ORDER BY created_at",
        old_plan_id
    )
    .fetch_all(&state.pool)
    .await?;
    let committed: i32 = tasks.iter().map(|t| t.question_count).sum();
    if committed <= req.daily_minutes {
        return Ok(Json(json!({
            "replanned": false,
            "reason": "within_capacity",
            "committed_minutes": committed,
            "daily_minutes": req.daily_minutes,
        })));
    }
    let (new_plan_id, to_version) =
        crate::agent::fork_plan(&state.pool, old_plan_id, from_version).await?;
    let mut kept: Vec<String> = Vec::new();
    let mut deferred: Vec<String> = Vec::new();
    let mut budget = req.daily_minutes;
    for t in &tasks {
        if t.status == "done" || t.question_count <= budget {
            budget -= t.question_count;
            kept.push(t.title.clone());
        } else {
            deferred.push(t.title.clone());
        }
    }
    // Drop the deferred tasks from the new version (the old version rows
    // keep them for the audit trail).
    for t in &tasks {
        if deferred.contains(&t.title) {
            sqlx::query!(
                "DELETE FROM plan_tasks WHERE plan_id = $1 AND id = $2",
                new_plan_id,
                t.id
            )
            .execute(&state.pool)
            .await?;
        }
    }
    let revision_id = Uuid::new_v4();
    let receipt = json!({
        "daily_minutes": req.daily_minutes,
        "committed_minutes_before": committed,
        "kept": kept,
        "deferred": deferred,
    });
    let explanation = format!(
        "Plan trimmed to your {}-minute budget: {} task(s) deferred.",
        req.daily_minutes,
        deferred.len()
    );
    sqlx::query!(
        "INSERT INTO plan_revisions
           (id, plan_id, from_version, to_version, reason_code, explanation,
            automatic, receipt)
         VALUES ($1, $2, $3, $4, 'capacity_change', $5, false, $6)",
        revision_id,
        new_plan_id,
        from_version,
        to_version,
        explanation,
        receipt
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({
        "replanned": true,
        "plan_id": new_plan_id,
        "version": to_version,
        "kept_tasks": kept.len(),
        "deferred_tasks": deferred.len(),
        "deferred": deferred,
    })))
}

// ---- PLAN-04: exam-switch knowledge-gap report (§8.4) -----------------------

/// Where the learner stands against a different exam's curriculum: per-chapter
/// independent-attempt coverage, honest about what "covered" means.
pub async fn exam_switch_gap_report(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(to_exam_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let exam_exists = sqlx::query!("SELECT 1 AS one FROM exams WHERE id = $1", to_exam_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("exam_not_found"))?;
    let _ = exam_exists;
    let rows = sqlx::query!(
        r#"SELECT c.id AS chapter_id, c.name AS chapter_name,
                  COALESCE(COUNT(DISTINCT a.id), 0) AS "attempts!"
           FROM curriculum_nodes c
           LEFT JOIN question_versions qv ON qv.chapter_id = c.id
           LEFT JOIN attempts a ON a.question_version_id = qv.id
                AND a.user_id = $1 AND a.assisted = FALSE
                AND a.correct IS NOT NULL
           WHERE c.exam_id = $2 AND c.kind = 'chapter'
           GROUP BY c.id, c.name ORDER BY c.name"#,
        user.user_id,
        to_exam_id
    )
    .fetch_all(&state.pool)
    .await?;
    // §8.8 honesty: fewer than 10 independent attempts is low evidence —
    // the report says so instead of inventing readiness.
    const COVERED_ATTEMPTS: i64 = 10;
    let chapters: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "chapter_id": r.chapter_id,
                "chapter_name": r.chapter_name,
                "independent_attempts": r.attempts,
                "covered": r.attempts >= COVERED_ATTEMPTS,
                "evidence": if r.attempts >= COVERED_ATTEMPTS { "covered" } else if r.attempts > 0 { "low_evidence" } else { "no_evidence" },
            })
        })
        .collect();
    let covered = chapters
        .iter()
        .filter(|c| c["covered"] == serde_json::Value::from(true))
        .count();
    Ok(Json(json!({
        "to_exam_id": to_exam_id,
        "coverage_rule": "at least 10 independent (non-assisted) answered attempts",
        "total_chapters": chapters.len(),
        "covered_chapters": covered,
        "chapters": chapters,
    })))
}

// ---- AI-17: transparent selection policy disclosure (§8.5) ------------------

/// The estimator is deterministic and disclosed: per-chapter ability from
/// real attempts with a shrinking K, difficulty anchors, and the selection
/// rules that turn those numbers into tasks. Nothing here is invented.
pub async fn selection_policy(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT c.name AS chapter_name, lcs.ability AS "ability?",
                  lcs.evidence_count AS "evidence!", lcs.independent_count AS "independent!"
           FROM learner_concept_state lcs
           JOIN curriculum_nodes c ON c.id = lcs.chapter_id
           WHERE lcs.user_id = $1 ORDER BY lcs.ability ASC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let k_of = |independent: i32| (32.0 - 2.0 * independent as f32).max(8.0);
    let chapters: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "chapter": r.chapter_name,
                "ability": r.ability,
                "current_k": k_of(r.independent),
                "evidence_count": r.evidence,
            })
        })
        .collect();
    Ok(Json(json!({
        "estimator": {
            "model": "elo_baseline",
            "base": 1500.0,
            "difficulty_anchors": {"easy": 1350.0, "medium": 1500.0, "hard": 1650.0},
            "k_rule": "max(8, 32 - 2 x independent_count) — shrinks as evidence grows",
            "counted_evidence": "independent, answered attempts only (skips and assisted answers excluded)",
        },
        "selection_rules": [
            "cold start: one modest practice task on the first chapter",
            "revision-on-missed: a capped re-practice task after missed questions",
            "re-test queue: deterministic intervals 1/3/7/14 days, family variant preferred",
            "review queue: due cards first (most at risk), then new cards under daily caps",
        ],
        "your_chapters": chapters,
    })))
}
