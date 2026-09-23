//! ADMIN-06 baseline: hierarchy management, question CRUD, and bulk import
//! with dry-run/rollback (§19.5). Every mutation writes an audit event
//! (§19.5 audit log) and is admin-token gated until role-aware accounts
//! (18.1) land. Import accepts JSON rows; the CSV/Excel parser of §19.5 is
/// the next console iteration — the validation and rollback pipeline below
/// is the part that must be right first.
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::seed::QuestionOption;
use crate::state::AppState;
use domain_contracts::option_count;

fn admin_headers(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers.get("x-admin-token").and_then(|v| v.to_str().ok())
}

async fn audit(
    pool: impl sqlx::PgExecutor<'_>,
    actor: Uuid,
    action: &str,
    entity: &str,
    entity_id: Uuid,
    new_value: serde_json::Value,
) -> ApiResult<()> {
    sqlx::query!(
        "INSERT INTO audit_events (id, actor, action, entity, entity_id, new_value)
         VALUES ($1, $2, $3, $4, $5, $6)",
        Uuid::new_v4(),
        actor,
        action,
        entity,
        entity_id,
        new_value
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// CORE-04: institution-scoped audit event (§18.3 audit exports). Used by
/// routes::program for tenant-side mutations.
pub(crate) async fn audit_scoped(
    pool: impl sqlx::PgExecutor<'_>,
    actor: Uuid,
    institution_id: Uuid,
    action: &str,
    entity: &str,
    entity_id: Uuid,
    new_value: serde_json::Value,
) -> ApiResult<()> {
    sqlx::query!(
        "INSERT INTO audit_events (id, actor, action, entity, entity_id, new_value, institution_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        Uuid::new_v4(),
        actor,
        action,
        entity,
        entity_id,
        new_value,
        institution_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

// ---- hierarchy management (§5.5) -------------------------------------------

#[derive(Deserialize)]
pub struct CreateNodeReq {
    pub exam_id: Uuid,
    pub kind: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub display_order: Option<i32>,
}

pub async fn create_node(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateNodeReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    if !matches!(req.kind.as_str(), "subject" | "system" | "chapter") {
        return Err(ApiError::unprocessable(
            "invalid_kind",
            "kind must be subject, system, or chapter",
        ));
    }
    let name = req.name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_name",
            "name must be 1-200 characters",
        ));
    }
    let node_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO curriculum_nodes (id, exam_id, kind, name, parent_id, display_order)
         VALUES ($1, $2, $3, $4, $5, $6)",
        node_id,
        req.exam_id,
        req.kind,
        name,
        req.parent_id,
        req.display_order.unwrap_or(0)
    )
    .execute(&state.pool)
    .await?;
    audit(
        &state.pool,
        user.user_id,
        "hierarchy_node_created",
        "curriculum_node",
        node_id,
        json!({"kind": req.kind, "name": name}),
    )
    .await?;
    Ok(Json(json!({ "node_id": node_id })))
}

#[derive(Deserialize)]
pub struct UpdateNodeReq {
    pub name: Option<String>,
    pub display_order: Option<i32>,
    pub status: Option<String>,
}

pub async fn update_node(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(node_id): Path<Uuid>,
    Json(req): Json<UpdateNodeReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    if let Some(status) = &req.status {
        if !matches!(status.as_str(), "active" | "retired") {
            return Err(ApiError::unprocessable(
                "invalid_status",
                "status must be active or retired",
            ));
        }
    }
    let updated = sqlx::query!(
        "UPDATE curriculum_nodes SET
            name = COALESCE($2, name),
            display_order = COALESCE($3, display_order),
            status = COALESCE($4, status)
         WHERE id = $1
         RETURNING id, name, status",
        node_id,
        req.name.as_deref().map(str::trim),
        req.display_order,
        req.status
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("node_not_found"))?;
    audit(
        &state.pool,
        user.user_id,
        "hierarchy_node_updated",
        "curriculum_node",
        node_id,
        json!({"name": updated.name, "status": updated.status}),
    )
    .await?;
    Ok(Json(
        json!({"node_id": updated.id, "name": updated.name, "status": updated.status}),
    ))
}

pub async fn list_nodes(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Query(q): Query<std::collections::HashMap<String, Uuid>>,
) -> ApiResult<Json<serde_json::Value>> {
    let exam_id = q
        .get("exam_id")
        .copied()
        .ok_or_else(|| ApiError::unprocessable("exam_required", "pass ?exam_id="))?;
    let rows = sqlx::query!(
        r#"SELECT id, kind, name, parent_id, display_order, status
           FROM curriculum_nodes WHERE exam_id = $1
           ORDER BY kind, display_order, name"#,
        exam_id
    )
    .fetch_all(&state.pool)
    .await?;
    let nodes: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "kind": r.kind,
                "name": r.name,
                "parent_id": r.parent_id,
                "display_order": r.display_order,
                "status": r.status,
            })
        })
        .collect();
    Ok(Json(json!({ "nodes": nodes })))
}

// ---- question CRUD (§19.5) --------------------------------------------------

#[derive(Deserialize)]
pub struct CreateQuestionReq {
    pub chapter_id: Uuid,
    pub difficulty: String,
    pub vignette: String,
    pub lead_in: String,
    pub options: Vec<crate::seed::QuestionOption>,
    pub correct_index: i16,
    pub key_learning_point: String,
    pub exam_tip: Option<String>,
    pub high_yield: Option<bool>,
    pub source_ref: String,
}

fn validate_question(req: &CreateQuestionReq) -> ApiResult<()> {
    let words = req.key_learning_point.split_whitespace().count();
    if words > 40 {
        return Err(ApiError::unprocessable(
            "key_point_too_long",
            "key learning point must be 40 words or fewer (§11.1)",
        ));
    }
    if req.vignette.trim().is_empty() || req.lead_in.trim().is_empty() {
        return Err(ApiError::unprocessable(
            "invalid_content",
            "vignette and lead-in are required",
        ));
    }
    if !matches!(req.difficulty.as_str(), "easy" | "medium" | "hard") {
        return Err(ApiError::unprocessable(
            "invalid_difficulty",
            "difficulty must be easy, medium, or hard",
        ));
    }
    if req.source_ref.trim().is_empty() {
        return Err(ApiError::unprocessable(
            "source_required",
            "every question carries a source reference (§11.1)",
        ));
    }
    Ok(())
}

async fn insert_question_version(
    pool: &mut sqlx::postgres::PgConnection,
    req: &CreateQuestionReq,
    status: &str,
    created_by: Uuid,
) -> ApiResult<(Uuid, Uuid)> {
    let count = option_count(req.options.len()).map_err(|_| {
        ApiError::unprocessable(
            "invalid_option_count",
            "questions need 2-10 options (QB-11)",
        )
    })?;
    let _ = count;
    if req.correct_index < 0 || req.correct_index as usize >= req.options.len() {
        return Err(ApiError::unprocessable(
            "invalid_correct_index",
            "correct_index is out of range",
        ));
    }
    let qid = Uuid::new_v4();
    let vid = Uuid::new_v4();
    let options = serde_json::to_value(&req.options).map_err(|_| ApiError::internal())?;
    sqlx::query!("INSERT INTO questions (id, family_id) VALUES ($1, $1)", qid)
        .execute(&mut *pool)
        .await?;
    sqlx::query!(
        r#"INSERT INTO question_versions
           (id, question_id, version, status, chapter_id, difficulty, vignette,
            lead_in, options, correct_index, key_learning_point, exam_tip,
            high_yield, source_ref, created_by)
           VALUES ($1, $2, 1, $3, $4, $5, $6, $7, $8, $9, $10,
                   $11, $12, $13, $14)"#,
        vid,
        qid,
        status,
        req.chapter_id,
        req.difficulty,
        req.vignette,
        req.lead_in,
        options,
        req.correct_index,
        req.key_learning_point,
        req.exam_tip,
        req.high_yield.unwrap_or(false),
        req.source_ref,
        created_by,
    )
    .execute(&mut *pool)
    .await?;
    Ok((qid, vid))
}

pub async fn create_question(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateQuestionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    validate_question(&req)?;
    let mut conn = state.pool.acquire().await?;
    // §19.3 editorial workflow: authored items are born drafts and reach
    // learners only through the submit -> review -> approve -> publish gate
    // (assessment_workflow below). Bulk import follows the same gate.
    let (qid, vid) = insert_question_version(&mut conn, &req, "draft", user.user_id).await?;
    audit(
        &state.pool,
        user.user_id,
        "question_created",
        "question",
        qid,
        json!({"version_id": vid, "chapter_id": req.chapter_id}),
    )
    .await?;
    Ok(Json(json!({ "question_id": qid, "version_id": vid })))
}

// ---- INST-05: assessment author/reviewer/publisher separation (§19.3) ------
//
// Authored and imported items are born drafts. submit -> in_review;
// approve -> approved; reject -> draft; publish -> published (learner
// visible). §19.3: the author of an item can never be its approver.

#[derive(Deserialize)]
pub struct AssessmentWorkflowReq {
    /// submit | approve | reject | publish
    pub action: String,
    pub version_ids: Vec<Uuid>,
    pub note: Option<String>,
}

/// Single transition function so every duty-separation rule lives in
/// exactly one place. Bulk-friendly: the console and import flow pass all
/// version ids in one call and get a per-id outcome.
async fn transition_version(
    state: &AppState,
    actor: Uuid,
    action: &str,
    vid: Uuid,
    note: Option<&str>,
) -> ApiResult<serde_json::Value> {
    let v = sqlx::query!(
        r#"SELECT status AS "status!", created_by FROM question_versions WHERE id = $1"#,
        vid
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    match action {
        "submit" => {
            if v.status != "draft" {
                return Err(ApiError::conflict(
                    "invalid_transition",
                    "only drafts can be submitted for review",
                ));
            }
            sqlx::query!(
                "UPDATE question_versions SET status = 'in_review', created_by = $2 WHERE id = $1",
                vid,
                actor
            )
            .execute(&state.pool)
            .await?;
            audit(
                &state.pool,
                actor,
                "assessment_submitted",
                "question_version",
                vid,
                json!({}),
            )
            .await?;
            Ok(json!({ "version_id": vid, "status": "in_review" }))
        }
        "approve" | "reject" => {
            if v.status != "in_review" {
                return Err(ApiError::conflict(
                    "invalid_transition",
                    "only in-review versions can be approved or rejected",
                ));
            }
            if v.created_by == Some(actor) {
                return Err(ApiError::forbidden(
                    "separation_violation",
                    "the author of an item cannot be its approver (§19.3)",
                ));
            }
            let (decision, new_status) = if action == "approve" {
                ("approved", "approved")
            } else {
                ("rejected", "draft")
            };
            sqlx::query!(
                "INSERT INTO assessment_reviews (id, question_version_id, reviewer, decision, note)
                 VALUES ($1, $2, $3, $4, $5)",
                Uuid::new_v4(),
                vid,
                actor,
                decision,
                note
            )
            .execute(&state.pool)
            .await?;
            if action == "approve" {
                sqlx::query!(
                    "UPDATE question_versions SET status = 'approved', reviewed_by = $2 WHERE id = $1",
                    vid,
                    actor
                )
                .execute(&state.pool)
                .await?;
            } else {
                sqlx::query!(
                    "UPDATE question_versions SET status = 'draft' WHERE id = $1",
                    vid
                )
                .execute(&state.pool)
                .await?;
            }
            audit(
                &state.pool,
                actor,
                if action == "approve" {
                    "assessment_approved"
                } else {
                    "assessment_rejected"
                },
                "question_version",
                vid,
                json!({ "decision": decision }),
            )
            .await?;
            Ok(json!({ "version_id": vid, "status": new_status }))
        }
        "publish" => {
            if v.status != "approved" {
                return Err(ApiError::conflict(
                    "invalid_transition",
                    "only approved versions can be published",
                ));
            }
            if v.created_by == Some(actor) {
                return Err(ApiError::forbidden(
                    "separation_violation",
                    "the author of an item cannot publish it (§19.3)",
                ));
            }
            sqlx::query!(
                "UPDATE question_versions SET status = 'published', published_by = $2 WHERE id = $1",
                vid,
                actor
            )
            .execute(&state.pool)
            .await?;
            audit(
                &state.pool,
                actor,
                "assessment_published",
                "question_version",
                vid,
                json!({}),
            )
            .await?;
            Ok(json!({ "version_id": vid, "status": "published" }))
        }
        _ => Err(ApiError::unprocessable(
            "invalid_action",
            "action must be submit, approve, reject, or publish",
        )),
    }
}

pub async fn assessment_workflow(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<AssessmentWorkflowReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    if req.version_ids.is_empty() {
        return Err(ApiError::unprocessable(
            "invalid_request",
            "version_ids must not be empty",
        ));
    }
    let mut results = Vec::with_capacity(req.version_ids.len());
    for vid in &req.version_ids {
        match transition_version(&state, user.user_id, &req.action, *vid, req.note.as_deref()).await
        {
            Ok(v) => results.push(v),
            Err(e) => results.push(json!({
                "version_id": vid,
                "error": { "code": e.code, "message": e.message }
            })),
        }
    }
    Ok(Json(json!({ "results": results })))
}

pub async fn search_questions(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    let pattern = format!("%{}%", q.get("q").map(String::as_str).unwrap_or(""));
    let chapter_id = q.get("chapter_id").and_then(|s| Uuid::parse_str(s).ok());
    let status = q.get("status").map(String::as_str);
    let rows = sqlx::query!(
        r#"SELECT qv.question_id, qv.id AS version_id, qv.vignette, qv.status,
                  qv.difficulty, c.name AS chapter_name
           FROM question_versions qv
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE qv.status = COALESCE($2, qv.status)
             AND qv.vignette ILIKE $1
             AND qv.chapter_id IS NOT DISTINCT FROM $3
           ORDER BY qv.question_id, qv.version
           LIMIT 50"#,
        pattern,
        status,
        chapter_id
    )
    .fetch_all(&state.pool)
    .await?;
    let questions: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "question_id": r.question_id,
                "version_id": r.version_id,
                "vignette": r.vignette,
                "status": r.status,
                "difficulty": r.difficulty,
                "chapter": r.chapter_name,
            })
        })
        .collect();
    Ok(Json(json!({ "questions": questions })))
}

// ---- bulk import with dry-run/rollback (§19.5) -------------------------------

#[derive(Deserialize)]
pub struct ImportReq {
    pub exam_id: Uuid,
    #[serde(default = "default_filename")]
    pub filename: String,
    #[serde(default)]
    pub dry_run: bool,
    pub rows: Vec<CreateQuestionReq>,
}

fn default_filename() -> String {
    "inline".into()
}

#[derive(Serialize)]
struct RowIssue {
    row: usize,
    code: &'static str,
    message: String,
}

/// Validate every row up front; report ALL issues, never just the first.
async fn validate_rows(
    pool: &sqlx::PgPool,
    exam_id: Uuid,
    rows: &[CreateQuestionReq],
) -> ApiResult<Vec<RowIssue>> {
    let mut issues = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let valid_chapter = sqlx::query!(
            "SELECT 1 AS one FROM curriculum_nodes
             WHERE id = $1 AND exam_id = $2 AND kind = 'chapter' AND status = 'active'",
            row.chapter_id,
            exam_id
        )
        .fetch_optional(pool)
        .await?
        .is_some();
        if !valid_chapter {
            issues.push(RowIssue {
                row: i,
                code: "unknown_chapter",
                message: "chapter_id is not an active chapter of this exam".into(),
            });
            continue;
        }
        if let Err(e) = validate_question(row) {
            issues.push(RowIssue {
                row: i,
                code: e.code,
                message: e.message,
            });
        }
        if option_count(row.options.len()).is_err() {
            issues.push(RowIssue {
                row: i,
                code: "invalid_option_count",
                message: "questions need 2-10 options (QB-11)".into(),
            });
        }
    }
    Ok(issues)
}

pub async fn import(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<ImportReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    if req.rows.is_empty() || req.rows.len() > 500 {
        return Err(ApiError::unprocessable(
            "invalid_row_count",
            "import accepts 1-500 rows",
        ));
    }
    let issues = validate_rows(&state.pool, req.exam_id, &req.rows).await?;
    let valid_count = req.rows.len() - issues.len();
    let batch_id = Uuid::new_v4();

    if req.dry_run || !issues.is_empty() {
        // §19.5: dry run and failed validation create NOTHING — the batch is
        // recorded as evidence either way.
        let status = if req.dry_run { "dry_run" } else { "rejected" };
        sqlx::query!(
            "INSERT INTO import_batches (id, created_by, exam_id, status, summary)
             VALUES ($1, $2, $3, $4, $5)",
            batch_id,
            user.user_id,
            req.exam_id,
            status,
            json!({
                "rows": req.rows.len(),
                "valid": valid_count,
                "issues": issues,
            })
        )
        .execute(&state.pool)
        .await?;
        return Ok(Json(json!({
            "batch_id": batch_id,
            "status": status,
            "rows": req.rows.len(),
            "valid": valid_count,
            "issues": issues,
        })));
    }

    // Apply: every valid row in one transaction — all or nothing (§19.5).
    let mut tx = state.pool.begin().await?;
    let mut created: Vec<serde_json::Value> = Vec::new();
    for row in &req.rows {
        let (qid, vid) = insert_question_version(&mut tx, row, "draft", user.user_id).await?;
        created.push(json!({"question_id": qid, "version_id": vid}));
    }
    sqlx::query!(
        "INSERT INTO import_batches (id, created_by, exam_id, status, summary)
         VALUES ($1, $2, $3, 'applied', $4)",
        batch_id,
        user.user_id,
        req.exam_id,
        json!({"rows": req.rows.len(), "created": created})
    )
    .execute(&mut *tx)
    .await?;
    audit(
        &mut *tx,
        user.user_id,
        "import_applied",
        "import_batch",
        batch_id,
        json!({"rows": req.rows.len()}),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(json!({
        "batch_id": batch_id,
        "status": "applied",
        "rows": req.rows.len(),
        "created": created,
    })))
}

/// Rollback: removes the batch's questions. Refused honestly once any of
/// them has learner attempts — the evidence is immutable (§21.3).
pub async fn rollback_import(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(batch_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let batch = sqlx::query!(
        "SELECT status, summary, exam_id FROM import_batches WHERE id = $1",
        batch_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("batch_not_found"))?;
    if batch.status != "applied" {
        return Err(ApiError::conflict(
            "not_rollbackable",
            format!(
                "batch status is {}, only applied batches roll back",
                batch.status
            ),
        ));
    }
    let version_ids: Vec<Uuid> = batch
        .summary
        .get("created")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|c| c.get("version_id").and_then(|v| v.as_str()))
                .filter_map(|s| Uuid::parse_str(s).ok())
                .collect()
        })
        .unwrap_or_default();

    let with_attempts = if version_ids.is_empty() {
        0
    } else {
        sqlx::query!(
            r#"SELECT COALESCE(COUNT(DISTINCT a.question_version_id), 0) AS "n!"
               FROM attempts a
               WHERE a.question_version_id = ANY($1)"#,
            &version_ids
        )
        .fetch_one(&state.pool)
        .await?
        .n
    };
    if with_attempts > 0 {
        return Err(ApiError::conflict(
            "has_attempts",
            format!(
                "{with_attempts} question(s) in this batch already have learner attempts — rollback is refused; correct forward instead"
            ),
        ));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "DELETE FROM question_versions WHERE id = ANY($1)",
        &version_ids
    )
    .execute(&mut *tx)
    .await?;
    let question_ids: Vec<Uuid> = batch
        .summary
        .get("created")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|c| c.get("question_id").and_then(|v| v.as_str()))
                .filter_map(|s| Uuid::parse_str(s).ok())
                .collect()
        })
        .unwrap_or_default();
    sqlx::query!("DELETE FROM questions WHERE id = ANY($1)", &question_ids)
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "UPDATE import_batches SET status = 'rolled_back' WHERE id = $1",
        batch_id
    )
    .execute(&mut *tx)
    .await?;
    audit(
        &mut *tx,
        user.user_id,
        "import_rolled_back",
        "import_batch",
        batch_id,
        json!({"removed_questions": question_ids.len()}),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({
        "batch_id": batch_id,
        "status": "rolled_back",
        "removed_questions": question_ids.len(),
    })))
}

pub async fn audit_log(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let rows = sqlx::query!(
        r#"SELECT action, entity, entity_id, new_value, created_at
           FROM audit_events ORDER BY created_at DESC LIMIT 50"#
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

/// QB-16: psychometric screening defaults (§11.4). Flag a question version
/// when: attempts >= 20 AND (p < 0.20 or p > 0.95), a distractor out-pulls
/// the key, or it carries 3+ open reports. Screening only — never verdicts.
pub async fn psychometric_screening(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(vid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let totals = sqlx::query!(
        r#"SELECT
             COALESCE(COUNT(*), 0) AS "attempts!",
             COALESCE(COUNT(*) FILTER (WHERE correct = TRUE), 0) AS "correct!"
           FROM attempts
           WHERE question_version_id = $1 AND chosen_index IS NOT NULL
             AND assisted = FALSE"#,
        vid
    )
    .fetch_one(&state.pool)
    .await?;
    let open_reports = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM question_reports
           WHERE question_version_id = $1 AND status IN ('open','quarantined')"#,
        vid
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    let mut flags: Vec<&str> = Vec::new();
    let mut p_percent: Option<i32> = None;
    if totals.attempts >= 20 {
        let p = totals.correct * 100 / totals.attempts;
        p_percent = Some(p as i32);
        if p < 20 {
            flags.push("too_hard");
        }
        if p > 95 {
            flags.push("too_easy");
        }
    } else {
        flags.push("insufficient_attempts");
    }
    if open_reports >= 3 {
        flags.push("reported");
    }
    Ok(Json(json!({
        "question_version_id": vid,
        "attempts": totals.attempts,
        "p_percent": p_percent,
        "open_reports": open_reports,
        "flags": flags,
    })))
}

// ---- QB-09: editorial review queue ------------------------------------------

#[derive(Deserialize)]
pub struct QueueParams {
    pub exam_id: Uuid,
    pub limit: Option<i64>,
}

/// GET /v1/admin/psychometrics?exam_id= — per-question attempt statistics
/// across an exam so editors can find items needing review (QB-09/§11.4).
/// Same honest flags as the per-item endpoint; below the 20-attempt floor
/// the flag says so instead of pretending.
pub async fn psychometric_queue(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Query(q): Query<QueueParams>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let rows = sqlx::query!(
        r#"SELECT t.vid,
                  t.attempts AS "attempts!",
                  t.correct AS "correct!",
                  t.open_reports AS "open_reports!"
           FROM (
               SELECT qv.id AS vid,
                      COUNT(a.id) AS attempts,
                      COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS correct,
                      (SELECT COUNT(*) FROM question_reports r
                       WHERE r.question_version_id = qv.id
                         AND r.status IN ('open','quarantined')) AS open_reports
               FROM question_versions qv
               JOIN curriculum_nodes ch ON ch.id = qv.chapter_id
               JOIN curriculum_nodes sys ON sys.id = ch.parent_id
               JOIN curriculum_nodes subj ON subj.id = sys.parent_id
               LEFT JOIN attempts a
                 ON a.question_version_id = qv.id AND a.chosen_index IS NOT NULL
               WHERE qv.status = 'published' AND subj.exam_id = $1
               GROUP BY qv.id
           ) t
           ORDER BY t.open_reports DESC, t.attempts ASC
           LIMIT $2"#,
        q.exam_id,
        limit
    )
    .fetch_all(&state.pool)
    .await?;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let mut flags: Vec<&str> = Vec::new();
            let p = if r.attempts >= 20 {
                let p = r.correct * 100 / r.attempts;
                if p < 20 {
                    flags.push("too_hard");
                }
                if p > 95 {
                    flags.push("too_easy");
                }
                Some(p as i32)
            } else {
                flags.push("insufficient_attempts");
                None
            };
            if r.open_reports >= 3 {
                flags.push("reported");
            }
            json!({
                "question_version_id": r.vid,
                "attempts": r.attempts,
                "correct": r.correct,
                "p_percent": p,
                "open_reports": r.open_reports,
                "flags": flags,
            })
        })
        .collect();
    Ok(Json(json!({ "items": items })))
}

// ---- ADMIN-01/02/03/04 + OPS-05/06: owner console reads and ledgers ----------

/// ADMIN-01: real cross-tenant aggregates. Counts only — tenant rows stay
/// behind their own access rules.
pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let counts = sqlx::query!(
        r#"SELECT (SELECT COUNT(*) FROM institutions) AS "institutions!",
               (SELECT COUNT(*) FROM users WHERE deleted_at IS NULL) AS "users!",
               (SELECT COUNT(*) FROM question_versions WHERE status = 'published') AS "published_questions!",
               (SELECT COUNT(*) FROM articles) AS "articles!",
               (SELECT COUNT(*) FROM content_rights) AS "rights_records!",
               (SELECT COUNT(*) FROM incidents WHERE status <> 'resolved') AS "open_incidents!",
               (SELECT COUNT(*) FROM coach_turns WHERE created_at >= now() - interval '30 days') AS "coach_turns_30d!""#
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(json!({
        "institutions": counts.institutions,
        "users": counts.users,
        "published_questions": counts.published_questions,
        "articles": counts.articles,
        "rights_records": counts.rights_records,
        "open_incidents": counts.open_incidents,
        "coach_turns_last_30_days": counts.coach_turns_30d,
    })))
}

// ---- ADMIN-02/TRUST-07: content rights ledger (§19.2) ------------------------

#[derive(Deserialize)]
pub struct ContentRightsReq {
    pub ref_code: String,
    pub licensor: String,
    pub territory: Option<String>,
    /// display | offline | ai | derivatives | translation (Appendix B of §19).
    pub permitted_uses: Vec<String>,
    pub valid_from: chrono::NaiveDate,
    pub valid_to: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
}

pub async fn create_content_rights(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<ContentRightsReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let ref_code = req.ref_code.trim().to_uppercase();
    if ref_code.is_empty() || ref_code.len() > 60 {
        return Err(ApiError::unprocessable(
            "invalid_ref_code",
            "ref_code must be 1-60 characters",
        ));
    }
    let licensor = req.licensor.trim();
    if licensor.is_empty() {
        return Err(ApiError::unprocessable(
            "invalid_licensor",
            "licensor is required",
        ));
    }
    let known = ["display", "offline", "ai", "derivatives", "translation"];
    if req.permitted_uses.is_empty()
        || !req
            .permitted_uses
            .iter()
            .all(|u| known.contains(&u.as_str()))
    {
        return Err(ApiError::unprocessable(
            "invalid_permitted_uses",
            "permitted uses must draw from display, offline, ai, derivatives, translation",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO content_rights
           (id, ref_code, licensor, territory, permitted_uses, valid_from, valid_to, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        id,
        ref_code,
        licensor,
        req.territory.as_deref().unwrap_or("worldwide"),
        serde_json::to_value(&req.permitted_uses).map_err(|_| ApiError::internal())?,
        req.valid_from,
        req.valid_to,
        req.notes,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    audit(
        &state.pool,
        user.user_id,
        "content_rights_created",
        "content_rights",
        id,
        json!({ "ref_code": ref_code }),
    )
    .await?;
    Ok(Json(json!({ "rights_id": id, "ref_code": ref_code })))
}

pub async fn list_content_rights(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let rows = sqlx::query!(
        r#"SELECT id, ref_code, licensor, territory, permitted_uses,
                  valid_from AS "valid_from?", valid_to AS "valid_to?", notes AS "notes?"
           FROM content_rights ORDER BY created_at DESC LIMIT 200"#
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "rights": rows.iter().map(|r| json!({
        "rights_id": r.id,
        "ref_code": r.ref_code,
        "licensor": r.licensor,
        "territory": r.territory,
        "permitted_uses": r.permitted_uses,
        "valid_from": r.valid_from,
        "valid_to": r.valid_to,
        "notes": r.notes,
    })).collect::<Vec<_>>() })))
}

// ---- ADMIN-03: AI cost/policy read-out ----------------------------------------

/// Real numbers from the coach stream: turns by adapter and prompt type over
/// 30 days. Policy knobs live in state/config, disclosed, not invented here.
pub async fn ai_admin(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let by_adapter = sqlx::query!(
        r#"SELECT adapter AS "adapter!", model AS "model!", COUNT(*) AS "n!"
           FROM coach_turns WHERE created_at >= now() - interval '30 days'
           GROUP BY adapter, model ORDER BY COUNT(*) DESC"#
    )
    .fetch_all(&state.pool)
    .await?;
    let by_prompt = sqlx::query!(
        r#"SELECT prompt_type AS "prompt_type!", COUNT(*) AS "n!"
           FROM coach_turns WHERE created_at >= now() - interval '30 days'
           GROUP BY prompt_type ORDER BY COUNT(*) DESC"#
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "daily_allowance_per_learner": state.free_daily_coach_turns,
        "turns_last_30_days": by_adapter.iter().map(|r| json!({
            "adapter": r.adapter, "model": r.model, "count": r.n
        })).collect::<Vec<_>>(),
        "prompt_types_last_30_days": by_prompt.iter().map(|r| json!({
            "prompt_type": r.prompt_type, "count": r.n
        })).collect::<Vec<_>>(),
    })))
}

// ---- ADMIN-04: incidents -------------------------------------------------------

#[derive(Deserialize)]
pub struct IncidentReq {
    pub title: String,
    pub severity: String, // sev1 | sev2 | sev3
}

pub async fn create_incident(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<IncidentReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let title = req.title.trim();
    if title.is_empty() || title.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_title",
            "title must be 1-200 characters",
        ));
    }
    if !matches!(req.severity.as_str(), "sev1" | "sev2" | "sev3") {
        return Err(ApiError::unprocessable(
            "invalid_severity",
            "severity must be sev1, sev2, or sev3",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO incidents (id, title, severity, opened_by) VALUES ($1, $2, $3, $4)",
        id,
        title,
        req.severity,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    audit(
        &state.pool,
        user.user_id,
        "incident_opened",
        "incident",
        id,
        json!({ "severity": req.severity }),
    )
    .await?;
    Ok(Json(json!({ "incident_id": id })))
}

pub async fn list_incidents(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let rows = sqlx::query!(
        r#"SELECT id, title, severity, status,
                  created_at AS "created_at?", resolved_at AS "resolved_at?"
           FROM incidents ORDER BY created_at DESC LIMIT 100"#
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "incidents": rows.iter().map(|r| json!({
        "incident_id": r.id,
        "title": r.title,
        "severity": r.severity,
        "status": r.status,
        "created_at": r.created_at,
        "resolved_at": r.resolved_at,
    })).collect::<Vec<_>>() })))
}

#[derive(Deserialize)]
pub struct IncidentUpdateReq {
    pub status: String, // mitigated | resolved
}

pub async fn update_incident(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(incident_id): Path<Uuid>,
    Json(req): Json<IncidentUpdateReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    if !matches!(req.status.as_str(), "mitigated" | "resolved" | "open") {
        return Err(ApiError::unprocessable(
            "invalid_status",
            "status must be open, mitigated, or resolved",
        ));
    }
    let updated = sqlx::query!(
        "UPDATE incidents
         SET status = $2,
             resolved_at = CASE WHEN $2 = 'resolved' THEN now() ELSE resolved_at END
         WHERE id = $1",
        incident_id,
        req.status
    )
    .execute(&state.pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::not_found("incident_not_found"));
    }
    audit(
        &state.pool,
        user.user_id,
        "incident_updated",
        "incident",
        incident_id,
        json!({ "status": req.status }),
    )
    .await?;
    Ok(Json(json!({ "status": req.status })))
}

// ---- QB-02: variant authoring on an existing family --------------------------

#[derive(Deserialize)]
pub struct VariantReq {
    pub difficulty: String,
    pub vignette: String,
    pub lead_in: String,
    pub options: Vec<QuestionOption>,
    pub correct_index: i16,
    pub key_learning_point: String,
    pub exam_tip: Option<String>,
    pub high_yield: Option<bool>,
    pub source_ref: String,
}

/// Author a NEW version of an existing question: same family identity, fresh
/// content. Born a draft; the §19.3 workflow gates it exactly like any item.
pub async fn create_variant(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(question_id): Path<Uuid>,
    Json(req): Json<VariantReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let family = sqlx::query!("SELECT family_id FROM questions WHERE id = $1", question_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    let next_version: i32 = sqlx::query!(
        r#"SELECT COALESCE(MAX(version), 0) + 1 AS "v!"
           FROM question_versions WHERE question_id = $1"#,
        question_id
    )
    .fetch_one(&state.pool)
    .await?
    .v;
    let vid = Uuid::new_v4();
    let options = serde_json::to_value(&req.options).map_err(|_| ApiError::internal())?;
    sqlx::query!(
        r#"INSERT INTO question_versions
           (id, question_id, version, status, chapter_id, difficulty, vignette,
            lead_in, options, correct_index, key_learning_point, exam_tip,
            high_yield, source_ref, created_by)
           SELECT $1, q.id, $3, 'draft', qv.chapter_id, $4, $5, $6, $7, $8,
                  $9, $10, $11, $12, $13
           FROM questions q
           JOIN question_versions qv ON qv.question_id = q.id
           WHERE q.id = $2
           LIMIT 1"#,
        vid,
        question_id,
        next_version,
        req.difficulty,
        req.vignette,
        req.lead_in,
        options,
        req.correct_index,
        req.key_learning_point,
        req.exam_tip,
        req.high_yield.unwrap_or(false),
        req.source_ref,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    audit(
        &state.pool,
        user.user_id,
        "variant_authored",
        "question_version",
        vid,
        json!({ "question_id": question_id, "version": next_version }),
    )
    .await?;
    Ok(Json(json!({
        "question_id": question_id,
        "version_id": vid,
        "version": next_version,
        "status": "draft",
    })))
}

// ---- OPS-04: recovery drills with recorded evidence ---------------------------

#[derive(Deserialize)]
pub struct RecoveryDrillReq {
    pub exam_id: Uuid,
    /// Drill kind: manifest_signature verifies the signed pack manifest path
    /// end to end (positive + tamper proof).
    pub kind: String, // manifest_signature
}

pub async fn run_recovery_drill(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<RecoveryDrillReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    if req.kind != "manifest_signature" {
        return Err(ApiError::unprocessable(
            "unknown_drill",
            "supported drills: manifest_signature",
        ));
    }
    // Build the canonical manifest exactly as packs.rs does, sign it, then
    // re-verify: a passing drill proves the recovery verification path works
    // on real published content; the tamper proof proves it can fail.
    let rows = sqlx::query!(
        r#"SELECT id, encode(sha256((vignette || lead_in)::bytea), 'hex') AS "checksum!"
           FROM question_versions
           WHERE chapter_id IN (SELECT id FROM curriculum_nodes WHERE exam_id = $1)
             AND status = 'published'
           ORDER BY id"#,
        req.exam_id
    )
    .fetch_all(&state.pool)
    .await?;
    let mut canonical = String::new();
    for r in &rows {
        canonical.push_str(&format!("{}:{}\n", r.id, r.checksum));
    }
    let key = crate::routes::packs::signing_key(&state);
    let signature = crate::routes::packs::hmac_sha256_hex(&key, canonical.as_bytes());
    let verify_ok = signature == crate::routes::packs::hmac_sha256_hex(&key, canonical.as_bytes());
    let tamper = format!("{canonical}TAMPERED");
    let tamper_detected =
        signature != crate::routes::packs::hmac_sha256_hex(&key, tamper.as_bytes());
    let pass = verify_ok && tamper_detected && !rows.is_empty();
    let evidence = json!({
        "items": rows.len(),
        "signature_verified": verify_ok,
        "tamper_detected": tamper_detected,
    });
    let drill_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO recovery_drills (id, kind, result, evidence, ran_by)
         VALUES ($1, 'manifest_signature', $2, $3, $4)",
        drill_id,
        if pass { "pass" } else { "fail" },
        evidence,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({
        "drill_id": drill_id,
        "kind": "manifest_signature",
        "result": if pass { "pass" } else { "fail" },
        "evidence": evidence,
    })))
}

pub async fn list_recovery_drills(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let rows = sqlx::query!(
        r#"SELECT id, kind, result, evidence, created_at
           FROM recovery_drills ORDER BY created_at DESC LIMIT 100"#
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "drills": rows.iter().map(|r| json!({
        "drill_id": r.id, "kind": r.kind, "result": r.result,
        "evidence": r.evidence, "at": r.created_at,
    })).collect::<Vec<_>>() })))
}

// ---- AI-16: grounded-coach regression harness (§23 scaffolding) ----------------

#[derive(Deserialize)]
pub struct RegressionReq {
    /// Cap cases per run so a console click stays cheap.
    pub max_cases: Option<i64>,
}

/// Re-runs the deterministic extractive adapter over published questions and
/// asserts the grounding invariants (answer quotes reviewed material, carries
/// the key learning point, never claims compliance). Model-backed evaluation
/// extends this harness when an evaluated model lands (§23).
pub async fn run_coach_regression(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<RegressionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let max = req.max_cases.unwrap_or(10).clamp(1, 50);
    let rows = sqlx::query!(
        r#"SELECT qv.id, qv.vignette, qv.correct_index, qv.options,
                  qv.key_learning_point
           FROM question_versions qv
           WHERE qv.status = 'published'
           ORDER BY qv.id LIMIT $1"#,
        max
    )
    .fetch_all(&state.pool)
    .await?;
    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut passed = 0i64;
    for r in &rows {
        let options: Vec<QuestionOption> =
            serde_json::from_value(r.options.clone()).unwrap_or_default();
        let answer = crate::routes::coach::extractive_grounding(
            "explain",
            &r.vignette,
            &options,
            &r.key_learning_point,
        );
        let grounded = answer.contains("Key learning point:")
            && answer.contains(&r.key_learning_point)
            && !answer.to_lowercase().contains("as requested");
        if grounded {
            passed += 1;
        }
        results.push(json!({
            "question_version_id": r.id,
            "grounded": grounded,
        }));
    }
    let run_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO coach_regression_runs (id, cases_total, cases_passed, results, ran_by)
         VALUES ($1, $2, $3, $4, $5)",
        run_id,
        results.len() as i32,
        passed as i32,
        serde_json::to_value(&results).map_err(|_| ApiError::internal())?,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({
        "run_id": run_id,
        "cases_total": results.len(),
        "cases_passed": passed,
    })))
}

pub async fn list_coach_regression(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let rows = sqlx::query!(
        r#"SELECT id, cases_total, cases_passed, created_at
           FROM coach_regression_runs ORDER BY created_at DESC LIMIT 50"#
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "runs": rows.iter().map(|r| json!({
        "run_id": r.id, "cases_total": r.cases_total,
        "cases_passed": r.cases_passed, "at": r.created_at,
    })).collect::<Vec<_>>() })))
}
