//! CORE-10: stable knowledge identities separate from exam navigation.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

fn require_admin(state: &AppState, user: &AuthUser, headers: &HeaderMap) -> ApiResult<()> {
    state.require_admin(
        user,
        headers.get("x-admin-token").and_then(|v| v.to_str().ok()),
    )
}

fn normalized_key(value: &str) -> ApiResult<String> {
    let key = value.trim().to_ascii_lowercase();
    let valid = !key.is_empty()
        && key.len() <= 100
        && !key.starts_with('-')
        && !key.ends_with('-')
        && !key.contains("--")
        && key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !valid {
        return Err(ApiError::unprocessable(
            "invalid_concept_key",
            "canonical_key must be a lowercase slug of up to 100 characters",
        ));
    }
    Ok(key)
}

fn required_text(value: &str, field: &'static str, max: usize) -> ApiResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max {
        return Err(ApiError::unprocessable(
            "invalid_concept_text",
            format!("{field} must contain 1 to {max} characters"),
        ));
    }
    Ok(value.to_string())
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "concepts/CreateConceptRequest.ts",
        rename = "CreateConceptRequest"
    )
)]
pub struct CreateConceptReq {
    pub canonical_key: String,
    pub display_name: String,
    pub definition: String,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "concepts/CreateConceptVersionRequest.ts",
        rename = "CreateConceptVersionRequest"
    )
)]
pub struct NewVersionReq {
    pub display_name: String,
    pub definition: String,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "concepts/SetNodeConceptsRequest.ts",
        rename = "SetNodeConceptsRequest"
    )
)]
pub struct SetNodeConceptsReq {
    pub concept_ids: Vec<Uuid>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "concepts/AdminConcept.ts",
        rename = "AdminConcept"
    )
)]
pub struct AdminConcept {
    pub concept_id: Uuid,
    pub canonical_key: String,
    pub current_version: i32,
    pub display_name: String,
    pub definition: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "concepts/AdminConceptListResponse.ts",
        rename = "AdminConceptListResponse"
    )
)]
pub struct AdminConceptListResponse {
    pub concepts: Vec<AdminConcept>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "concepts/CreateConceptResponse.ts",
        rename = "CreateConceptResponse"
    )
)]
pub struct CreateConceptResponse {
    pub concept_id: Uuid,
    pub canonical_key: String,
    pub current_version: i32,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "concepts/CreateConceptVersionResponse.ts",
        rename = "CreateConceptVersionResponse"
    )
)]
pub struct CreateConceptVersionResponse {
    pub concept_id: Uuid,
    pub current_version: i32,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "concepts/NodeConcept.ts", rename = "NodeConcept")
)]
pub struct NodeConcept {
    pub concept_id: Uuid,
    pub canonical_key: String,
    pub version: i32,
    pub display_name: String,
    pub definition: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "concepts/NodeConceptsResponse.ts",
        rename = "NodeConceptsResponse"
    )
)]
pub struct NodeConceptsResponse {
    pub concepts: Vec<NodeConcept>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "concepts/SetNodeConceptsResponse.ts",
        rename = "SetNodeConceptsResponse"
    )
)]
pub struct SetNodeConceptsResponse {
    pub node_id: Uuid,
    pub mapped: usize,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<AdminConceptListResponse>> {
    require_admin(&state, &user, &headers)?;
    let rows = sqlx::query!(
        r#"SELECT c.id, c.canonical_key, c.current_version,
                  v.display_name, v.definition
           FROM concepts c
           JOIN concept_versions v
             ON v.concept_id = c.id AND v.version = c.current_version
           ORDER BY c.canonical_key"#
    )
    .fetch_all(&state.pool)
    .await?;
    let concepts: Vec<AdminConcept> = rows
        .into_iter()
        .map(|row| AdminConcept {
            concept_id: row.id,
            canonical_key: row.canonical_key,
            current_version: row.current_version,
            display_name: row.display_name,
            definition: row.definition,
        })
        .collect();
    Ok(Json(AdminConceptListResponse { concepts }))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Json(req): Json<CreateConceptReq>,
) -> ApiResult<Json<CreateConceptResponse>> {
    require_admin(&state, &user, &headers)?;
    let key = normalized_key(&req.canonical_key)?;
    let name = required_text(&req.display_name, "display_name", 200)?;
    let definition = required_text(&req.definition, "definition", 4000)?;
    let id = Uuid::new_v4();
    let mut tx = state.pool.begin().await?;
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO concepts (id, canonical_key, current_version, created_by)
         VALUES ($1, $2, 1, $3)
         ON CONFLICT (canonical_key) DO NOTHING
         RETURNING id",
    )
    .bind(id)
    .bind(&key)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?;
    if inserted.is_none() {
        return Err(ApiError::conflict(
            "concept_key_exists",
            "a concept already uses this canonical key",
        ));
    }
    sqlx::query!(
        "INSERT INTO concept_versions
         (concept_id, version, display_name, definition, created_by)
         VALUES ($1, 1, $2, $3, $4)",
        id,
        name,
        definition,
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "concept_identity_created",
        "concept_identity",
        id,
        json!({ "canonical_key": key, "version": 1 }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(CreateConceptResponse {
        concept_id: id,
        canonical_key: key,
        current_version: 1,
    }))
}

pub async fn create_version(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(concept_id): Path<Uuid>,
    Json(req): Json<NewVersionReq>,
) -> ApiResult<Json<CreateConceptVersionResponse>> {
    require_admin(&state, &user, &headers)?;
    let name = required_text(&req.display_name, "display_name", 200)?;
    let definition = required_text(&req.definition, "definition", 4000)?;
    let mut tx = state.pool.begin().await?;
    let current = sqlx::query_scalar::<_, i32>(
        "SELECT current_version FROM concepts WHERE id = $1 FOR UPDATE",
    )
    .bind(concept_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("concept_not_found"))?;
    let version = current.checked_add(1).ok_or_else(ApiError::internal)?;
    sqlx::query!(
        "INSERT INTO concept_versions
         (concept_id, version, display_name, definition, created_by)
         VALUES ($1, $2, $3, $4, $5)",
        concept_id,
        version,
        name,
        definition,
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE concepts SET current_version = $2 WHERE id = $1",
        concept_id,
        version
    )
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "concept_version_created",
        "concept_identity",
        concept_id,
        json!({ "version": version }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(CreateConceptVersionResponse {
        concept_id,
        current_version: version,
    }))
}

pub async fn node_mappings(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(node_id): Path<Uuid>,
) -> ApiResult<Json<NodeConceptsResponse>> {
    require_admin(&state, &user, &headers)?;
    let node = sqlx::query_scalar::<_, Uuid>("SELECT id FROM curriculum_nodes WHERE id = $1")
        .bind(node_id)
        .fetch_optional(&state.pool)
        .await?;
    if node.is_none() {
        return Err(ApiError::not_found("hierarchy_node_not_found"));
    }
    let rows = sqlx::query!(
        r#"SELECT c.id AS concept_id, c.canonical_key, c.current_version AS version,
                  v.display_name, v.definition
           FROM curriculum_node_concepts m
           JOIN concepts c ON c.id = m.concept_id
           JOIN concept_versions v
             ON v.concept_id = c.id AND v.version = c.current_version
           WHERE m.curriculum_node_id = $1
           ORDER BY c.canonical_key"#,
        node_id
    )
    .fetch_all(&state.pool)
    .await?;
    let concepts: Vec<NodeConcept> = rows
        .into_iter()
        .map(|row| NodeConcept {
            concept_id: row.concept_id,
            canonical_key: row.canonical_key,
            version: row.version,
            display_name: row.display_name,
            definition: row.definition,
        })
        .collect();
    Ok(Json(NodeConceptsResponse { concepts }))
}

pub async fn set_node_mappings(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(node_id): Path<Uuid>,
    Json(req): Json<SetNodeConceptsReq>,
) -> ApiResult<Json<SetNodeConceptsResponse>> {
    require_admin(&state, &user, &headers)?;
    let mut ids = req.concept_ids;
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ApiError::unprocessable(
            "duplicate_concept_mapping",
            "each concept may be mapped to a node once",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let node =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM curriculum_nodes WHERE id = $1 FOR UPDATE")
            .bind(node_id)
            .fetch_optional(&mut *tx)
            .await?;
    if node.is_none() {
        return Err(ApiError::not_found("hierarchy_node_not_found"));
    }
    let found: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concepts WHERE id = ANY($1)")
        .bind(&ids)
        .fetch_one(&mut *tx)
        .await?;
    if found != ids.len() as i64 {
        return Err(ApiError::unprocessable(
            "unknown_concept",
            "every mapped concept must exist",
        ));
    }
    sqlx::query!(
        "DELETE FROM curriculum_node_concepts WHERE curriculum_node_id = $1",
        node_id
    )
    .execute(&mut *tx)
    .await?;
    for concept_id in &ids {
        sqlx::query!(
            "INSERT INTO curriculum_node_concepts
             (curriculum_node_id, concept_id, created_by)
             VALUES ($1, $2, $3)",
            node_id,
            concept_id,
            user.user_id
        )
        .execute(&mut *tx)
        .await?;
    }
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "curriculum_concepts_mapped",
        "curriculum_node",
        node_id,
        json!({ "concept_ids": ids }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(SetNodeConceptsResponse {
        node_id,
        mapped: ids.len(),
    }))
}
