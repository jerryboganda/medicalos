//! EX-01/02/03/05/06 and TRUST-05/QB-10 governance endpoints.

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

fn admin(state: &AppState, headers: &axum::http::HeaderMap) -> ApiResult<()> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)
}

pub async fn list_exams(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT id, code, name, official_source_url, aliases
           FROM exams ORDER BY code"#
    )
    .fetch_all(&state.pool)
    .await?;
    let exams: Vec<_> = rows
        .into_iter()
        .map(|r| {
            json!({
                "exam_id": r.id,
                "code": r.code,
                "name": r.name,
                "official_source_url": r.official_source_url,
                "aliases": r.aliases,
            })
        })
        .collect();
    Ok(Json(json!({ "exams": exams })))
}

#[derive(Deserialize)]
pub struct ExamSpecReq {
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub config: serde_json::Value,
}

pub async fn create_exam_spec(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(exam_id): Path<Uuid>,
    Json(req): Json<ExamSpecReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&state, &headers)?;
    if !req.config.is_object() {
        return Err(ApiError::unprocessable(
            "invalid_exam_config",
            "config must be an object",
        ));
    }
    if req.effective_to.is_some_and(|d| d < req.effective_from) {
        return Err(ApiError::unprocessable(
            "invalid_effective_dates",
            "effective_to must not precede effective_from",
        ));
    }
    let exists = sqlx::query!("SELECT 1 AS one FROM exams WHERE id = $1", exam_id)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_none() {
        return Err(ApiError::not_found("exam_not_found"));
    }
    let version = sqlx::query!(
        r#"SELECT COALESCE(MAX(version), 0) + 1 AS "version!"
           FROM exam_spec_versions WHERE exam_id = $1"#,
        exam_id
    )
    .fetch_one(&state.pool)
    .await?
    .version;
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO exam_spec_versions
           (id, exam_id, version, effective_from, effective_to, config)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
        id,
        exam_id,
        version,
        req.effective_from,
        req.effective_to,
        req.config
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "spec_id": id, "version": version })))
}

#[derive(Deserialize)]
pub struct AssessmentFormReq {
    pub name: String,
    pub assessment_family: String,
    pub blueprint: serde_json::Value,
    #[serde(default)]
    pub reserved: bool,
    #[serde(default)]
    pub ai_allowed: bool,
    /// EX-05: published versions bound to a reserved form are served only
    /// through that form — required when reserved is true.
    pub question_ids: Option<Vec<Uuid>>,
}

pub async fn create_assessment_form(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(spec_id): Path<Uuid>,
    Json(req): Json<AssessmentFormReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&state, &headers)?;
    if req.name.trim().is_empty() || req.assessment_family.trim().is_empty() {
        return Err(ApiError::unprocessable(
            "invalid_assessment_form",
            "name and assessment_family are required",
        ));
    }
    if !req.blueprint.is_object() {
        return Err(ApiError::unprocessable(
            "invalid_blueprint",
            "blueprint must be an object",
        ));
    }
    let exists = sqlx::query!(
        "SELECT 1 AS one FROM exam_spec_versions WHERE id = $1",
        spec_id
    )
    .fetch_optional(&state.pool)
    .await?;
    if exists.is_none() {
        return Err(ApiError::not_found("exam_spec_not_found"));
    }
    // EX-05: a reserved family must bind its fixed question set, and every
    // bound version must be published — reserved content is reviewed content.
    let question_ids = req.question_ids.unwrap_or_default();
    if req.reserved {
        if question_ids.is_empty() {
            return Err(ApiError::unprocessable(
                "invalid_reserved_form",
                "reserved forms must bind their question_ids",
            ));
        }
        for vid in &question_ids {
            let published = sqlx::query!(
                "SELECT 1 AS one FROM question_versions WHERE id = $1 AND status = 'published'",
                vid
            )
            .fetch_optional(&state.pool)
            .await?;
            if published.is_none() {
                return Err(ApiError::unprocessable(
                    "invalid_reserved_question",
                    "reserved forms may only bind published question versions",
                ));
            }
        }
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO assessment_forms
           (id, exam_spec_id, name, assessment_family, blueprint, reserved, ai_allowed)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        id,
        spec_id,
        req.name.trim(),
        req.assessment_family.trim(),
        req.blueprint,
        req.reserved,
        req.ai_allowed
    )
    .execute(&state.pool)
    .await?;
    if req.reserved {
        for vid in &question_ids {
            sqlx::query!(
                "INSERT INTO reserved_questions (question_version_id, form_id)
                 VALUES ($1, $2)
                 ON CONFLICT (question_version_id) DO NOTHING",
                vid,
                id
            )
            .execute(&state.pool)
            .await?;
        }
    }
    Ok(Json(json!({
        "form_id": id,
        "reserved": req.reserved,
        "ai_allowed": req.ai_allowed,
        "reserved_questions": if req.reserved { question_ids.len() } else { 0 },
    })))
}

/// EX-05: start the fixed session a reserved form exists for. The form's
/// bound questions are snapshotted in stable order; the standard answer and
/// submit pipeline takes over from there.
pub async fn start_assessment_session(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(form_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let form = sqlx::query!(
        "SELECT id, ai_allowed FROM assessment_forms WHERE id = $1",
        form_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("assessment_form_not_found"))?;
    let items = sqlx::query!(
        r#"SELECT rq.question_version_id
           FROM reserved_questions rq
           JOIN question_versions qv ON qv.id = rq.question_version_id
           WHERE rq.form_id = $1 AND qv.status = 'published'
           ORDER BY rq.question_version_id"#,
        form_id
    )
    .fetch_all(&state.pool)
    .await?;
    if items.is_empty() {
        return Err(ApiError::unprocessable(
            "form_empty",
            "this assessment form has no published reserved questions",
        ));
    }
    let sid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO practice_sessions (id, user_id, preset, form_id, ai_allowed)
         VALUES ($1, $2, 'exam', $3, $4)",
        sid,
        user.user_id,
        form.id,
        form.ai_allowed
    )
    .execute(&state.pool)
    .await?;
    for (i, q) in items.iter().enumerate() {
        let idx = i as i16;
        sqlx::query!(
            "INSERT INTO session_items (id, session_id, item_index, question_version_id)
             VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            sid,
            idx,
            q.question_version_id
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(json!({
        "session_id": sid,
        "question_count": items.len(),
        "ai_allowed": form.ai_allowed,
    })))
}

#[derive(Deserialize)]
pub struct AccommodationReq {
    pub key: String,
    pub value: serde_json::Value,
}

pub async fn set_accommodation(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<AccommodationReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let key = req.key.trim();
    if key.is_empty() || key.len() > 100 {
        return Err(ApiError::unprocessable(
            "invalid_accommodation",
            "key must be 1-100 characters",
        ));
    }
    sqlx::query!(
        r#"INSERT INTO learner_accommodations (user_id, accommodation_key, value)
           VALUES ($1, $2, $3)
           ON CONFLICT (user_id, accommodation_key) DO UPDATE
           SET value = EXCLUDED.value, updated_at = now()"#,
        user.user_id,
        key,
        req.value
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "saved": key })))
}

#[derive(Deserialize)]
pub struct OutcomeReq {
    pub exam_id: Uuid,
    pub sat_on: NaiveDate,
    pub outcome: serde_json::Value,
    #[serde(default)]
    pub consented: bool,
}

pub async fn add_outcome(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<OutcomeReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !req.consented {
        return Err(ApiError::unprocessable(
            "consent_required",
            "outcome collection requires learner consent",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO exam_outcomes
           (id, user_id, exam_id, sat_on, outcome, consented)
           VALUES ($1, $2, $3, $4, $5, true)"#,
        id,
        user.user_id,
        req.exam_id,
        req.sat_on,
        req.outcome
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "outcome_id": id, "verified": false })))
}

#[derive(Deserialize)]
pub struct ReadinessQuery {
    pub exam_id: Uuid,
}

pub async fn readiness(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Query(q): Query<ReadinessQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let exam_exists = sqlx::query!("SELECT 1 AS one FROM exams WHERE id = $1", q.exam_id)
        .fetch_optional(&state.pool)
        .await?;
    if exam_exists.is_none() {
        return Err(ApiError::not_found("exam_not_found"));
    }
    let validation = sqlx::query!(
        r#"SELECT id, intended_use, population, method, result, approved_by, approved_at
           FROM validation_records
           WHERE intended_use = 'readiness'
           ORDER BY approved_at DESC LIMIT 1"#
    )
    .fetch_optional(&state.pool)
    .await?;
    let Some(v) = validation else {
        return Ok(Json(json!({
            "available": false,
            "reason": "validation_required",
            "message": "Numerical readiness is withheld until the intended use is externally validated."
        })));
    };
    Ok(Json(json!({
        "available": false,
        "reason": "calibrated_model_not_configured",
        "validation": {
            "validation_id": v.id,
            "population": v.population,
            "method": v.method,
            "approved_by": v.approved_by,
            "approved_at": v.approved_at,
        }
    })))
}

// ---- INST-06: QTI 2.1 package export -------------------------------------------

/// Real QTI 2.1 XML for an exam's published questions: a package envelope
/// plus one <assessmentItem> per version. Exchange-ready output; certification
/// of a consuming LMS is a separate activity (§18.5) and is never claimed here.
pub async fn qti_export(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(exam_id): Path<Uuid>,
) -> Result<axum::response::Response, ApiError> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    let rows = sqlx::query!(
        r#"SELECT qv.id, qv.lead_in, qv.options, qv.correct_index
           FROM question_versions qv
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE c.exam_id = $1 AND qv.status = 'published'
           ORDER BY qv.id"#,
        exam_id
    )
    .fetch_all(&state.pool)
    .await?;
    let esc = |t: &str| {
        t.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let mut items = String::new();
    for r in &rows {
        let opts: Vec<serde_json::Value> =
            serde_json::from_value(r.options.clone()).unwrap_or_default();
        let mut choices = String::new();
        for (i, o) in opts.iter().enumerate() {
            let letter = format!("C{}", i + 1);
            let text = o["text"].as_str().unwrap_or("");
            choices.push_str(&format!(
                "      <simpleChoice identifier=\"{letter}\">{}</simpleChoice>\n",
                esc(text)
            ));
        }
        let correct = format!("C{}", r.correct_index + 1);
        items.push_str(&format!(
            "  <qti-assessment-item identifier=\"{id}\" title=\"Item {id}\">\n    \
             <responseDeclaration identifier=\"RESPONSE\" cardinality=\"single\" baseType=\"identifier\">\n      \
             <correctResponse><value>{correct}</value></correctResponse>\n    \
             </responseDeclaration>\n    \
             <itemBody>\n      <p>{lead}</p>\n{choices}    </itemBody>\n  </qti-assessment-item>\n",
            id = r.id,
            lead = esc(&r.lead_in),
            correct = correct,
            choices = choices,
        ));
    }
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<qti-package>\n<imsmanifest identifier=\"MANIFEST-{exam}\"/>\n{items}</qti-package>\n",
        exam = exam_id,
        items = items
    );
    let disposition = format!("attachment; filename=qti-{exam_id}.xml");
    let mut response_headers = axum::http::HeaderMap::new();
    response_headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/xml; charset=utf-8"),
    );
    response_headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&disposition).map_err(|_| ApiError::internal())?,
    );
    Ok((response_headers, xml).into_response())
}
