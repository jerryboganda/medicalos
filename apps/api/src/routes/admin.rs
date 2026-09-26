//! ADMIN-06 baseline: hierarchy management, question CRUD, and bulk import
//! with dry-run/rollback (§19.5). Every mutation writes an audit event
//! (§19.5 audit log) and is admin-token gated until role-aware accounts
//! (18.1) land. JSON, CSV, and XLSX imports share one validation and rollback
//! pipeline.
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use calamine::Reader;
use ed25519_dalek::{Signer, Verifier};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Cursor;
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

pub(crate) async fn audit(
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

fn validate_hint_length(hint: Option<&str>) -> ApiResult<()> {
    if hint.is_some_and(|hint| hint.chars().count() > 2000) {
        return Err(ApiError::unprocessable(
            "hint_too_long",
            "hint must be at most 2000 characters",
        ));
    }
    Ok(())
}

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
    pub hint: Option<String>,
    pub high_yield: Option<bool>,
    pub source_ref: String,
    #[serde(default)]
    pub rights_ref: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub media_refs: Vec<String>,
}

fn validate_import_metadata(
    tags: &[String],
    source_refs: &[String],
    media_refs: &[String],
) -> ApiResult<()> {
    for (values, name, limit, length) in [
        (tags, "tags", 100, 100),
        (source_refs, "references", 30, 500),
        (media_refs, "media references", 30, 1000),
    ] {
        if values.len() > limit
            || values.iter().any(|value| {
                value.trim().is_empty()
                    || value.chars().count() > length
                    || value.chars().any(char::is_control)
            })
        {
            return Err(ApiError::unprocessable(
                "invalid_question_metadata",
                format!("{name} must contain at most {limit} non-empty values of at most {length} characters"),
            ));
        }
    }
    Ok(())
}

fn validate_question(req: &CreateQuestionReq) -> ApiResult<()> {
    let words = req.key_learning_point.split_whitespace().count();
    if words > 40 {
        return Err(ApiError::unprocessable(
            "key_point_too_long",
            "key learning point must be 40 words or fewer (§11.1)",
        ));
    }
    validate_hint_length(req.hint.as_deref())?;
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
    if req
        .rights_ref
        .as_deref()
        .is_some_and(|value| value.trim().is_empty() || value.trim().len() > 60)
    {
        return Err(ApiError::unprocessable(
            "invalid_rights_ref",
            "rights_ref must be 1-60 characters when supplied",
        ));
    }
    validate_import_metadata(&req.tags, &req.source_refs, &req.media_refs)?;
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
    let tags: Vec<String> = req
        .tags
        .iter()
        .map(|value| value.trim().to_string())
        .collect();
    let source_refs: Vec<String> = if req.source_refs.is_empty() {
        vec![req.source_ref.trim().to_string()]
    } else {
        req.source_refs
            .iter()
            .map(|value| value.trim().to_string())
            .collect()
    };
    let media_refs: Vec<String> = req
        .media_refs
        .iter()
        .map(|value| value.trim().to_string())
        .collect();
    let rights_ref = req
        .rights_ref
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_uppercase);
    sqlx::query!("INSERT INTO questions (id, family_id) VALUES ($1, $1)", qid)
        .execute(&mut *pool)
        .await?;
    sqlx::query!(
        r#"INSERT INTO question_versions
           (id, question_id, version, status, chapter_id, difficulty, vignette,
            lead_in, options, correct_index, key_learning_point, exam_tip,
             hint, high_yield, source_ref, created_by, tags, source_refs, media_refs, rights_ref)
            VALUES ($1, $2, 1, $3, $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, $14, $15, $16, $17, $18, $19)"#,
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
        req.hint,
        req.high_yield.unwrap_or(false),
        req.source_ref,
        created_by,
        &tags,
        &source_refs,
        &media_refs,
        rights_ref,
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
            let mut tx = state.pool.begin().await?;
            let updated = sqlx::query!(
                "UPDATE question_versions SET status = 'published', published_by = $2 WHERE id = $1 AND status = 'approved'",
                vid,
                actor
            )
            .execute(&mut *tx)
            .await?;
            if updated.rows_affected() != 1 {
                return Err(ApiError::conflict(
                    "invalid_transition",
                    "only approved versions can be published",
                ));
            }
            crate::routes::program::ensure_pregen_on(&mut tx, vid).await?;
            audit(
                &mut *tx,
                actor,
                "assessment_published",
                "question_version",
                vid,
                json!({}),
            )
            .await?;
            tx.commit().await?;
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
    headers: axum::http::HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let pattern = format!("%{}%", q.get("q").map(String::as_str).unwrap_or(""));
    let chapter_id = q
        .get("chapter_id")
        .map(|value| {
            Uuid::parse_str(value).map_err(|_| {
                ApiError::unprocessable("invalid_chapter_id", "chapter_id must be a UUID")
            })
        })
        .transpose()?;
    let status = q.get("status").map(String::as_str);
    let rows = sqlx::query!(
        r#"SELECT qv.question_id, qv.id AS version_id, qv.vignette, qv.status,
                  qv.difficulty, qv.source_ref, qv.rights_ref, qv.tags, qv.source_refs,
                  qv.media_refs, c.name AS chapter_name
           FROM question_versions qv
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE qv.status = COALESCE($2, qv.status)
             AND qv.vignette ILIKE $1
             AND ($3::UUID IS NULL OR qv.chapter_id = $3)
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
                "source_ref": r.source_ref,
                "rights_ref": r.rights_ref,
                "tags": r.tags,
                "references": r.source_refs,
                "media_refs": r.media_refs,
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

#[derive(Deserialize)]
pub struct ImportFileQuery {
    pub exam_id: Uuid,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Serialize)]
struct RowIssue {
    row: usize,
    code: &'static str,
    message: String,
}

#[derive(Default)]
struct ParsedImport {
    rows: Vec<CreateQuestionReq>,
    row_numbers: Vec<usize>,
    issues: Vec<RowIssue>,
    total_rows: usize,
}

const MAX_IMPORT_ROWS: usize = 500;
const MAX_IMPORT_COLUMNS: usize = 40;

fn import_file_error(message: impl Into<String>) -> ApiError {
    ApiError::unprocessable("invalid_import_file", message)
}

fn is_import_header(header: &str) -> bool {
    matches!(
        header,
        "chapter_id"
            | "difficulty"
            | "vignette"
            | "lead_in"
            | "correct_option"
            | "key_learning_point"
            | "source_ref"
            | "rights_ref"
            | "exam_tip"
            | "hint"
            | "high_yield"
            | "tags"
            | "references"
            | "media_refs"
    ) || ["option_", "rationale_"].iter().any(|prefix| {
        header
            .strip_prefix(prefix)
            .and_then(|number| number.parse::<u8>().ok())
            .is_some_and(|number| (1..=10).contains(&number))
    })
}

fn split_import_list(value: &str) -> Vec<String> {
    value
        .split('|')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn optional_import_string(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn import_cell<'a>(headers: &HashMap<String, usize>, values: &'a [String], name: &str) -> &'a str {
    headers
        .get(name)
        .and_then(|index| values.get(*index))
        .map(|value| value.trim())
        .unwrap_or_default()
}

fn import_question(
    headers: &HashMap<String, usize>,
    values: &[String],
    row_number: usize,
) -> ApiResult<CreateQuestionReq> {
    let chapter_id = Uuid::parse_str(import_cell(headers, values, "chapter_id"))
        .map_err(|_| import_file_error(format!("row {row_number}: chapter_id must be a UUID")))?;
    let mut options = Vec::new();
    for index in 1..=10 {
        let text = import_cell(headers, values, &format!("option_{index}"));
        let rationale = import_cell(headers, values, &format!("rationale_{index}"));
        if text.is_empty() && rationale.is_empty() {
            continue;
        }
        if text.is_empty() || rationale.is_empty() {
            return Err(import_file_error(format!(
                "row {row_number}: option_{index} and rationale_{index} must both be filled"
            )));
        }
        options.push(QuestionOption {
            text: text.to_string(),
            rationale: rationale.to_string(),
        });
    }
    let correct_option = import_cell(headers, values, "correct_option")
        .parse::<usize>()
        .ok()
        .filter(|selected| *selected > 0 && *selected <= options.len())
        .ok_or_else(|| {
            import_file_error(format!(
                "row {row_number}: correct_option must select one of the {} populated options",
                options.len()
            ))
        })?;
    let high_yield_value = import_cell(headers, values, "high_yield").to_ascii_lowercase();
    let high_yield = match high_yield_value.as_str() {
        "" => None,
        "true" | "yes" | "1" => Some(true),
        "false" | "no" | "0" => Some(false),
        _ => {
            return Err(import_file_error(format!(
                "row {row_number}: high_yield must be true or false"
            )))
        }
    };
    let source_ref = import_cell(headers, values, "source_ref").to_string();
    let mut source_refs = vec![source_ref.clone()];
    source_refs.extend(split_import_list(import_cell(
        headers,
        values,
        "references",
    )));

    Ok(CreateQuestionReq {
        chapter_id,
        difficulty: import_cell(headers, values, "difficulty").to_string(),
        vignette: import_cell(headers, values, "vignette").to_string(),
        lead_in: import_cell(headers, values, "lead_in").to_string(),
        options,
        correct_index: (correct_option - 1) as i16,
        key_learning_point: import_cell(headers, values, "key_learning_point").to_string(),
        exam_tip: optional_import_string(import_cell(headers, values, "exam_tip")),
        hint: optional_import_string(import_cell(headers, values, "hint")),
        high_yield,
        source_ref,
        rights_ref: optional_import_string(import_cell(headers, values, "rights_ref"))
            .map(|value| value.to_ascii_uppercase()),
        tags: split_import_list(import_cell(headers, values, "tags")),
        source_refs,
        media_refs: split_import_list(import_cell(headers, values, "media_refs")),
    })
}

fn parse_import_table(
    raw_headers: Vec<String>,
    records: Vec<(usize, Vec<String>)>,
) -> ApiResult<ParsedImport> {
    if raw_headers.is_empty() || raw_headers.len() > MAX_IMPORT_COLUMNS {
        return Err(import_file_error(
            "row 1: header row is empty or has too many columns",
        ));
    }
    let mut headers = HashMap::new();
    for (index, raw) in raw_headers.iter().enumerate() {
        let name = raw
            .trim()
            .trim_start_matches('\u{feff}')
            .trim()
            .to_ascii_lowercase();
        if !is_import_header(&name) {
            return Err(import_file_error(format!(
                "row 1: unknown or empty column header: {name}"
            )));
        }
        if headers.insert(name.clone(), index).is_some() {
            return Err(import_file_error(format!(
                "row 1: duplicate column header: {name}"
            )));
        }
    }
    for name in [
        "chapter_id",
        "difficulty",
        "vignette",
        "lead_in",
        "option_1",
        "rationale_1",
        "option_2",
        "rationale_2",
        "correct_option",
        "key_learning_point",
        "source_ref",
        "rights_ref",
    ] {
        if !headers.contains_key(name) {
            return Err(import_file_error(format!(
                "row 1: required column is missing: {name}"
            )));
        }
    }

    let header_count = raw_headers.len();
    let mut parsed = ParsedImport::default();
    for (row_number, values) in records {
        if values.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        parsed.total_rows += 1;
        if parsed.total_rows > MAX_IMPORT_ROWS {
            return Err(ApiError::unprocessable(
                "invalid_row_count",
                format!("import accepts 1-{MAX_IMPORT_ROWS} rows"),
            ));
        }
        if values.len() > header_count {
            parsed.issues.push(RowIssue {
                row: row_number,
                code: "too_many_cells",
                message: "row has values beyond the last column header".into(),
            });
            continue;
        }
        match import_question(&headers, &values, row_number) {
            Ok(row) => {
                parsed.rows.push(row);
                parsed.row_numbers.push(row_number);
            }
            Err(error) => parsed.issues.push(RowIssue {
                row: row_number,
                code: error.code,
                message: error.message,
            }),
        }
    }
    Ok(parsed)
}

fn parse_csv_import(bytes: &[u8]) -> ApiResult<ParsedImport> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| import_file_error("CSV must use UTF-8 encoding"))?;
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|error| {
            let row = error
                .position()
                .map(|position| position.record() as usize + 1)
                .unwrap_or(1);
            import_file_error(format!("row {row}: invalid CSV header: {error}"))
        })?
        .iter()
        .map(|value| value.to_string())
        .collect();
    let mut records = Vec::new();
    for (index, result) in reader.records().enumerate() {
        let record = result.map_err(|error| {
            let row = error
                .position()
                .map(|position| position.record() as usize + 1)
                .unwrap_or(index + 2);
            import_file_error(format!("row {row}: invalid CSV: {error}"))
        })?;
        let row_number = record
            .position()
            .map(|position| position.record() as usize + 1)
            .unwrap_or(index + 2);
        records.push((
            row_number,
            record.iter().map(|value| value.to_string()).collect(),
        ));
        if records.len() > MAX_IMPORT_ROWS {
            return Err(ApiError::unprocessable(
                "invalid_row_count",
                format!("import accepts 1-{MAX_IMPORT_ROWS} rows"),
            ));
        }
    }
    parse_import_table(headers, records)
}

fn parse_xlsx_import(bytes: &[u8]) -> ApiResult<ParsedImport> {
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err(import_file_error("file is not a valid .xlsx package"));
    }
    let mut workbook = calamine::open_workbook_auto_from_rs(Cursor::new(bytes))
        .map_err(|_| import_file_error("file is not a readable .xlsx workbook"))?;
    let sheet = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| import_file_error("workbook has no worksheets"))?;
    let range = workbook
        .worksheet_range(&sheet)
        .map_err(|_| import_file_error("first worksheet could not be read"))?;
    let (height, width) = range.get_size();
    if height > MAX_IMPORT_ROWS + 1 || width > MAX_IMPORT_COLUMNS {
        return Err(ApiError::unprocessable(
            "invalid_row_count",
            format!(
                "worksheet must have at most {} rows and {} columns",
                MAX_IMPORT_ROWS + 1,
                MAX_IMPORT_COLUMNS
            ),
        ));
    }
    let formulas = workbook
        .worksheet_formula(&sheet)
        .map_err(|_| import_file_error("worksheet formulas could not be read"))?;
    if let Some(row) = formulas.rows().enumerate().find_map(|(index, formulas)| {
        formulas
            .iter()
            .any(|formula| !formula.trim().is_empty())
            .then_some(index + 1)
    }) {
        return Err(import_file_error(format!(
            "row {row}: worksheet formulas are not supported; replace them with values"
        )));
    }
    let mut rows = range.rows();
    let headers = rows
        .next()
        .ok_or_else(|| import_file_error("first worksheet is empty"))?
        .iter()
        .map(ToString::to_string)
        .collect();
    let records = rows
        .enumerate()
        .map(|(index, row)| {
            (
                index + 2,
                row.iter().map(ToString::to_string).collect::<Vec<_>>(),
            )
        })
        .collect();
    parse_import_table(headers, records)
}

/// Validate every row up front; report ALL issues, never just the first.
async fn validate_rows(
    conn: &mut sqlx::postgres::PgConnection,
    exam_id: Uuid,
    rows: &[CreateQuestionReq],
    row_numbers: &[usize],
) -> ApiResult<Vec<RowIssue>> {
    let mut issues = Vec::new();
    let rights_refs: BTreeSet<String> = rows
        .iter()
        .filter_map(|row| row.rights_ref.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_uppercase)
        .collect();
    let mut active_rights = HashMap::new();
    for rights_ref in rights_refs {
        if let Some(rights) = sqlx::query(
            "SELECT permitted_uses, asset_refs FROM content_rights
             WHERE ref_code = $1 AND revoked_at IS NULL
               AND valid_from <= CURRENT_DATE
               AND (valid_to IS NULL OR valid_to >= CURRENT_DATE)
             FOR SHARE",
        )
        .bind(&rights_ref)
        .fetch_optional(&mut *conn)
        .await?
        {
            active_rights.insert(
                rights_ref,
                (
                    rights.try_get::<serde_json::Value, _>("permitted_uses")?,
                    rights.try_get::<serde_json::Value, _>("asset_refs")?,
                ),
            );
        }
    }
    for (i, row) in rows.iter().enumerate() {
        let row_number = row_numbers.get(i).copied().unwrap_or(i + 1);
        let valid_chapter = sqlx::query!(
            "SELECT 1 AS one FROM curriculum_nodes
             WHERE id = $1 AND exam_id = $2 AND kind = 'chapter' AND status = 'active'",
            row.chapter_id,
            exam_id
        )
        .fetch_optional(&mut *conn)
        .await?
        .is_some();
        if !valid_chapter {
            issues.push(RowIssue {
                row: row_number,
                code: "unknown_chapter",
                message: "chapter_id is not an active chapter of this exam".into(),
            });
        }
        if let Err(error) = validate_question(row) {
            issues.push(RowIssue {
                row: row_number,
                code: error.code,
                message: error.message,
            });
        }
        if option_count(row.options.len()).is_err() {
            issues.push(RowIssue {
                row: row_number,
                code: "invalid_option_count",
                message: "questions need 2-10 options (QB-11)".into(),
            });
        }
        if row.correct_index < 0 || row.correct_index as usize >= row.options.len() {
            issues.push(RowIssue {
                row: row_number,
                code: "invalid_correct_index",
                message: "correct option must select one populated option".into(),
            });
        }

        let Some(rights_ref) = row
            .rights_ref
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            issues.push(RowIssue {
                row: row_number,
                code: "rights_ref_required",
                message: "each imported question must name a content-rights record".into(),
            });
            continue;
        };
        let Some((permitted_uses, asset_refs)) =
            active_rights.get(&rights_ref.to_ascii_uppercase())
        else {
            issues.push(RowIssue {
                row: row_number,
                code: "rights_unavailable",
                message: "rights_ref is missing, revoked, or outside its validity dates".into(),
            });
            continue;
        };
        let grants_use = |required: &str| {
            permitted_uses
                .as_array()
                .is_some_and(|uses| uses.iter().any(|use_| use_.as_str() == Some(required)))
        };
        if !grants_use("display") || !grants_use("derivatives") {
            issues.push(RowIssue {
                row: row_number,
                code: "rights_use_not_permitted",
                message: "the rights record must permit both display and derivatives".into(),
            });
            continue;
        }
        let covers_asset = |required: &str| {
            asset_refs.as_array().is_some_and(|assets| {
                assets
                    .iter()
                    .any(|asset| asset.as_str() == Some(required.trim()))
            })
        };
        let source_refs = std::iter::once(row.source_ref.as_str())
            .chain(row.source_refs.iter().map(String::as_str));
        if source_refs.into_iter().any(|asset| !covers_asset(asset))
            || row.media_refs.iter().any(|asset| !covers_asset(asset))
        {
            issues.push(RowIssue {
                row: row_number,
                code: "rights_asset_scope_incomplete",
                message: "the rights record must cover the question source and every source/media reference".into(),
            });
        }
    }
    Ok(issues)
}

async fn import_rows(
    state: Arc<AppState>,
    user_id: Uuid,
    exam_id: Uuid,
    dry_run: bool,
    mut parsed: ParsedImport,
) -> ApiResult<Json<serde_json::Value>> {
    if parsed.total_rows == 0 || parsed.total_rows > MAX_IMPORT_ROWS {
        return Err(ApiError::unprocessable(
            "invalid_row_count",
            format!("import accepts 1-{MAX_IMPORT_ROWS} rows"),
        ));
    }
    // Keep rights row locks through the batch write so revocation cannot race
    // validation and leave a newly imported draft tied to an inactive grant.
    let mut tx = state.pool.begin().await?;
    parsed
        .issues
        .extend(validate_rows(&mut tx, exam_id, &parsed.rows, &parsed.row_numbers).await?);
    let invalid_rows: HashSet<usize> = parsed.issues.iter().map(|issue| issue.row).collect();
    let valid_count = parsed.total_rows.saturating_sub(invalid_rows.len());
    let batch_id = Uuid::new_v4();

    if dry_run || !parsed.issues.is_empty() {
        let status = if dry_run { "dry_run" } else { "rejected" };
        sqlx::query!(
            "INSERT INTO import_batches (id, created_by, exam_id, status, summary)
             VALUES ($1, $2, $3, $4, $5)",
            batch_id,
            user_id,
            exam_id,
            status,
            json!({"rows": parsed.total_rows, "valid": valid_count, "issues": parsed.issues})
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(Json(json!({
            "batch_id": batch_id,
            "status": status,
            "rows": parsed.total_rows,
            "valid": valid_count,
            "issues": parsed.issues,
        })));
    }

    let mut created: Vec<serde_json::Value> = Vec::new();
    for row in &parsed.rows {
        let (qid, vid) = insert_question_version(&mut tx, row, "draft", user_id).await?;
        created.push(json!({"question_id": qid, "version_id": vid}));
    }
    sqlx::query!(
        "INSERT INTO import_batches (id, created_by, exam_id, status, summary)
         VALUES ($1, $2, $3, 'applied', $4)",
        batch_id,
        user_id,
        exam_id,
        json!({"rows": parsed.total_rows, "created": created})
    )
    .execute(&mut *tx)
    .await?;
    audit(
        &mut *tx,
        user_id,
        "import_applied",
        "import_batch",
        batch_id,
        json!({"rows": parsed.total_rows}),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(json!({
        "batch_id": batch_id,
        "status": "applied",
        "rows": parsed.total_rows,
        "created": created,
    })))
}

pub async fn import(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<ImportReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let ImportReq {
        exam_id,
        dry_run,
        rows,
        ..
    } = req;
    let total_rows = rows.len();
    let row_numbers = (1..=total_rows).collect();
    let parsed = ParsedImport {
        rows,
        row_numbers,
        total_rows,
        ..ParsedImport::default()
    };
    import_rows(state, user.user_id, exam_id, dry_run, parsed).await
}

pub async fn import_file(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Query(query): Query<ImportFileQuery>,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim();
    let parsed = match content_type {
        "text/csv" => parse_csv_import(&body)?,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            parse_xlsx_import(&body)?
        }
        _ => {
            return Err(import_file_error(
                "use a UTF-8 .csv or .xlsx question template",
            ))
        }
    };
    import_rows(state, user.user_id, query.exam_id, query.dry_run, parsed).await
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
    let rows = sqlx::query(
        "SELECT actor, action, entity, entity_id, old_value, new_value, created_at
         FROM audit_events ORDER BY created_at DESC LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await?;
    let events: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "actor": r.get::<Option<Uuid>, _>("actor"),
                "action": r.get::<String, _>("action"),
                "entity": r.get::<String, _>("entity"),
                "entity_id": r.get::<Option<Uuid>, _>("entity_id"),
                "old_value": r.get::<Option<serde_json::Value>, _>("old_value"),
                "new_value": r.get::<Option<serde_json::Value>, _>("new_value"),
                "at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
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
#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "admin/AdminDashboard.ts",
        rename = "AdminDashboard"
    )
)]
pub struct AdminDashboard {
    institutions: i64,
    users: i64,
    published_questions: i64,
    articles: i64,
    rights_records: i64,
    open_incidents: i64,
    coach_turns_last_30_days: i64,
}

pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<AdminDashboard>> {
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
    Ok(Json(AdminDashboard {
        institutions: counts.institutions,
        users: counts.users,
        published_questions: counts.published_questions,
        articles: counts.articles,
        rights_records: counts.rights_records,
        open_incidents: counts.open_incidents,
        coach_turns_last_30_days: counts.coach_turns_30d,
    }))
}

// ---- ADMIN-02/TRUST-07: content rights ledger (§19.2) ------------------------

#[derive(Deserialize)]
pub struct ContentRightsReq {
    pub ref_code: String,
    pub licensor: String,
    pub territory: Option<String>,
    /// Content use flags, including private_import and document_extraction.
    pub permitted_uses: Vec<String>,
    pub valid_from: chrono::NaiveDate,
    pub valid_to: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
    pub contract_ref: Option<String>,
    pub contract_version: Option<String>,
    #[serde(default)]
    pub asset_refs: Vec<String>,
    #[serde(default)]
    pub audiences: Vec<String>,
    pub seat_limit: Option<i32>,
    pub offline_terms: Option<String>,
    pub quotation_limit_words: Option<i32>,
    pub ai_terms: Option<String>,
    pub derivative_terms: Option<String>,
    pub attribution: Option<String>,
    pub royalty_terms: Option<String>,
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
    let known = [
        "display",
        "search",
        "offline",
        "embeddings",
        "ai",
        "derivatives",
        "translation",
        "private_import",
        "document_extraction",
    ];
    if req.permitted_uses.is_empty()
        || !req
            .permitted_uses
            .iter()
            .all(|u| known.contains(&u.as_str()))
    {
        return Err(ApiError::unprocessable(
            "invalid_permitted_uses",
            "permitted uses must draw from display, search, offline, embeddings, ai, derivatives, translation, private_import, document_extraction",
        ));
    }
    let mut uses = std::collections::HashSet::new();
    if req.permitted_uses.iter().any(|use_| !uses.insert(use_)) {
        return Err(ApiError::unprocessable(
            "duplicate_permitted_use",
            "each permitted use may appear only once",
        ));
    }
    let valid_list = |values: &[String]| {
        values.len() <= 100
            && values.iter().all(|value| {
                !value.trim().is_empty()
                    && value.chars().count() <= 200
                    && !value.chars().any(char::is_control)
            })
    };
    if !valid_list(&req.asset_refs) || !valid_list(&req.audiences) {
        return Err(ApiError::unprocessable(
            "invalid_rights_scope",
            "assets and audiences must each contain at most 100 non-empty values of at most 200 characters",
        ));
    }
    if req.seat_limit.is_some_and(|limit| limit <= 0)
        || req
            .quotation_limit_words
            .is_some_and(|limit| !(0..=1_000_000).contains(&limit))
        || [
            req.contract_ref.as_deref(),
            req.contract_version.as_deref(),
            req.offline_terms.as_deref(),
            req.ai_terms.as_deref(),
            req.derivative_terms.as_deref(),
            req.attribution.as_deref(),
            req.royalty_terms.as_deref(),
            req.notes.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|value| value.chars().count() > 2000)
    {
        return Err(ApiError::unprocessable(
            "invalid_rights_scope",
            "seat limits must be positive, quotation limits 0-1000000 words, and scope terms at most 2000 characters",
        ));
    }
    if req
        .valid_to
        .is_some_and(|valid_to| valid_to < req.valid_from)
    {
        return Err(ApiError::unprocessable(
            "invalid_rights_window",
            "valid_to must be on or after valid_from",
        ));
    }
    let asset_refs: Vec<String> = req
        .asset_refs
        .iter()
        .map(|value| value.trim().to_string())
        .collect();
    let audiences: Vec<String> = req
        .audiences
        .iter()
        .map(|value| value.trim().to_string())
        .collect();
    let id = Uuid::new_v4();
    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "INSERT INTO content_rights
           (id, ref_code, licensor, territory, permitted_uses, valid_from, valid_to, notes, created_by,
            contract_ref, contract_version, asset_refs, audiences, seat_limit, offline_terms,
            quotation_limit_words, ai_terms, derivative_terms, attribution, royalty_terms)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)",
        id,
        ref_code,
        licensor,
        req.territory.as_deref().unwrap_or("worldwide"),
        serde_json::to_value(&req.permitted_uses).map_err(|_| ApiError::internal())?,
        req.valid_from,
        req.valid_to,
        req.notes.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        user.user_id,
        req.contract_ref.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        req.contract_version.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        serde_json::to_value(&asset_refs).map_err(|_| ApiError::internal())?,
        serde_json::to_value(&audiences).map_err(|_| ApiError::internal())?,
        req.seat_limit,
        req.offline_terms.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        req.quotation_limit_words,
        req.ai_terms.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        req.derivative_terms.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        req.attribution.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        req.royalty_terms.as_deref().map(str::trim).filter(|value| !value.is_empty())
    )
    .execute(&mut *tx)
    .await?;
    audit(
        &mut *tx,
        user.user_id,
        "content_rights_created",
        "content_rights",
        id,
        json!({ "ref_code": ref_code }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "rights_id": id, "ref_code": ref_code })))
}

#[derive(sqlx::FromRow)]
struct ContentRightsRow {
    id: Uuid,
    ref_code: String,
    licensor: String,
    territory: String,
    permitted_uses: serde_json::Value,
    valid_from: chrono::NaiveDate,
    valid_to: Option<chrono::NaiveDate>,
    notes: Option<String>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    revoked_by: Option<Uuid>,
    revocation_note: Option<String>,
    contract_ref: Option<String>,
    contract_version: Option<String>,
    asset_refs: serde_json::Value,
    audiences: serde_json::Value,
    seat_limit: Option<i32>,
    offline_terms: Option<String>,
    quotation_limit_words: Option<i32>,
    ai_terms: Option<String>,
    derivative_terms: Option<String>,
    attribution: Option<String>,
    royalty_terms: Option<String>,
    status: String,
}

pub async fn list_content_rights(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let rows = sqlx::query_as::<_, ContentRightsRow>(
        r#"SELECT id, ref_code, licensor, territory, permitted_uses,
                  valid_from, valid_to, notes, revoked_at, revoked_by, revocation_note,
                  contract_ref, contract_version, asset_refs, audiences, seat_limit,
                  offline_terms, quotation_limit_words, ai_terms, derivative_terms,
                  attribution, royalty_terms,
                  CASE
                    WHEN revoked_at IS NOT NULL THEN 'revoked'
                    WHEN valid_from > CURRENT_DATE THEN 'scheduled'
                    WHEN valid_to IS NOT NULL AND valid_to < CURRENT_DATE THEN 'expired'
                    ELSE 'active'
                  END AS status
           FROM content_rights ORDER BY created_at DESC LIMIT 200"#,
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
        "revoked_at": r.revoked_at,
        "revoked_by": r.revoked_by,
        "revocation_note": r.revocation_note,
        "contract_ref": r.contract_ref,
        "contract_version": r.contract_version,
        "asset_refs": r.asset_refs,
        "audiences": r.audiences,
        "seat_limit": r.seat_limit,
        "offline_terms": r.offline_terms,
        "quotation_limit_words": r.quotation_limit_words,
        "ai_terms": r.ai_terms,
        "derivative_terms": r.derivative_terms,
        "attribution": r.attribution,
        "royalty_terms": r.royalty_terms,
        "status": r.status,
    })).collect::<Vec<_>>() })))
}

#[derive(Deserialize)]
pub struct RevokeContentRightsReq {
    pub reason: String,
}

pub async fn revoke_content_rights(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(rights_id): Path<Uuid>,
    Json(req): Json<RevokeContentRightsReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let reason = req.reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(ApiError::unprocessable(
            "invalid_revocation_reason",
            "reason must be 1-500 characters",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let revoked_id = sqlx::query_scalar::<_, Uuid>(
        "UPDATE content_rights
         SET revoked_at = now(), revoked_by = $2, revocation_note = $3
         WHERE id = $1 AND revoked_at IS NULL
         RETURNING id",
    )
    .bind(rights_id)
    .bind(user.user_id)
    .bind(reason)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(_) = revoked_id else {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM content_rights WHERE id = $1)",
        )
        .bind(rights_id)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(ApiError::not_found("content_rights_not_found"));
        }
        tx.commit().await?;
        return Ok(Json(json!({ "revoked": false, "already_revoked": true })));
    };
    audit(
        &mut *tx,
        user.user_id,
        "rights_revoked",
        "content_rights",
        rights_id,
        json!({ "reason": reason }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "revoked": true, "already_revoked": false })))
}

// ---- LIB-07: extraction coverage evidence ----------------------------------

const EXTRACTION_MEDIA_TYPES: &[&str] = &[
    "application/pdf",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/epub+zip",
    "text/html",
    "image/jpeg",
    "image/png",
    "image/tiff",
    "image/webp",
    "text/plain",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateExtractionReportReq {
    pub source_label: String,
    pub source_sha256: String,
    pub media_type: String,
    pub parser_version: String,
    pub rights_ref: String,
    pub malware_scan_status: String,
    pub expected_regions: Vec<String>,
    pub extracted_regions: Vec<String>,
    pub uncertain_regions: Vec<String>,
    pub critical_regions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewExtractionReportReq {
    pub decision: String,
    pub verified_regions: Vec<String>,
    pub note: String,
}

fn valid_region_refs(values: &[String]) -> bool {
    let mut seen = HashSet::new();
    values.len() <= 5000
        && values.iter().all(|value| {
            !value.is_empty()
                && value.len() <= 128
                && value.bytes().all(|character| {
                    character.is_ascii_alphanumeric()
                        || matches!(character, b'_' | b'-' | b'.' | b':' | b'/')
                })
                && seen.insert(value.as_str())
        })
}

fn region_set(values: &[String]) -> BTreeSet<String> {
    values.iter().cloned().collect()
}

fn string_set(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect()
}

#[derive(sqlx::FromRow)]
struct ExtractionReportRow {
    id: Uuid,
    source_label: String,
    source_sha256: String,
    media_type: String,
    parser_version: String,
    rights_ref: String,
    rights_available: bool,
    malware_scan_status: String,
    expected_regions: serde_json::Value,
    extracted_regions: serde_json::Value,
    uncertain_regions: serde_json::Value,
    critical_regions: serde_json::Value,
    created_by: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
    review_decision: Option<String>,
    reviewer_id: Option<Uuid>,
    verified_regions: Option<serde_json::Value>,
    review_note: Option<String>,
    reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
}

const EXTRACTION_REPORT_SELECT: &str = r#"
    SELECT report.id, report.source_label, report.source_sha256, report.media_type,
           report.parser_version, rights.ref_code AS rights_ref,
           (rights.revoked_at IS NULL
            AND rights.valid_from <= CURRENT_DATE
            AND (rights.valid_to IS NULL OR rights.valid_to >= CURRENT_DATE)
            AND rights.permitted_uses @> '["document_extraction"]'::jsonb) AS rights_available,
           report.malware_scan_status, report.expected_regions, report.extracted_regions,
           report.uncertain_regions, report.critical_regions, report.created_by, report.created_at,
           review.decision AS review_decision, review.reviewer_id,
           review.verified_regions, review.note AS review_note, review.created_at AS reviewed_at
    FROM document_extraction_reports report
    JOIN content_rights rights ON rights.id = report.rights_id
    LEFT JOIN document_extraction_reviews review ON review.report_id = report.id
"#;

fn extraction_report_json(row: ExtractionReportRow) -> serde_json::Value {
    let expected = string_set(&row.expected_regions);
    let extracted = string_set(&row.extracted_regions);
    let missing: Vec<String> = expected.difference(&extracted).cloned().collect();
    let mut needs_review = string_set(&row.uncertain_regions);
    needs_review.extend(string_set(&row.critical_regions));
    let status = if !row.rights_available {
        "rights_unavailable"
    } else if row.malware_scan_status == "blocked" {
        "blocked"
    } else if row.review_decision.as_deref() == Some("rejected") {
        "rejected"
    } else if !missing.is_empty() {
        "incomplete"
    } else if row.malware_scan_status != "clean" {
        "review_required"
    } else if row.review_decision.as_deref() == Some("approved")
        || (needs_review.is_empty() && row.review_decision.is_none())
    {
        "complete"
    } else {
        "review_required"
    };
    json!({
        "report_id": row.id,
        "source_label": row.source_label,
        "source_sha256": row.source_sha256,
        "media_type": row.media_type,
        "parser_version": row.parser_version,
        "rights_ref": row.rights_ref,
        "rights_available": row.rights_available,
        "malware_scan_status": row.malware_scan_status,
        "expected_regions": expected,
        "extracted_regions": extracted,
        "missing_regions": missing,
        "uncertain_regions": string_set(&row.uncertain_regions),
        "critical_regions": string_set(&row.critical_regions),
        "status": status,
        "created_by": row.created_by,
        "created_at": row.created_at,
        "review": row.review_decision.map(|decision| json!({
            "decision": decision,
            "reviewer_id": row.reviewer_id,
            "verified_regions": row.verified_regions.map(|regions| string_set(&regions)),
            "note": row.review_note,
            "reviewed_at": row.reviewed_at,
        })),
    })
}

async fn extraction_report(state: &AppState, report_id: Uuid) -> ApiResult<ExtractionReportRow> {
    let query = format!("{EXTRACTION_REPORT_SELECT} WHERE report.id = $1");
    sqlx::query_as::<_, ExtractionReportRow>(&query)
        .bind(report_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("extraction_report_not_found"))
}

pub async fn list_extraction_reports(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    let query = format!("{EXTRACTION_REPORT_SELECT} ORDER BY report.created_at DESC LIMIT 100");
    let rows = sqlx::query_as::<_, ExtractionReportRow>(&query)
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(
        json!({ "reports": rows.into_iter().map(extraction_report_json).collect::<Vec<_>>() }),
    ))
}

pub async fn get_extraction_report(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(report_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(admin_headers(&headers))?;
    Ok(Json(extraction_report_json(
        extraction_report(&state, report_id).await?,
    )))
}

pub async fn create_extraction_report(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateExtractionReportReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    state.require_admin(admin_headers(&headers))?;
    let source_label = req.source_label.trim();
    let parser_version = req.parser_version.trim();
    let rights_ref = req.rights_ref.trim().to_ascii_uppercase();
    if source_label.is_empty()
        || source_label.chars().count() > 240
        || source_label
            .chars()
            .any(|character| character.is_control() || matches!(character, '/' | '\\' | ':'))
        || parser_version.is_empty()
        || parser_version.chars().count() > 100
        || parser_version.chars().any(char::is_control)
        || req.source_sha256.len() != 64
        || !req
            .source_sha256
            .bytes()
            .all(|character| character.is_ascii_hexdigit())
        || req
            .source_sha256
            .bytes()
            .any(|character| character.is_ascii_uppercase())
        || rights_ref.is_empty()
        || rights_ref.len() > 60
    {
        return Err(ApiError::unprocessable(
            "invalid_extraction_source",
            "use a non-path source label with a lowercase SHA-256, parser version, and rights reference",
        ));
    }
    if !EXTRACTION_MEDIA_TYPES.contains(&req.media_type.as_str()) {
        return Err(ApiError::unprocessable(
            "unsupported_document_type",
            "unsupported extraction report format",
        ));
    }
    if !matches!(
        req.malware_scan_status.as_str(),
        "clean" | "blocked" | "not_scanned"
    ) {
        return Err(ApiError::unprocessable(
            "invalid_scan_status",
            "scan status must be clean, blocked, or not_scanned",
        ));
    }
    let reference_count = req.expected_regions.len()
        + req.extracted_regions.len()
        + req.uncertain_regions.len()
        + req.critical_regions.len();
    if reference_count > 5000
        || !valid_region_refs(&req.expected_regions)
        || !valid_region_refs(&req.extracted_regions)
        || !valid_region_refs(&req.uncertain_regions)
        || !valid_region_refs(&req.critical_regions)
        || req.expected_regions.is_empty()
    {
        return Err(ApiError::unprocessable(
            "invalid_extraction_regions",
            "region lists require unique stable references and a combined maximum of 5000",
        ));
    }
    let expected = region_set(&req.expected_regions);
    let extracted = region_set(&req.extracted_regions);
    let uncertain = region_set(&req.uncertain_regions);
    let critical = region_set(&req.critical_regions);
    if !extracted.is_subset(&expected)
        || !uncertain.is_subset(&extracted)
        || !critical.is_subset(&expected)
    {
        return Err(ApiError::unprocessable("invalid_extraction_regions", "extracted, uncertain, and critical references must be grounded in the expected regions"));
    }

    let mut tx = state.pool.begin().await?;
    let rights_id = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT id FROM content_rights
           WHERE ref_code = $1 AND revoked_at IS NULL
             AND valid_from <= CURRENT_DATE
             AND (valid_to IS NULL OR valid_to >= CURRENT_DATE)
             AND permitted_uses @> '["document_extraction"]'::jsonb
           FOR SHARE"#,
    )
    .bind(&rights_ref)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::forbidden(
            "extraction_rights_unavailable",
            "no active content-rights record permits this extraction",
        )
    })?;
    let report_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO document_extraction_reports
               (id, source_label, source_sha256, media_type, parser_version, rights_id,
                malware_scan_status, expected_regions, extracted_regions, uncertain_regions,
                critical_regions, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
    )
    .bind(report_id)
    .bind(source_label)
    .bind(&req.source_sha256)
    .bind(&req.media_type)
    .bind(parser_version)
    .bind(rights_id)
    .bind(&req.malware_scan_status)
    .bind(json!(req.expected_regions))
    .bind(json!(req.extracted_regions))
    .bind(json!(req.uncertain_regions))
    .bind(json!(req.critical_regions))
    .bind(user.user_id)
    .execute(&mut *tx)
    .await?;
    audit(
        &mut *tx,
        user.user_id,
        "document_extraction_report_created",
        "document_extraction_report",
        report_id,
        json!({
            "source_sha256": req.source_sha256,
            "parser_version": parser_version,
            "expected_regions": expected.len(),
            "missing_regions": expected.difference(&extracted).count(),
            "malware_scan_status": req.malware_scan_status,
        }),
    )
    .await?;
    tx.commit().await?;
    let report = extraction_report(&state, report_id).await?;
    Ok((StatusCode::CREATED, Json(extraction_report_json(report))))
}

pub async fn review_extraction_report(
    State(state): State<Arc<AppState>>,
    reviewer: AuthUser,
    headers: axum::http::HeaderMap,
    Path(report_id): Path<Uuid>,
    Json(req): Json<ReviewExtractionReportReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    state.require_admin(admin_headers(&headers))?;
    if !matches!(req.decision.as_str(), "approved" | "rejected") {
        return Err(ApiError::unprocessable(
            "invalid_review_decision",
            "decision must be approved or rejected",
        ));
    }
    let note = req.note.trim();
    if note.chars().count() < 10
        || note.chars().count() > 2000
        || note.chars().any(char::is_control)
    {
        return Err(ApiError::unprocessable(
            "invalid_review_note",
            "review note must be 10-2000 characters",
        ));
    }
    if !valid_region_refs(&req.verified_regions) {
        return Err(ApiError::unprocessable(
            "invalid_verified_regions",
            "verified region references must be unique and valid",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let query =
        format!("{EXTRACTION_REPORT_SELECT} WHERE report.id = $1 FOR UPDATE OF report, rights");
    let row = sqlx::query_as::<_, ExtractionReportRow>(&query)
        .bind(report_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("extraction_report_not_found"))?;
    if row.created_by == Some(reviewer.user_id) {
        return Err(ApiError::forbidden(
            "extraction_self_review_forbidden",
            "extractors cannot review their own extraction report",
        ));
    }
    if !row.rights_available {
        return Err(ApiError::forbidden(
            "extraction_rights_unavailable",
            "no active content-rights record permits this extraction",
        ));
    }
    if row.review_decision.is_some() {
        return Err(ApiError::conflict(
            "extraction_review_already_recorded",
            "this report already has a review decision",
        ));
    }

    let expected = string_set(&row.expected_regions);
    let extracted = string_set(&row.extracted_regions);
    let missing: BTreeSet<String> = expected.difference(&extracted).cloned().collect();
    let mut required_review = string_set(&row.uncertain_regions);
    required_review.extend(string_set(&row.critical_regions));
    let verified = region_set(&req.verified_regions);
    if req.decision == "approved" {
        if row.malware_scan_status != "clean" {
            return Err(ApiError::conflict(
                "extraction_scan_required",
                "approval requires a clean scan result",
            ));
        }
        if !missing.is_empty() {
            return Err(ApiError::conflict(
                "extraction_incomplete",
                "missing regions must be extracted before approval",
            ));
        }
        if verified != required_review {
            return Err(ApiError::unprocessable(
                "critical_review_required",
                "approval must verify every critical and uncertain region",
            ));
        }
    } else if !verified.is_subset(&expected) {
        return Err(ApiError::unprocessable(
            "invalid_verified_regions",
            "rejected reviews may reference only expected regions",
        ));
    }

    sqlx::query(
        r#"INSERT INTO document_extraction_reviews
               (id, report_id, reviewer_id, decision, verified_regions, note)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(Uuid::new_v4())
    .bind(report_id)
    .bind(reviewer.user_id)
    .bind(&req.decision)
    .bind(json!(verified))
    .bind(note)
    .execute(&mut *tx)
    .await?;
    audit(
        &mut *tx,
        reviewer.user_id,
        "document_extraction_report_reviewed",
        "document_extraction_report",
        report_id,
        json!({ "decision": req.decision, "verified_region_count": verified.len() }),
    )
    .await?;
    tx.commit().await?;
    let report = extraction_report(&state, report_id).await?;
    Ok((StatusCode::CREATED, Json(extraction_report_json(report))))
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
    let daily_allowance_per_learner = crate::routes::settings::current_bounded_i64(
        &state.pool,
        "free_daily_coach_turns",
        state.free_daily_coach_turns,
        0,
        1000,
    )
    .await?;
    Ok(Json(json!({
        "daily_allowance_per_learner": daily_allowance_per_learner,
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
    pub hint: Option<String>,
    pub high_yield: Option<bool>,
    pub source_ref: String,
    pub rights_ref: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source_refs: Option<Vec<String>>,
    pub media_refs: Option<Vec<String>>,
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
    validate_hint_length(req.hint.as_deref())?;
    if req
        .rights_ref
        .as_deref()
        .is_some_and(|value| value.trim().is_empty() || value.trim().len() > 60)
    {
        return Err(ApiError::unprocessable(
            "invalid_rights_ref",
            "rights_ref must be 1-60 characters when supplied",
        ));
    }
    validate_import_metadata(
        req.tags.as_deref().unwrap_or_default(),
        req.source_refs.as_deref().unwrap_or_default(),
        req.media_refs.as_deref().unwrap_or_default(),
    )?;
    let _family = sqlx::query!("SELECT family_id FROM questions WHERE id = $1", question_id)
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
              hint, high_yield, source_ref, created_by, tags, source_refs, media_refs, rights_ref)
            SELECT $1, q.id, $3, 'draft', qv.chapter_id, $4, $5, $6, $7, $8,
                   $9, $10, $11, $12, $13, $14,
                    COALESCE($15, qv.tags), COALESCE($16, qv.source_refs), COALESCE($17, qv.media_refs),
                    COALESCE($18, qv.rights_ref)
            FROM questions q
            JOIN question_versions qv ON qv.question_id = q.id
            WHERE q.id = $2
            ORDER BY qv.version DESC
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
        req.hint,
        req.high_yield.unwrap_or(false),
        req.source_ref,
        user.user_id,
        req.tags.as_deref(),
        req.source_refs.as_deref(),
        req.media_refs.as_deref(),
        req.rights_ref.map(|value| value.trim().to_ascii_uppercase())
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
    // Build the canonical manifest exactly as packs.rs does, then verify it
    // using the public key as the browser does offline.
    let chapter_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM curriculum_nodes
         WHERE exam_id = $1 AND kind = 'chapter' ORDER BY id",
    )
    .bind(req.exam_id)
    .fetch_all(&state.pool)
    .await?;
    let rows = sqlx::query!(
        r#"SELECT id, vignette, lead_in, difficulty, options,
                  correct_index, key_learning_point, exam_tip, source_ref
           FROM question_versions
           WHERE chapter_id = ANY($1) AND status = 'published'
           ORDER BY chapter_id, id"#,
        &chapter_ids
    )
    .fetch_all(&state.pool)
    .await?;
    let question_ids: Vec<_> = rows.iter().map(|row| row.id).collect();
    let answered =
        crate::routes::packs::answered_tutor_question_ids(&state.pool, user.user_id, &question_ids)
            .await?;
    let answered_ids: Vec<_> = answered.iter().copied().collect();
    let mut tutoring_cards =
        crate::routes::packs::tutoring_cards_for_questions(&state.pool, &answered_ids).await?;
    let mut items = Vec::with_capacity(rows.len());
    for r in &rows {
        let resource = crate::routes::packs::build_pack_resource(
            crate::routes::packs::PackQuestionData {
                question_version_id: r.id,
                vignette: r.vignette.clone(),
                lead_in: r.lead_in.clone(),
                difficulty: r.difficulty.clone(),
                options: r.options.clone(),
                correct_index: r.correct_index,
                key_learning_point: r.key_learning_point.clone(),
                exam_tip: r.exam_tip.clone(),
                source_ref: r.source_ref.clone(),
                tutoring_cards: tutoring_cards.remove(&r.id).unwrap_or_default(),
            },
        )?;
        let checksum = resource.checksum.clone();
        items.push((r.id, checksum));
    }
    let canonical = crate::routes::packs::manifest_canonical(
        req.exam_id,
        "recovery-drill",
        &chapter_ids,
        &items,
    )?;
    let signer = crate::routes::packs::ed25519_signing_key(&state)?;
    let signature = signer.sign(canonical.as_bytes());
    let verify_ok = signer
        .verifying_key()
        .verify(canonical.as_bytes(), &signature)
        .is_ok();
    let tamper = format!("{canonical}TAMPERED");
    let tamper_detected = signer
        .verifying_key()
        .verify(tamper.as_bytes(), &signature)
        .is_err();
    let pass = verify_ok && tamper_detected && !rows.is_empty();
    let evidence = json!({
        "items": rows.len(),
        "signature_verified": verify_ok,
        "tamper_detected": tamper_detected,
        "algorithm": "ed25519",
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
