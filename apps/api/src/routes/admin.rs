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
            high_yield, source_ref)
           VALUES ($1, $2, 1, 'published', $3, $4, $5, $6, $7, $8, $9, $10,
                   $11, $12)"#,
        vid,
        qid,
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
    let (qid, vid) = insert_question_version(&mut conn, &req).await?;
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

pub async fn search_questions(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    let pattern = format!("%{}%", q.get("q").map(String::as_str).unwrap_or(""));
    let rows = sqlx::query!(
        r#"SELECT qv.question_id, qv.id AS version_id, qv.vignette, qv.status,
                  qv.difficulty, c.name AS chapter_name
           FROM question_versions qv
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE qv.status = COALESCE($2, qv.status)
             AND qv.vignette ILIKE $1
           ORDER BY qv.question_id, qv.version
           LIMIT 50"#,
        pattern,
        q.get("status")
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
        let (qid, vid) = insert_question_version(&mut *tx, row).await?;
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
