//! Batch: AI-18 pre-generated tutoring, OPS-06 feature flags, INST-01/02/04
//! institutions/cohorts/assignments, CAREER-01 portfolio, CAREER-03 CE
//! records, SIM-01/02/05 deterministic scenario engine, Phase 6 translation
//! scaffold. API-first; surfaces attach in the next client iterations.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use std::collections::HashSet;
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

#[derive(Deserialize, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "packs/TutoringCard.ts", rename = "TutoringCard")
)]
pub struct TutoringCard {
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"explain\" | \"why_wrong\" | \"compare\" | \"mnemonic\" | \"test_me\"")
    )]
    pub prompt_type: String,
    pub content: String,
    pub source_ref: String,
}

/// Ensure the five deterministic, source-linked cards exist for one published
/// question version. A separate version gets a separate cache by its ID.
pub(crate) async fn ensure_pregen_on(
    connection: &mut PgConnection,
    vid: Uuid,
) -> ApiResult<Vec<TutoringCard>> {
    let qv = sqlx::query!(
        r#"SELECT correct_index, key_learning_point, options, source_ref
           FROM question_versions WHERE id = $1 AND status = 'published'"#,
        vid
    )
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    let restricted: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
               SELECT 1 FROM reserved_questions rq
               JOIN assessment_forms f ON f.id = rq.form_id
               WHERE rq.question_version_id = $1 AND f.ai_allowed = FALSE
           )"#,
    )
    .bind(vid)
    .fetch_one(&mut *connection)
    .await?;
    if restricted {
        return Ok(Vec::new());
    }

    let options: Vec<QuestionOption> =
        serde_json::from_value(qv.options).map_err(|_| ApiError::internal())?;
    let key = qv.correct_index as usize;
    if options.len() < 2 || key >= options.len() {
        return Err(ApiError::internal());
    }
    let key_option = &options[key];
    let distractor = &options[(key + 1) % options.len()];
    let cards = [
        (
            "explain",
            format!(
                "Simple version: {} — because {}.",
                key_option.text, key_option.rationale
            ),
        ),
        (
            "why_wrong",
            "Compare your selected option with the keyed answer using the feedback rationales."
                .to_string(),
        ),
        (
            "compare",
            format!(
                "\"{}\" vs \"{}\": the difference that matters is {} vs {}.",
                key_option.text, distractor.text, key_option.rationale, distractor.rationale
            ),
        ),
        (
            "mnemonic",
            format!(
                "Mnemonic cue: {} (from the reviewed key point).",
                qv.key_learning_point
            ),
        ),
        (
            "test_me",
            format!(
                "Recall: State the key learning point for this question.\nAnswer: {}",
                qv.key_learning_point
            ),
        ),
    ];
    for (prompt_type, content) in cards {
        sqlx::query!(
            "INSERT INTO pregen_tutoring (id, question_version_id, prompt_type, content)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (question_version_id, prompt_type)
             DO UPDATE SET content = EXCLUDED.content
             WHERE pregen_tutoring.prompt_type IN ('why_wrong', 'test_me')",
            Uuid::new_v4(),
            vid,
            prompt_type,
            content
        )
        .execute(&mut *connection)
        .await?;
    }
    let rows = sqlx::query!(
        "SELECT prompt_type, content FROM pregen_tutoring
         WHERE question_version_id = $1 ORDER BY prompt_type",
        vid
    )
    .fetch_all(&mut *connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| TutoringCard {
            prompt_type: row.prompt_type,
            content: row.content,
            source_ref: qv.source_ref.clone(),
        })
        .collect())
}

pub(crate) async fn ensure_pregen(pool: &PgPool, vid: Uuid) -> ApiResult<Vec<TutoringCard>> {
    let mut tx = pool.begin().await?;
    let cards = ensure_pregen_on(&mut tx, vid).await?;
    tx.commit().await?;
    Ok(cards)
}

/// Populate the cache from reviewed material. This is an editorial operation;
/// learners may read cards only after an eligible tutor answer.
pub async fn generate_pregen(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: HeaderMap,
    Path(vid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(headers.get("x-admin-token").and_then(|v| v.to_str().ok()))?;
    let cards = ensure_pregen(&state.pool, vid).await?;
    if cards.is_empty() {
        return Err(ApiError::forbidden(
            "ai_restricted_for_assessment",
            "AI assistance is not allowed for this reserved assessment",
        ));
    }
    Ok(Json(json!({ "generated": cards.len() })))
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
    // Tenant-scoped authority (CORE-04): institution admins manage their own
    // membership; the global admin token stays valid for operator tooling.
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    let via_admin_token = state.require_admin(provided).is_ok();
    if !via_admin_token {
        let staff = sqlx::query!(
            "SELECT 1 AS one FROM institution_members
             WHERE institution_id = $1 AND user_id = $2 AND role = 'admin'",
            institution_id,
            user.user_id
        )
        .fetch_optional(&state.pool)
        .await?;
        if staff.is_none() {
            return Err(ApiError::forbidden(
                "admin_required",
                "institution membership is managed by the institution's admins",
            ));
        }
    }
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
pub struct ExternalEnrollmentReq {
    pub provider: String,
    pub subject: String,
    pub user_id: Uuid,
    pub role: Option<String>,
    pub cohort_id: Option<Uuid>,
}

/// Provision an institution-scoped external identity for a learner.
/// This endpoint records a mapping for a provider integration; it does not
/// accept or verify provider authentication assertions.
pub async fn bind_external_enrollment(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(institution_id): Path<Uuid>,
    Json(req): Json<ExternalEnrollmentReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provider = req.provider.trim();
    if provider.is_empty()
        || provider.len() > 2048
        || provider.chars().any(char::is_control)
        || req.subject.trim().is_empty()
        || req.subject.len() > 500
        || req.subject.chars().any(char::is_control)
    {
        return Err(ApiError::unprocessable(
            "invalid_external_identity",
            "provider must be 1-2048 characters and subject 1-500 characters without control characters",
        ));
    }
    if req.role.as_deref().is_some_and(|role| role != "learner") {
        return Err(ApiError::unprocessable(
            "invalid_enrollment_role",
            "external enrollment can provision learners only",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let staff = sqlx::query!(
        "SELECT 1 AS one FROM institution_members
         WHERE institution_id = $1 AND user_id = $2
           AND role IN ('admin', 'instructor')
         FOR UPDATE",
        institution_id,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if staff.is_none() {
        return Err(ApiError::forbidden(
            "instructor_required",
            "institution staff access required",
        ));
    }

    let active_user = sqlx::query!(
        "SELECT 1 AS one FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        req.user_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if active_user.is_none() {
        return Err(ApiError::not_found("user_not_found"));
    }
    if let Some(cohort_id) = req.cohort_id {
        let cohort = sqlx::query!(
            "SELECT 1 AS one FROM cohorts WHERE id = $1 AND institution_id = $2",
            cohort_id,
            institution_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        if cohort.is_none() {
            return Err(ApiError::not_found("cohort_not_found"));
        }
    }

    let identity_created = sqlx::query!(
        "INSERT INTO external_identities (institution_id, provider, subject, user_id)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (institution_id, provider, subject) DO NOTHING",
        institution_id,
        provider,
        req.subject,
        req.user_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected()
        > 0;

    if !identity_created {
        let existing = sqlx::query!(
            "SELECT user_id FROM external_identities
             WHERE institution_id = $1 AND provider = $2 AND subject = $3",
            institution_id,
            provider,
            req.subject
        )
        .fetch_one(&mut *tx)
        .await?;
        if existing.user_id != req.user_id {
            return Err(ApiError::conflict(
                "external_identity_conflict",
                "this provider identity is already bound to another user",
            ));
        }
    }

    let membership_created = sqlx::query!(
        "INSERT INTO institution_members (institution_id, user_id, role)
         VALUES ($1, $2, 'learner')
         ON CONFLICT (institution_id, user_id) DO NOTHING",
        institution_id,
        req.user_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected()
        > 0;

    let cohort_membership_created = if let Some(cohort_id) = req.cohort_id {
        sqlx::query!(
            "INSERT INTO cohort_members (cohort_id, user_id) VALUES ($1, $2)
             ON CONFLICT (cohort_id, user_id) DO NOTHING",
            cohort_id,
            req.user_id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0
    } else {
        false
    };

    if identity_created || membership_created || cohort_membership_created {
        crate::routes::admin::audit_scoped(
            &mut *tx,
            user.user_id,
            institution_id,
            "external_enrollment_bound",
            "external_identity",
            req.user_id,
            json!({
                "provider": provider,
                "role": "learner",
                "cohort_id": req.cohort_id
            }),
        )
        .await?;
    }

    let membership = sqlx::query!(
        "SELECT role FROM institution_members WHERE institution_id = $1 AND user_id = $2",
        institution_id,
        req.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(json!({
        "user_id": req.user_id,
        "provider": provider,
        "subject": req.subject,
        "role": membership.role,
        "cohort_id": req.cohort_id,
        "identity_created": identity_created
    })))
}

#[derive(Deserialize)]
pub struct CreateCohortReq {
    pub name: String,
    pub member_ids: Option<Vec<Uuid>>,
    pub program_id: Option<Uuid>,
}

pub async fn list_cohorts(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(institution_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_institution_staff(&state, institution_id, user.user_id).await?;
    let rows = sqlx::query!(
        r#"SELECT c.id AS cohort_id, c.name, c.program_id AS "program_id?",
                  COUNT(u.id)::BIGINT AS "members!"
           FROM cohorts c
           LEFT JOIN cohort_members cm ON cm.cohort_id = c.id
           LEFT JOIN users u ON u.id = cm.user_id AND u.deleted_at IS NULL
           WHERE c.institution_id = $1
           GROUP BY c.id
           ORDER BY c.name, c.id"#,
        institution_id
    )
    .fetch_all(&state.pool)
    .await?;
    let cohorts: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            json!({
                "cohort_id": row.cohort_id,
                "name": row.name,
                "program_id": row.program_id,
                "members": row.members
            })
        })
        .collect();
    Ok(Json(json!({ "cohorts": cohorts })))
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
    let mut tx = state.pool.begin().await?;
    let admin_member = sqlx::query!(
        "SELECT 1 AS one FROM institution_members
         WHERE institution_id = $1 AND user_id = $2 AND role IN ('admin','instructor')
         FOR UPDATE",
        institution_id,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::forbidden(
            "instructor_required",
            "only institution staff create cohorts",
        )
    })?;
    let _ = admin_member;

    let mut member_ids = req.member_ids.unwrap_or_default();
    if member_ids.len() > 500 {
        return Err(ApiError::unprocessable(
            "too_many_members",
            "a cohort can enroll at most 500 learners at a time",
        ));
    }
    member_ids.sort_unstable();
    member_ids.dedup();
    let active_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE id = ANY($1) AND deleted_at IS NULL FOR SHARE",
    )
    .bind(&member_ids)
    .fetch_all(&mut *tx)
    .await?;
    if active_ids.len() != member_ids.len() {
        return Err(ApiError::unprocessable(
            "invalid_members",
            "every cohort learner must be an active account",
        ));
    }
    let non_learner_members = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM institution_members
         WHERE institution_id = $1 AND user_id = ANY($2) AND role <> 'learner'
         FOR UPDATE",
    )
    .bind(institution_id)
    .bind(&member_ids)
    .fetch_all(&mut *tx)
    .await?;
    if !non_learner_members.is_empty() {
        return Err(ApiError::unprocessable(
            "invalid_cohort_role",
            "cohorts can contain learner members only",
        ));
    }

    if let Some(program_id) = req.program_id {
        let program = sqlx::query!(
            "SELECT 1 AS one FROM institution_programs
             WHERE id = $1 AND institution_id = $2",
            program_id,
            institution_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        if program.is_none() {
            return Err(ApiError::not_found("program_not_found"));
        }
    }

    let cohort_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO cohorts (id, institution_id, name, program_id)
         VALUES ($1, $2, $3, $4)",
        cohort_id,
        institution_id,
        name,
        req.program_id
    )
    .execute(&mut *tx)
    .await?;
    for member_id in &member_ids {
        let member = *member_id;
        sqlx::query!(
            "INSERT INTO institution_members (institution_id, user_id, role)
             VALUES ($1, $2, 'learner')
             ON CONFLICT (institution_id, user_id) DO NOTHING",
            institution_id,
            member
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO cohort_members (cohort_id, user_id) VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
            cohort_id,
            member
        )
        .execute(&mut *tx)
        .await?;
    }
    crate::routes::admin::audit_scoped(
        &mut *tx,
        user.user_id,
        institution_id,
        "cohort_created",
        "cohort",
        cohort_id,
        json!({ "name": name, "program_id": req.program_id, "member_ids": member_ids }),
    )
    .await?;
    tx.commit().await?;
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
    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "INSERT INTO institution_programs (id, institution_id, name) VALUES ($1, $2, $3)",
        id,
        institution_id,
        name
    )
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit_scoped(
        &mut *tx,
        user.user_id,
        institution_id,
        "program_created",
        "institution_program",
        id,
        json!({ "name": name }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "program_id": id })))
}

pub async fn list_programs(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(institution_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_institution_staff(&state, institution_id, user.user_id).await?;
    let rows = sqlx::query!(
        r#"SELECT p.id AS program_id, p.name,
                  COALESCE(
                      array_agg(m.chapter_id ORDER BY m.chapter_id)
                          FILTER (WHERE m.chapter_id IS NOT NULL),
                      ARRAY[]::UUID[]
                  ) AS "chapter_ids!"
           FROM institution_programs p
           LEFT JOIN institution_program_curriculum m ON m.program_id = p.id
           WHERE p.institution_id = $1
           GROUP BY p.id
           ORDER BY p.name, p.id"#,
        institution_id
    )
    .fetch_all(&state.pool)
    .await?;
    let programs: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            json!({
                "program_id": row.program_id,
                "name": row.name,
                "chapter_ids": row.chapter_ids
            })
        })
        .collect();
    Ok(Json(json!({ "programs": programs })))
}

#[derive(Deserialize)]
pub struct SetProgramCurriculumReq {
    pub chapter_ids: Vec<Uuid>,
}

pub async fn set_program_curriculum(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((institution_id, program_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SetProgramCurriculumReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    let staff = sqlx::query!(
        "SELECT 1 AS one FROM institution_members
         WHERE institution_id = $1 AND user_id = $2
           AND role IN ('admin', 'instructor')
         FOR UPDATE",
        institution_id,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if staff.is_none() {
        return Err(ApiError::forbidden(
            "instructor_required",
            "institution staff access required",
        ));
    }
    let program = sqlx::query!(
        "SELECT 1 AS one FROM institution_programs
         WHERE id = $1 AND institution_id = $2
         FOR UPDATE",
        program_id,
        institution_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if program.is_none() {
        return Err(ApiError::not_found("program_not_found"));
    }
    if req.chapter_ids.len() > 500 {
        return Err(ApiError::unprocessable(
            "too_many_chapters",
            "a program can map at most 500 chapters at a time",
        ));
    }

    let mut chapter_ids = req.chapter_ids;
    chapter_ids.sort_unstable();
    chapter_ids.dedup();
    let valid_chapters = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM curriculum_nodes
         WHERE id = ANY($1) AND kind = 'chapter'",
    )
    .bind(&chapter_ids)
    .fetch_one(&mut *tx)
    .await?;
    if valid_chapters != chapter_ids.len() as i64 {
        return Err(ApiError::unprocessable(
            "invalid_chapters",
            "every curriculum mapping must reference an existing chapter",
        ));
    }

    sqlx::query!(
        "DELETE FROM institution_program_curriculum WHERE program_id = $1",
        program_id
    )
    .execute(&mut *tx)
    .await?;
    for chapter_id in &chapter_ids {
        sqlx::query!(
            "INSERT INTO institution_program_curriculum
             (program_id, chapter_id, created_by) VALUES ($1, $2, $3)",
            program_id,
            chapter_id,
            user.user_id
        )
        .execute(&mut *tx)
        .await?;
    }
    crate::routes::admin::audit_scoped(
        &mut *tx,
        user.user_id,
        institution_id,
        "program_curriculum_replaced",
        "institution_program",
        program_id,
        json!({ "chapter_ids": chapter_ids }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(json!({
        "program_id": program_id,
        "chapter_ids": chapter_ids,
        "chapter_count": chapter_ids.len()
    })))
}

pub async fn program_curriculum_coverage(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((institution_id, program_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_institution_staff(&state, institution_id, user.user_id).await?;
    let program = sqlx::query!(
        "SELECT 1 AS one FROM institution_programs
         WHERE id = $1 AND institution_id = $2",
        program_id,
        institution_id
    )
    .fetch_optional(&state.pool)
    .await?;
    if program.is_none() {
        return Err(ApiError::not_found("program_not_found"));
    }

    let cohort_size = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT cm.user_id)::BIGINT
         FROM cohorts c
         JOIN cohort_members cm ON cm.cohort_id = c.id
         JOIN users u ON u.id = cm.user_id AND u.deleted_at IS NULL
         WHERE c.institution_id = $1 AND c.program_id = $2",
    )
    .bind(institution_id)
    .bind(program_id)
    .fetch_one(&state.pool)
    .await?;
    let rows = sqlx::query!(
        r#"WITH program_learners AS (
               SELECT DISTINCT cm.user_id
               FROM cohorts c
               JOIN cohort_members cm ON cm.cohort_id = c.id
               JOIN users u ON u.id = cm.user_id AND u.deleted_at IS NULL
               WHERE c.institution_id = $1 AND c.program_id = $2
           ), chapter_evidence AS (
               SELECT qv.chapter_id,
                      COUNT(DISTINCT a.user_id)::BIGINT AS learners_with_evidence,
                      COUNT(*)::BIGINT AS attempts
               FROM program_learners l
               JOIN attempts a ON a.user_id = l.user_id AND a.chosen_index IS NOT NULL
               JOIN question_versions qv ON qv.id = a.question_version_id
               GROUP BY qv.chapter_id
           )
           SELECT n.id AS chapter_id, n.name AS chapter_name,
                  COALESCE(e.learners_with_evidence, 0)::BIGINT AS "learners_with_evidence!",
                  COALESCE(e.attempts, 0)::BIGINT AS "attempts!"
           FROM institution_program_curriculum m
           JOIN curriculum_nodes n ON n.id = m.chapter_id
           LEFT JOIN chapter_evidence e ON e.chapter_id = n.id
           WHERE m.program_id = $2
           ORDER BY n.display_order, n.name"#,
        institution_id,
        program_id
    )
    .fetch_all(&state.pool)
    .await?;
    let suppressed = cohort_size < 5;
    let chapters: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            let learners_with_evidence = if suppressed {
                serde_json::Value::Null
            } else {
                json!(row.learners_with_evidence)
            };
            let attempts = if suppressed {
                serde_json::Value::Null
            } else {
                json!(row.attempts)
            };
            let coverage_percent = if suppressed || cohort_size == 0 {
                serde_json::Value::Null
            } else {
                json!(row.learners_with_evidence as f64 / cohort_size as f64 * 100.0)
            };
            json!({
                "chapter_id": row.chapter_id,
                "chapter": row.chapter_name,
                "learners_with_evidence": learners_with_evidence,
                "attempts": attempts,
                "coverage_percent": coverage_percent
            })
        })
        .collect();

    Ok(Json(json!({
        "program_id": program_id,
        "cohort_size": cohort_size,
        "minimum_group_size": 5,
        "suppressed": suppressed,
        "chapters": chapters
    })))
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
    #[serde(default)]
    pub rubric: Vec<ScenarioRubricCriterionReq>,
}

#[derive(Deserialize)]
pub struct ScenarioRubricCriterionReq {
    pub criterion_key: String,
    pub label: String,
    pub max_score: f32,
}

#[derive(Deserialize)]
pub struct CreateScenarioVersionReq {
    pub state_machine: serde_json::Value,
    #[serde(default)]
    pub rubric: Vec<ScenarioRubricCriterionReq>,
}

fn validate_scenario_rubric(
    rubric: &[ScenarioRubricCriterionReq],
    state_machine: &serde_json::Value,
) -> ApiResult<()> {
    if rubric.len() > 50 {
        return Err(ApiError::unprocessable(
            "invalid_rubric",
            "a station may define at most 50 rubric criteria",
        ));
    }
    let mut keys = HashSet::new();
    for criterion in rubric {
        let key = criterion.criterion_key.trim();
        let label = criterion.label.trim();
        if key.is_empty()
            || key.len() > 80
            || !key.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            })
            || label.is_empty()
            || label.chars().count() > 200
            || !criterion.max_score.is_finite()
            || criterion.max_score <= 0.0
            || criterion.max_score > 100.0
        {
            return Err(ApiError::unprocessable(
                "invalid_rubric_criterion",
                "criteria need a key, a 1-200 character label, and a maximum score in (0, 100]",
            ));
        }
        if !keys.insert(key.to_string()) {
            return Err(ApiError::unprocessable(
                "duplicate_rubric_criterion",
                "criterion keys must be unique within a scenario version",
            ));
        }
    }

    let terminal_states = state_machine
        .get("terminal_states")
        .and_then(serde_json::Value::as_array);
    if !rubric.is_empty() && terminal_states.is_none_or(Vec::is_empty) {
        return Err(ApiError::unprocessable(
            "terminal_states_required",
            "a scenario with a rubric must declare at least one terminal state",
        ));
    }
    if let Some(terminal_states) = terminal_states {
        let destinations: HashSet<&str> = state_machine
            .get("transitions")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|transition| transition.get("to").and_then(serde_json::Value::as_str))
            .collect();
        if terminal_states.is_empty()
            || terminal_states.iter().any(|state| {
                state
                    .as_str()
                    .is_none_or(|value| value.trim().is_empty() || !destinations.contains(value))
            })
        {
            return Err(ApiError::unprocessable(
                "invalid_terminal_states",
                "terminal states must be non-empty destinations of scenario transitions",
            ));
        }
    }
    Ok(())
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
    validate_scenario_rubric(&req.rubric, &req.state_machine)?;
    let id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let state_machine = req.state_machine;
    let rubric = req.rubric;
    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "INSERT INTO scenarios (id, slug, title, version, status, state_machine)
         VALUES ($1, $2, $3, 1, 'published', $4)",
        id,
        slug,
        req.title.trim(),
        state_machine.clone()
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO scenario_versions (id, scenario_id, version, status, state_machine)
         VALUES ($1, $2, 1, 'published', $3)",
        version_id,
        id,
        state_machine
    )
    .execute(&mut *tx)
    .await?;
    for criterion in rubric {
        sqlx::query(
            "INSERT INTO scenario_rubrics (id, scenario_id, scenario_version_id, criterion_key, label, max_score)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(version_id)
        .bind(criterion.criterion_key.trim())
        .bind(criterion.label.trim())
        .bind(criterion.max_score)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(
        json!({ "scenario_id": id, "scenario_version_id": version_id, "version": 1 }),
    ))
}

pub async fn create_scenario_version(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(scenario_id): Path<Uuid>,
    Json(req): Json<CreateScenarioVersionReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    admin(&state, &headers)?;
    validate_scenario_rubric(&req.rubric, &req.state_machine)?;

    let mut tx = state.pool.begin().await?;
    let current = sqlx::query_as::<_, (i32, String)>(
        "SELECT version, status FROM scenarios WHERE id = $1 FOR UPDATE",
    )
    .bind(scenario_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("scenario_not_found"))?;
    if current.1 != "published" {
        return Err(ApiError::conflict(
            "scenario_not_published",
            "only a published scenario can receive a new version",
        ));
    }
    let version = current.0.checked_add(1).ok_or_else(ApiError::internal)?;
    let version_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO scenario_versions (id, scenario_id, version, status, state_machine)
         VALUES ($1, $2, $3, 'published', $4)",
    )
    .bind(version_id)
    .bind(scenario_id)
    .bind(version)
    .bind(&req.state_machine)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE scenarios SET version = $2, state_machine = $3 WHERE id = $1")
        .bind(scenario_id)
        .bind(version)
        .bind(&req.state_machine)
        .execute(&mut *tx)
        .await?;
    for criterion in &req.rubric {
        sqlx::query(
            "INSERT INTO scenario_rubrics (id, scenario_id, scenario_version_id, criterion_key, label, max_score)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(scenario_id)
        .bind(version_id)
        .bind(criterion.criterion_key.trim())
        .bind(criterion.label.trim())
        .bind(criterion.max_score)
        .execute(&mut *tx)
        .await?;
    }
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "scenario_version_created",
        "scenario_version",
        version_id,
        json!({"scenario_id": scenario_id, "version": version, "criterion_count": req.rubric.len()}),
    )
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "scenario_id": scenario_id,
            "scenario_version_id": version_id,
            "version": version
        })),
    ))
}

/// Resolve one transition deterministically from the authored machine. The
/// machine shape is {initial, transitions: [{from, on, to}]}; anything not
/// covered is honestly refused instead of improvised (§14.3).
pub(crate) fn next_state(
    machine: &serde_json::Value,
    current: &str,
    event: &str,
) -> Option<String> {
    let transitions = machine.get("transitions")?.as_array()?;
    for t in transitions {
        if t.get("from")?.as_str()? == current && t.get("on")?.as_str()? == event {
            return t.get("to").and_then(|v| v.as_str()).map(String::from);
        }
    }
    None
}

pub(crate) fn scenario_events(machine: &serde_json::Value, state: Option<&str>) -> Vec<String> {
    let mut seen = HashSet::new();
    machine
        .get("transitions")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|transition| {
            state.is_none_or(|state| {
                transition.get("from").and_then(serde_json::Value::as_str) == Some(state)
            })
        })
        .filter_map(|transition| transition.get("on").and_then(serde_json::Value::as_str))
        .filter(|event| seen.insert((*event).to_string()))
        .map(str::to_string)
        .collect()
}

#[derive(Deserialize)]
pub struct TranscriptCorrectionReq {
    pub event_index: i32,
    pub corrected_text: String,
}

/// SIM-03: correct an uncertain text-mode transcript event before final
/// feedback. Append-only: the original transcript is never rewritten.
pub async fn submit_transcript_correction(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(run_id): Path<Uuid>,
    Json(req): Json<TranscriptCorrectionReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if req.event_index < 0 {
        return Err(ApiError::unprocessable(
            "invalid_event_index",
            "event_index must be 0 or greater",
        ));
    }
    let corrected_text = req.corrected_text.trim();
    if corrected_text.is_empty() || corrected_text.chars().count() > 500 {
        return Err(ApiError::unprocessable(
            "invalid_correction",
            "the corrected text must be 1-500 characters",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let run = sqlx::query!(
        "SELECT user_id, transcript FROM scenario_runs WHERE id = $1 FOR UPDATE",
        run_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    let via_admin_token = {
        let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
        state.require_admin(provided).is_ok()
    };
    if run.user_id != user.user_id && !via_admin_token {
        return Err(ApiError::forbidden(
            "not_run_owner",
            "only the run's learner or an operator may correct its transcript",
        ));
    }
    let events = run.transcript.as_array().ok_or_else(ApiError::internal)?;
    if req.event_index as usize >= events.len() {
        return Err(ApiError::unprocessable(
            "invalid_event_index",
            "event_index points outside this run's transcript",
        ));
    }
    let entry = &events[req.event_index as usize];
    if entry.get("uncertain").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(ApiError::unprocessable(
            "event_not_uncertain",
            "only uncertain transcript events can be corrected",
        ));
    }
    let already = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM scenario_transcript_corrections
         WHERE run_id = $1 AND event_index = $2)",
    )
    .bind(run_id)
    .bind(req.event_index)
    .fetch_one(&mut *tx)
    .await?;
    if already {
        return Err(ApiError::conflict(
            "event_already_corrected",
            "this transcript event already has a correction",
        ));
    }
    let original_event = entry
        .get("on")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO scenario_transcript_corrections
           (id, run_id, event_index, original_event, corrected_text, corrected_by)
         VALUES ($1, $2, $3, $4, $5, $6)",
        id,
        run_id,
        req.event_index,
        original_event,
        corrected_text,
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "event_index": req.event_index,
            "original_event": original_event,
            "corrected_text": corrected_text,
            "corrected_by": user.user_id,
        })),
    ))
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioTimelineEvent.ts",
        rename = "ScenarioTimelineEvent"
    )
)]
pub struct ScenarioTimelineEvent {
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    index: usize,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    sequence: usize,
    from: Option<String>,
    on: Option<String>,
    to: Option<String>,
    actor_role: Option<String>,
    uncertain: bool,
    uncertainty_reason: Option<String>,
}

pub(crate) fn transcript_timeline(transcript: &serde_json::Value) -> Vec<ScenarioTimelineEvent> {
    transcript
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, event)| ScenarioTimelineEvent {
            index,
            sequence: index + 1,
            from: event
                .get("from")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            on: event
                .get("on")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            to: event
                .get("to")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            actor_role: event
                .get("actor_role")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            // SIM-03: authored text-mode uncertainty travels with the
            // event so the learner and examiner see it before feedback.
            uncertain: event
                .get("uncertain")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            uncertainty_reason: event
                .get("uncertainty_reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        })
        .collect()
}

#[derive(sqlx::FromRow, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioSummary.ts",
        rename = "ScenarioSummary"
    )
)]
pub struct ScenarioSummary {
    slug: String,
    title: String,
    version: i32,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioListResponse.ts",
        rename = "ScenarioListResponse"
    )
)]
pub struct ScenarioListResponse {
    scenarios: Vec<ScenarioSummary>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioStartResponse.ts",
        rename = "ScenarioStartResponse"
    )
)]
pub struct ScenarioStartResponse {
    run_id: Uuid,
    scenario_slug: String,
    scenario: String,
    scenario_version: i32,
    current_state: String,
    available_actions: Vec<String>,
    #[cfg_attr(feature = "type-export", ts(type = "false"))]
    finished: bool,
}

pub async fn list_scenarios(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<ScenarioListResponse>> {
    let scenarios = sqlx::query_as::<_, ScenarioSummary>(
        r#"SELECT DISTINCT ON (scenario.id) scenario.slug, scenario.title, version.version
		   FROM scenarios scenario
		   JOIN scenario_versions version ON version.scenario_id = scenario.id
		   WHERE scenario.status = 'published' AND version.status = 'published'
		   ORDER BY scenario.id, version.version DESC
		   LIMIT 100"#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(ScenarioListResponse { scenarios }))
}

#[derive(Deserialize)]
pub struct StartScenarioReq {
    pub scenario_slug: String,
}

#[derive(sqlx::FromRow)]
struct ScenarioStartRow {
    id: Uuid,
    slug: String,
    title: String,
    scenario_version_id: Uuid,
    version: i32,
    state_machine: serde_json::Value,
}

pub async fn start_scenario(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<StartScenarioReq>,
) -> ApiResult<Json<ScenarioStartResponse>> {
    let scenario = sqlx::query_as::<_, ScenarioStartRow>(
        r#"SELECT s.id, s.slug, s.title, sv.id AS scenario_version_id,
                  sv.version, sv.state_machine
         FROM scenarios s JOIN scenario_versions sv ON sv.scenario_id = s.id
         WHERE s.slug = $1 AND s.status = 'published' AND sv.status = 'published'
         ORDER BY sv.version DESC LIMIT 1"#,
    )
    .bind(req.scenario_slug.trim())
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
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO scenario_runs (id, scenario_id, scenario_version_id, user_id, current_state)
		 VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(run_id)
    .bind(scenario.id)
    .bind(scenario.scenario_version_id)
    .bind(user.user_id)
    .bind(&initial)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO scenario_team_members (id, run_id, user_id, role, invited_by)
		 VALUES ($1, $2, $3, 'team_lead', $3)",
    )
    .bind(Uuid::new_v4())
    .bind(run_id)
    .bind(user.user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let available_actions = scenario_events(&scenario.state_machine, Some(&initial));
    Ok(Json(ScenarioStartResponse {
        run_id,
        scenario_slug: scenario.slug,
        scenario: scenario.title,
        scenario_version: scenario.version,
        current_state: initial,
        available_actions,
        finished: false,
    }))
}

#[derive(sqlx::FromRow)]
struct ScenarioRunView {
    title: String,
    current_state: String,
    transcript: serde_json::Value,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
    version: i32,
    state_machine: serde_json::Value,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioRun.ts",
        rename = "ScenarioRun"
    )
)]
pub struct ScenarioRun {
    run_id: Uuid,
    scenario: String,
    scenario_version: i32,
    current_state: String,
    #[cfg_attr(feature = "type-export", ts(type = "unknown[]"))]
    transcript: serde_json::Value,
    timeline: Vec<ScenarioTimelineEvent>,
    available_actions: Vec<String>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    started_at: DateTime<Utc>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    finished_at: Option<DateTime<Utc>>,
    finished: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioEventResponse.ts",
        rename = "ScenarioEventResponse"
    )
)]
pub struct ScenarioEventResponse {
    run_id: Uuid,
    current_state: String,
    finished: bool,
    available_actions: Vec<String>,
    timeline: Vec<ScenarioTimelineEvent>,
}

pub async fn get_scenario_run(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
) -> ApiResult<Json<ScenarioRun>> {
    let run = sqlx::query_as::<_, ScenarioRunView>(
        r#"SELECT scenario.title, run.current_state, run.transcript, run.started_at,
		          run.finished_at, version.version, version.state_machine
		   FROM scenario_runs run
		   JOIN scenarios scenario ON scenario.id = run.scenario_id
		   JOIN scenario_versions version ON version.id = run.scenario_version_id
		   WHERE run.id = $1
		     AND (run.user_id = $2 OR EXISTS (
		       SELECT 1 FROM scenario_team_members member
		       WHERE member.run_id = run.id AND member.user_id = $2
		     ))"#,
    )
    .bind(run_id)
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    let finished = run.finished_at.is_some();
    let available_actions = if finished {
        Vec::new()
    } else {
        scenario_events(&run.state_machine, Some(&run.current_state))
    };
    let timeline = transcript_timeline(&run.transcript);
    Ok(Json(ScenarioRun {
        run_id,
        scenario: run.title,
        scenario_version: run.version,
        current_state: run.current_state,
        transcript: run.transcript,
        timeline,
        available_actions,
        started_at: run.started_at,
        finished_at: run.finished_at,
        finished,
    }))
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
) -> ApiResult<Json<ScenarioEventResponse>> {
    #[derive(sqlx::FromRow)]
    struct ScenarioEventState {
        current_state: String,
        transcript: serde_json::Value,
        state_machine: serde_json::Value,
        actor_role: String,
    }
    let mut tx = state.pool.begin().await?;
    let run = sqlx::query_as::<_, ScenarioEventState>(
        "SELECT r.current_state, r.transcript, sv.state_machine,
                COALESCE(member.role, CASE WHEN r.user_id = $2 THEN 'team_lead' END) AS actor_role
           FROM scenario_runs r JOIN scenario_versions sv ON sv.id = r.scenario_version_id
         LEFT JOIN scenario_team_members member
           ON member.run_id = r.id AND member.user_id = $2
         WHERE r.id = $1 AND r.finished_at IS NULL
           AND (r.user_id = $2 OR member.id IS NOT NULL)
         FOR UPDATE OF r",
    )
    .bind(run_id)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    if run.actor_role == "observer" {
        return Err(ApiError::forbidden(
            "observer_read_only",
            "observers can view but cannot advance the station",
        ));
    }
    let event = req.event.trim();
    if event.is_empty() || event.chars().count() > 120 || event.chars().any(char::is_control) {
        return Err(ApiError::unprocessable(
            "invalid_scenario_event",
            "event name must be 1-120 printable characters",
        ));
    }
    let next = next_state(&run.state_machine, &run.current_state, event).ok_or_else(|| {
        ApiError::unprocessable(
            "no_transition",
            format!(
                "event {} is not valid from state {}",
                req.event, run.current_state
            ),
        )
    })?;
    let mut transcript = run.transcript.clone();
    let events = transcript.as_array_mut().ok_or_else(ApiError::internal)?;
    // SIM-03: text-mode transcript uncertainty is authored into the fixture
    // (states marked transcript_uncertain) and labelled as the drill it is —
    // never presented as real speech-recognition output.
    let state_spec = run
        .state_machine
        .get("states")
        .and_then(|states| states.get(&next));
    let uncertain = state_spec
        .and_then(|s| s.get("transcript_uncertain"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let mut entry = json!({
        "from": run.current_state.clone(),
        "on": event,
        "to": next.clone(),
        "actor_role": run.actor_role,
    });
    if uncertain {
        entry["uncertain"] = json!(true);
        entry["uncertainty_reason"] = json!(state_spec
            .and_then(|s| s.get("transcript_uncertainty_reason"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("authored text-mode uncertainty drill"));
    }
    events.push(entry);
    let finished = run
        .state_machine
        .get("terminal_states")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|states| states.iter().any(|state| state.as_str() == Some(&next)));
    sqlx::query(
        "UPDATE scenario_runs
         SET current_state = $2, transcript = $3,
             finished_at = CASE WHEN $4 THEN now() ELSE finished_at END
         WHERE id = $1",
    )
    .bind(run_id)
    .bind(&next)
    .bind(&transcript)
    .bind(finished)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let available_actions = if finished {
        Vec::new()
    } else {
        scenario_events(&run.state_machine, Some(&next))
    };
    Ok(Json(ScenarioEventResponse {
        run_id,
        current_state: next,
        finished,
        available_actions,
        timeline: transcript_timeline(&transcript),
    }))
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
    /// The version shown to the learner when they requested the replan.
    pub expected_version: i32,
}

/// The learner sets a new daily budget; today's plan is forked only if the
/// version they reviewed is still current. Protected and completed tasks stay
/// in the plan; only unprotected pending work may be deferred.
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
    crate::agent::get_or_create_today(&state.pool, user.user_id).await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 FOR UPDATE",
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let current = sqlx::query!(
        "SELECT id, version FROM plans
         WHERE user_id = $1 AND plan_date = CURRENT_DATE
         ORDER BY version DESC LIMIT 1 FOR UPDATE",
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if current.version != req.expected_version {
        return Err(ApiError::conflict_with_details(
            "stale_plan_version",
            "Your plan changed. Refresh it before replanning.",
            json!({ "current_version": current.version }),
        ));
    }
    let tasks = sqlx::query!(
        "SELECT id, title, estimated_minutes, status, protected FROM plan_tasks
         WHERE plan_id = $1 ORDER BY created_at, id",
        current.id
    )
    .fetch_all(&mut *tx)
    .await?;
    let committed: i64 = tasks
        .iter()
        .filter(|task| task.status == "pending")
        .map(|task| i64::from(task.estimated_minutes))
        .sum();
    if committed <= i64::from(req.daily_minutes) {
        tx.commit().await?;
        return Ok(Json(json!({
            "replanned": false,
            "reason": "within_capacity",
            "committed_minutes": committed,
            "daily_minutes": req.daily_minutes,
            "plan_id": current.id,
            "version": current.version,
        })));
    }
    if crate::agent::count_today_revisions_on(&mut tx, user.user_id).await?
        >= i64::from(crate::agent::MAX_PLAN_REVISIONS_PER_DAY)
    {
        return Err(ApiError::conflict_with_details(
            "plan_revision_limit",
            "Today's plan has reached its revision limit.",
            json!({ "revision_limit": crate::agent::MAX_PLAN_REVISIONS_PER_DAY }),
        ));
    }
    let protected_minutes: i64 = tasks
        .iter()
        .filter(|task| task.status == "pending" && task.protected)
        .map(|task| i64::from(task.estimated_minutes))
        .sum();
    if protected_minutes > i64::from(req.daily_minutes) {
        return Err(ApiError::conflict_with_details(
            "protected_tasks_over_capacity",
            "Protected tasks need more time than this daily budget. Increase the budget or unprotect a task.",
            json!({ "protected_minutes": protected_minutes, "daily_minutes": req.daily_minutes }),
        ));
    }

    let (new_plan_id, to_version) =
        crate::agent::fork_plan_on(&mut tx, current.id, current.version).await?;
    let mut kept_ids: Vec<Uuid> = Vec::new();
    let mut deferred_ids: Vec<Uuid> = Vec::new();
    let mut kept: Vec<String> = Vec::new();
    let mut deferred: Vec<String> = Vec::new();
    // Reserve protected minutes before considering optional tasks, so plan
    // order cannot cause the result to exceed the requested capacity.
    let mut budget = i64::from(req.daily_minutes) - protected_minutes;
    for t in &tasks {
        if t.status == "done" || t.protected {
            kept_ids.push(t.id);
            kept.push(t.title.clone());
        } else if t.status == "pending" && i64::from(t.estimated_minutes) <= budget {
            budget -= i64::from(t.estimated_minutes);
            kept_ids.push(t.id);
            kept.push(t.title.clone());
        } else if t.status == "pending" {
            deferred_ids.push(t.id);
            deferred.push(t.title.clone());
        }
    }
    // fork_plan_on regenerates task ids; match the deferred work through the
    // stable task_key it preserves across versions.
    sqlx::query!(
        r#"DELETE FROM plan_tasks
           WHERE plan_id = $1
             AND task_key IN (SELECT task_key FROM plan_tasks WHERE id = ANY($2))"#,
        new_plan_id,
        &deferred_ids
    )
    .execute(&mut *tx)
    .await?;
    let revision_id = Uuid::new_v4();
    let receipt = json!({
        "daily_minutes": req.daily_minutes,
        "committed_minutes_before": committed,
        "kept": kept,
        "deferred": deferred,
        "kept_task_ids": kept_ids,
        "deferred_task_ids": deferred_ids,
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
        current.version,
        to_version,
        explanation,
        receipt
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({
        "replanned": true,
        "plan_id": new_plan_id,
        "version": to_version,
        "kept_tasks": kept.len(),
        "deferred_tasks": deferred.len(),
        "deferred": deferred,
        "deferred_task_ids": deferred_ids,
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
    let covered = chapters.iter().filter(|c| c["covered"] == true).count();
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

// ---- INST-02: the learner's own institution memberships ---------------------

/// Memberships for the workspace picker: institution + role, nothing else.
pub async fn my_institutions(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT i.id, i.name, im.role
           FROM institution_members im
           JOIN institutions i ON i.id = im.institution_id
           WHERE im.user_id = $1 ORDER BY i.name"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "memberships": rows.iter().map(|r| json!({
        "institution_id": r.id,
        "name": r.name,
        "role": r.role,
    })).collect::<Vec<_>>() })))
}
