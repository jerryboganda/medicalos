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
    Ok(Json(json!({
        "form_id": id,
        "reserved": req.reserved,
        "ai_allowed": req.ai_allowed,
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
