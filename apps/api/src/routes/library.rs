//! LIB-01/02/03: versioned library articles with search and citation
//! anchors. Published versions only for learners; the editorial console's
//! authoring surface arrives with ADMIN-06 iteration 2. Jurisdiction/date
//! overlays (LIB-04) and hybrid semantic search (LIB-02 full) follow in the
//! Phase 2 library slice — this is the versioned + text-searched base.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgConnection;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const PRIVATE_IMPORT_MAX_BYTES: usize = 1024 * 1024;
const PRIVATE_IMPORT_MAX_DOCUMENTS: i64 = 25;
const PRIVATE_IMPORT_MAX_TOTAL_BYTES: i64 = 10 * 1024 * 1024;

// ---- CORE-03: the free tier's library retrievals are metered ----------------
// Search queries and article opens both consume one retrieval from the daily
// free allowance; the honest 403 carries the counters (§26.1 — upgrade
// prompts originate from entitlement checks, never from the Coach).

async fn require_library_allowance(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    let tier: String = sqlx::query_scalar!("SELECT tier FROM users WHERE id = $1", user_id)
        .fetch_one(&state.pool)
        .await?;
    if tier != "free" {
        return Ok(());
    }
    let day = chrono::Utc::now().date_naive();
    let used = sqlx::query!(
        r#"SELECT count AS "count!" FROM entitlement_usage
           WHERE user_id = $1 AND key = 'library_retrieval' AND day = $2"#,
        user_id,
        day
    )
    .fetch_optional(&state.pool)
    .await?
    .map(|r| r.count)
    .unwrap_or(0);
    if i64::from(used) >= state.free_daily_library {
        return Err(ApiError::forbidden_with_details(
            "library_allowance_reached",
            format!(
                "Daily free library allowance of {} retrievals reached — it resets tomorrow.",
                state.free_daily_library
            ),
            serde_json::json!({
                "allowance": {
                    "limit": state.free_daily_library,
                    "used": used,
                    "remaining": state.free_daily_library.saturating_sub(i64::from(used)).max(0),
                }
            }),
        ));
    }
    sqlx::query!(
        r#"INSERT INTO entitlement_usage (user_id, key, day, count)
           VALUES ($1, 'library_retrieval', $2, 1)
           ON CONFLICT (user_id, key, day) DO UPDATE
             SET count = entitlement_usage.count + 1"#,
        user_id,
        day
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    // LIB-02 hybrid ranking: the whole-phrase match anchors the query, then
    // per-token hits add weighted signal (title hits outweigh body hits).
    // Pure lexical by design — no vector index is claimed or faked.
    let query = q
        .get("q")
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();
    if query.is_empty() {
        return Ok(Json(json!({ "results": [], "private_documents": [] })));
    }
    require_library_allowance(&state, user.user_id).await?;
    let tokens: Vec<String> = query
        .split_whitespace()
        .filter(|t| t.len() >= 2)
        .map(String::from)
        .collect();
    let patterns: Vec<String> = tokens
        .iter()
        .map(|t| format!("%{t}%"))
        .chain(std::iter::once(format!("%{query}%")))
        .collect();
    // Only the latest published version of each article is searchable.
    let rows = sqlx::query!(
        r#"SELECT a.id, a.slug, a.title, av.version, av.body, av.source_ref
           FROM articles a
           JOIN article_versions av ON av.article_id = a.id AND av.status = 'published'
           WHERE (a.title ILIKE ANY($1) OR av.body ILIKE ANY($1))
             AND av.version = (
                 SELECT MAX(version) FROM article_versions
                 WHERE article_id = a.id AND status = 'published')
           LIMIT 100"#,
        &patterns
    )
    .fetch_all(&state.pool)
    .await?;
    let mut scored: Vec<(i64, serde_json::Value)> = rows
        .into_iter()
        .map(|r| {
            let title_lower = r.title.to_lowercase();
            let body_lower = r.body.to_lowercase();
            let mut score: i64 = 0;
            if body_lower.contains(&query) {
                score += 5;
            }
            for t in &tokens {
                if title_lower.contains(t) {
                    score += 10;
                }
                if body_lower.contains(t) {
                    score += 2;
                }
            }
            let article = json!({
                "content_type": "editorial_article",
                "article_id": r.id,
                "slug": r.slug,
                "title": r.title,
                "version": r.version,
                "score": score,
                "source_ref": r.source_ref,
                // A body excerpt keeps the list honest about what matched.
                "excerpt": r.body.chars().take(200).collect::<String>(),
            });
            (score, article)
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0).then_with(|| {
            a.1["title"]
                .as_str()
                .unwrap_or("")
                .cmp(b.1["title"].as_str().unwrap_or(""))
        })
    });
    let results: Vec<serde_json::Value> = scored
        .into_iter()
        .take(25)
        .map(|(_, article)| article)
        .collect();
    #[derive(sqlx::FromRow)]
    struct PrivateDocumentSearchHit {
        document_id: Uuid,
        title: String,
        media_type: String,
        rights_ref: String,
        sha256: String,
        created_at: chrono::DateTime<chrono::Utc>,
        content: String,
    }
    let private_documents = sqlx::query_as::<_, PrivateDocumentSearchHit>(
        r#"SELECT d.id AS document_id, d.title, d.media_type, r.ref_code AS rights_ref,
                  d.sha256, d.created_at, d.content
           FROM private_documents d
           JOIN content_rights r ON r.id = d.rights_id
           WHERE d.user_id = $1
             AND r.revoked_at IS NULL
             AND r.valid_from <= CURRENT_DATE
             AND (r.valid_to IS NULL OR r.valid_to >= CURRENT_DATE)
             AND r.permitted_uses @> '["private_import", "display", "search"]'::jsonb
             AND (d.title ILIKE ANY($2) OR d.content ILIKE ANY($2))
           ORDER BY d.created_at DESC
           LIMIT 25
           FOR SHARE OF r"#,
    )
    .bind(user.user_id)
    .bind(&patterns)
    .fetch_all(&state.pool)
    .await?;
    let private_documents: Vec<serde_json::Value> = private_documents
        .into_iter()
        .map(|document| {
            json!({
                "document_id": document.document_id,
                "content_type": "private_document",
                "title": document.title,
                "media_type": document.media_type,
                "rights_ref": document.rights_ref,
                "sha256": document.sha256,
                "created_at": document.created_at,
                "available": true,
                "excerpt": document.content.chars().take(200).collect::<String>(),
            })
        })
        .collect();
    Ok(Json(
        json!({ "results": results, "private_documents": private_documents }),
    ))
}

pub async fn get_article(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(slug): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let article = sqlx::query!(
        r#"SELECT a.id, a.slug, a.title, av.id AS "version_id!", av.version, av.body, av.source_ref,
                  av.jurisdiction AS "jurisdiction?", av.effective_from AS "effective_from?",
                  av.effective_to AS "effective_to?"
           FROM articles a
           JOIN article_versions av ON av.article_id = a.id AND av.status = 'published'
           WHERE a.slug = $1
             AND av.version = (
                 SELECT MAX(version) FROM article_versions
                 WHERE article_id = a.id AND status = 'published')"#,
        slug
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("article_not_found"))?;
    require_library_allowance(&state, _user.user_id).await?;
    sqlx::query!(
        "INSERT INTO article_reads (user_id, article_version_id)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
        _user.user_id,
        article.version_id
    )
    .execute(&state.pool)
    .await?;
    let citations = sqlx::query!(
        "SELECT anchor, target, kind FROM article_citations WHERE version_id = $1",
        article.version_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "slug": article.slug,
        "title": article.title,
        "version": article.version,
        "body": article.body,
        "source_ref": article.source_ref,
        "jurisdiction": article.jurisdiction,
        "effective_from": article.effective_from,
        "effective_to": article.effective_to,
        "citations": citations.iter().map(|c| json!({
            "anchor": c.anchor, "target": c.target, "kind": c.kind
        })).collect::<Vec<_>>(),
        "media": media_list(&state, article.id).await?,
    })))
}

#[derive(sqlx::FromRow)]
struct PrivateImportRight {
    rights_id: Uuid,
    ref_code: String,
    licensor: String,
    valid_to: Option<chrono::NaiveDate>,
    search_allowed: bool,
}

#[derive(sqlx::FromRow)]
struct PrivateDocumentSummary {
    document_id: Uuid,
    title: String,
    media_type: String,
    rights_ref: String,
    sha256: String,
    created_at: chrono::DateTime<chrono::Utc>,
    available: bool,
}

#[derive(sqlx::FromRow)]
struct PrivateImportQuota {
    document_count: i64,
    total_bytes: i64,
}

#[derive(Deserialize)]
pub struct CreatePrivateImportReq {
    pub title: String,
    pub media_type: String,
    pub content: String,
    pub rights_ref: String,
}

pub async fn private_import_rights(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rights = sqlx::query_as::<_, PrivateImportRight>(
        r#"SELECT id AS rights_id, ref_code, licensor, valid_to,
                  permitted_uses @> '["search"]'::jsonb AS search_allowed
           FROM content_rights
           WHERE revoked_at IS NULL
             AND valid_from <= CURRENT_DATE
             AND (valid_to IS NULL OR valid_to >= CURRENT_DATE)
             AND permitted_uses @> '["private_import", "display"]'::jsonb
           ORDER BY ref_code"#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "rights": rights.iter().map(|right| json!({
        "rights_id": right.rights_id,
        "ref_code": right.ref_code,
        "licensor": right.licensor,
        "valid_to": right.valid_to,
        "search_allowed": right.search_allowed,
    })).collect::<Vec<_>>() })))
}

pub async fn list_private_imports(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let documents = sqlx::query_as::<_, PrivateDocumentSummary>(
        r#"SELECT d.id AS document_id, d.title, d.media_type, r.ref_code AS rights_ref,
                  d.sha256, d.created_at,
                  (r.revoked_at IS NULL
                   AND r.valid_from <= CURRENT_DATE
                   AND (r.valid_to IS NULL OR r.valid_to >= CURRENT_DATE)
                   AND r.permitted_uses @> '["private_import", "display"]'::jsonb) AS available
           FROM private_documents d
           JOIN content_rights r ON r.id = d.rights_id
           WHERE d.user_id = $1
           ORDER BY d.created_at DESC"#,
    )
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        json!({ "documents": documents.iter().map(|document| json!({
        "document_id": document.document_id,
        "title": document.title,
        "media_type": document.media_type,
        "rights_ref": document.rights_ref,
        "sha256": document.sha256,
        "created_at": document.created_at,
        "available": document.available,
    })).collect::<Vec<_>>() }),
    ))
}

pub async fn create_private_import(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CreatePrivateImportReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let title = req.title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_document_title",
            "title must be 1-200 characters",
        ));
    }
    let media_type = req.media_type.trim().to_ascii_lowercase();
    if !matches!(media_type.as_str(), "text/plain" | "text/markdown") {
        return Err(ApiError::unprocessable(
            "unsupported_document_type",
            "only text/plain and text/markdown imports are supported",
        ));
    }
    if req.content.trim().is_empty()
        || req
            .content
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(ApiError::unprocessable(
            "invalid_document_content",
            "document text must be non-empty and contain no unsupported control characters",
        ));
    }
    if req.content.len() > PRIVATE_IMPORT_MAX_BYTES {
        return Err(ApiError::unprocessable(
            "document_too_large",
            "document text must be at most 1 MiB",
        ));
    }
    let rights_ref = req.rights_ref.trim().to_ascii_uppercase();
    if rights_ref.is_empty() || rights_ref.len() > 60 {
        return Err(ApiError::unprocessable(
            "invalid_rights_ref",
            "rights_ref must be 1-60 characters",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let rights_id = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT id FROM content_rights
           WHERE ref_code = $1 AND revoked_at IS NULL
             AND valid_from <= CURRENT_DATE
             AND (valid_to IS NULL OR valid_to >= CURRENT_DATE)
             AND permitted_uses @> '["private_import", "display"]'::jsonb
           FOR SHARE"#,
    )
    .bind(&rights_ref)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::forbidden(
            "rights_unavailable",
            "no active content-rights record permits this action",
        )
    })?;

    sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(user.user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("user_not_found"))?;
    let quota = sqlx::query_as::<_, PrivateImportQuota>(
        r#"SELECT COUNT(*) AS document_count,
                  COALESCE(SUM(octet_length(content)), 0)::BIGINT AS total_bytes
           FROM private_documents WHERE user_id = $1"#,
    )
    .bind(user.user_id)
    .fetch_one(&mut *tx)
    .await?;
    let incoming_bytes = i64::try_from(req.content.len()).map_err(|_| ApiError::internal())?;
    if quota.document_count >= PRIVATE_IMPORT_MAX_DOCUMENTS
        || quota.total_bytes.saturating_add(incoming_bytes) > PRIVATE_IMPORT_MAX_TOTAL_BYTES
    {
        return Err(ApiError::conflict(
            "private_import_quota_reached",
            "private imports are limited to 25 documents and 10 MiB per account",
        ));
    }

    let document_id = Uuid::new_v4();
    let sha256 = crate::auth::sha256_hex(&req.content);
    let created_at = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
        "INSERT INTO private_documents
           (id, user_id, rights_id, title, media_type, content, sha256)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING created_at",
    )
    .bind(document_id)
    .bind(user.user_id)
    .bind(rights_id)
    .bind(title)
    .bind(&media_type)
    .bind(&req.content)
    .bind(&sha256)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "document_id": document_id,
            "title": title,
            "media_type": media_type,
            "rights_ref": rights_ref,
            "sha256": sha256,
            "created_at": created_at,
            "available": true,
        })),
    ))
}

pub async fn get_private_import(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    let metadata = sqlx::query_as::<_, PrivateDocumentSummary>(
        r#"SELECT d.id AS document_id, d.title, d.media_type, r.ref_code AS rights_ref,
                  d.sha256, d.created_at,
                  (r.revoked_at IS NULL
                   AND r.valid_from <= CURRENT_DATE
                   AND (r.valid_to IS NULL OR r.valid_to >= CURRENT_DATE)
                   AND r.permitted_uses @> '["private_import", "display"]'::jsonb) AS available
           FROM private_documents d
           JOIN content_rights r ON r.id = d.rights_id
           WHERE d.id = $1 AND d.user_id = $2"#,
    )
    .bind(document_id)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("private_import_not_found"))?;
    if !metadata.available {
        return Err(ApiError::forbidden(
            "rights_unavailable",
            "no active content-rights record permits this action",
        ));
    }
    let content = sqlx::query_scalar::<_, String>(
        r#"SELECT d.content FROM private_documents d
           JOIN content_rights r ON r.id = d.rights_id
           WHERE d.id = $1 AND d.user_id = $2
             AND r.revoked_at IS NULL
             AND r.valid_from <= CURRENT_DATE
             AND (r.valid_to IS NULL OR r.valid_to >= CURRENT_DATE)
             AND r.permitted_uses @> '["private_import", "display"]'::jsonb
           FOR SHARE OF r"#,
    )
    .bind(document_id)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::forbidden(
            "rights_unavailable",
            "no active content-rights record permits this action",
        )
    })?;
    tx.commit().await?;
    Ok(Json(json!({
        "document_id": metadata.document_id,
        "title": metadata.title,
        "media_type": metadata.media_type,
        "rights_ref": metadata.rights_ref,
        "sha256": metadata.sha256,
        "created_at": metadata.created_at,
        "available": metadata.available,
        "content": content,
    })))
}

pub async fn delete_private_import(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let deleted = sqlx::query_scalar::<_, Uuid>(
        "DELETE FROM private_documents WHERE id = $1 AND user_id = $2 RETURNING id",
    )
    .bind(document_id)
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?
    .is_some();
    if !deleted {
        return Err(ApiError::not_found("private_import_not_found"));
    }
    Ok(Json(json!({ "deleted": true })))
}

// ---- LIB-08: media with captions and chapters (rights reference required) ----

pub async fn media_list(state: &AppState, article_id: Uuid) -> ApiResult<serde_json::Value> {
    let rows = sqlx::query!(
        r#"SELECT id, url, kind, duration_seconds AS "duration_seconds?",
                  captions, chapters, rights_ref
           FROM media_assets WHERE article_id = $1 ORDER BY created_at"#,
        article_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(json!(rows
        .iter()
        .map(|r| json!({
            "media_id": r.id,
            "url": r.url,
            "kind": r.kind,
            "duration_seconds": r.duration_seconds,
            "captions": r.captions,
            "chapters": r.chapters,
            "rights_ref": r.rights_ref,
        }))
        .collect::<Vec<_>>()))
}

#[derive(Deserialize)]
pub struct MediaReq {
    pub url: String,
    pub kind: String, // audio | video
    pub duration_seconds: Option<i32>,
    #[serde(default)]
    pub captions: serde_json::Value,
    #[serde(default)]
    pub chapters: serde_json::Value,
    pub rights_ref: Option<String>,
}

pub async fn attach_media(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(article_id): Path<Uuid>,
    Json(req): Json<MediaReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    let url = req.url.trim();
    if url.is_empty() || !url.starts_with("https://") {
        return Err(ApiError::unprocessable(
            "invalid_url",
            "media url must be an https URL",
        ));
    }
    if !matches!(req.kind.as_str(), "audio" | "video") {
        return Err(ApiError::unprocessable(
            "invalid_media_kind",
            "kind must be audio or video",
        ));
    }
    if !req.captions.is_array() || !req.chapters.is_array() {
        return Err(ApiError::unprocessable(
            "invalid_media_lists",
            "captions and chapters must be arrays",
        ));
    }
    let rights_ref = req
        .rights_ref
        .map(|r| r.trim().to_string())
        .unwrap_or_default();
    if rights_ref.is_empty() {
        return Err(ApiError::unprocessable(
            "rights_ref_required",
            "media needs a rights reference (LIB-06)",
        ));
    }
    let exists = sqlx::query!("SELECT 1 AS one FROM articles WHERE id = $1", article_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("article_not_found"))?;
    let _ = exists;
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO media_assets
           (id, article_id, url, kind, duration_seconds, captions, chapters, rights_ref)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        id,
        article_id,
        url,
        req.kind,
        req.duration_seconds,
        req.captions,
        req.chapters,
        rights_ref
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "media_id": id })))
}

// ---- IMG-01/02: rights-checked image cases and stacks ------------------------

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ImageFindingsReq {
    Structured(Vec<ImageFinding>),
    LegacyText(String),
}

#[derive(Deserialize, serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "image/ImageFinding.ts", rename = "ImageFinding")
)]
pub struct ImageFinding {
    pub section: String,
    pub text: String,
}

#[derive(Deserialize)]
pub struct ImageCaseReq {
    pub title: String,
    pub kind: String, // still | stack
    /// Ordered images; stacks render as a scrollable series (IMG-02).
    pub images: Vec<ImageRef>,
    pub findings: ImageFindingsReq,
    pub modality: Option<String>,
}

#[derive(Deserialize)]
pub struct SetImageCaseConceptsReq {
    pub concept_ids: Vec<Uuid>,
}

#[derive(Clone, Deserialize, serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "image/ImageRef.ts", rename = "ImageRef")
)]
pub struct ImageRef {
    pub url: String,
    pub rights_ref: String,
}

async fn active_display_refs(
    connection: &mut PgConnection,
    required: Option<&[String]>,
) -> ApiResult<std::collections::HashSet<String>> {
    let available = sqlx::query_scalar::<_, String>(
        r#"SELECT ref_code FROM content_rights
           WHERE ($1::text[] IS NULL OR ref_code = ANY($1))
             AND revoked_at IS NULL
             AND valid_from <= CURRENT_DATE
             AND (valid_to IS NULL OR valid_to >= CURRENT_DATE)
             AND permitted_uses @> '["display"]'::jsonb
           FOR SHARE"#,
    )
    .bind(required.map(|refs| refs.to_vec()))
    .fetch_all(&mut *connection)
    .await?;
    Ok(available.into_iter().collect())
}

async fn require_display_rights(
    connection: &mut PgConnection,
    images: &[ImageRef],
) -> ApiResult<()> {
    let mut required = images
        .iter()
        .map(|image| image.rights_ref.trim().to_ascii_uppercase())
        .collect::<Vec<_>>();
    required.sort();
    required.dedup();
    if required.is_empty() {
        return Err(ApiError::forbidden(
            "image_rights_unavailable",
            "every image needs a current content-rights grant that permits display",
        ));
    }
    let available = active_display_refs(connection, Some(&required)).await?;
    if available.len() != required.len() {
        return Err(ApiError::forbidden(
            "image_rights_unavailable",
            "every image needs a current content-rights grant that permits display",
        ));
    }
    Ok(())
}

pub async fn create_image_case(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(mut req): Json<ImageCaseReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    let title = req.title.trim();
    if title.is_empty() || title.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_title",
            "title must be 1-200 characters",
        ));
    }
    if !matches!(req.kind.as_str(), "still" | "stack") {
        return Err(ApiError::unprocessable(
            "invalid_kind",
            "kind must be still or stack",
        ));
    }
    if req.images.is_empty() || req.images.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_images",
            "1-200 images per case",
        ));
    }
    if req.kind == "still" && req.images.len() != 1 {
        return Err(ApiError::unprocessable(
            "invalid_images",
            "still image cases must contain exactly one image",
        ));
    }
    for img in &mut req.images {
        if !img.url.starts_with("https://") || img.rights_ref.trim().is_empty() {
            return Err(ApiError::unprocessable(
                "invalid_image",
                "every image needs an https URL and a rights reference (IMG-01)",
            ));
        }
        img.rights_ref = img.rights_ref.trim().to_ascii_uppercase();
    }
    let mut findings = match req.findings {
        ImageFindingsReq::Structured(findings) => findings,
        ImageFindingsReq::LegacyText(text) => vec![ImageFinding {
            section: "Findings".to_string(),
            text,
        }],
    };
    if findings.is_empty() || findings.len() > 20 {
        return Err(ApiError::unprocessable(
            "invalid_findings",
            "findings must contain 1-20 labeled sections",
        ));
    }
    let mut findings_characters = 0;
    for finding in &mut findings {
        finding.section = finding.section.trim().to_string();
        finding.text = finding.text.trim().to_string();
        let section_length = finding.section.chars().count();
        let text_length = finding.text.chars().count();
        if section_length == 0 || section_length > 80 || text_length == 0 || text_length > 2000 {
            return Err(ApiError::unprocessable(
                "invalid_findings",
                "each findings section needs a 1-80 character label and 1-2000 characters of text",
            ));
        }
        findings_characters += section_length + text_length;
    }
    if findings_characters > 5000 {
        return Err(ApiError::unprocessable(
            "invalid_findings",
            "findings must contain no more than 5000 characters in total",
        ));
    }
    let findings_text = findings
        .iter()
        .map(|finding| format!("{}: {}", finding.section, finding.text))
        .collect::<Vec<_>>()
        .join("\n\n");
    let findings_structured = serde_json::to_value(&findings).map_err(|_| ApiError::internal())?;
    let mut tx = state.pool.begin().await?;
    require_display_rights(&mut tx, &req.images).await?;
    let case_kind = req.kind.clone();
    let image_count = req.images.len();
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO image_cases
           (id, title, kind, images, findings, findings_structured, modality, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(title)
    .bind(case_kind.clone())
    .bind(serde_json::to_value(&req.images).map_err(|_| ApiError::internal())?)
    .bind(findings_text)
    .bind(findings_structured)
    .bind(req.modality)
    .bind(user.user_id)
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "image_case_created",
        "image_case",
        id,
        json!({ "kind": case_kind, "image_count": image_count }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "case_id": id })))
}

#[derive(sqlx::FromRow)]
struct ImageCaseRow {
    id: Uuid,
    title: String,
    kind: String,
    images: serde_json::Value,
    findings_structured: serde_json::Value,
    modality: Option<String>,
}

#[derive(Clone, serde::Serialize, sqlx::FromRow)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageConceptLink.ts",
        rename = "ImageConceptLink"
    )
)]
pub struct ImageConceptLink {
    pub concept_id: Uuid,
    pub canonical_key: String,
    pub version: i32,
    pub display_name: String,
    pub definition: String,
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "image/ImageCaseKind.ts", rename = "ImageCaseKind")
)]
pub enum ImageCaseKind {
    Still,
    Stack,
}

impl TryFrom<String> for ImageCaseKind {
    type Error = ApiError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "still" => Ok(Self::Still),
            "stack" => Ok(Self::Stack),
            _ => Err(ApiError::internal()),
        }
    }
}

#[derive(Clone, serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageCaseAnnotation.ts",
        rename = "ImageCaseAnnotation"
    )
)]
pub struct ImageCaseAnnotation {
    pub annotation_id: Uuid,
    pub image_index: i32,
    pub x_percent: f64,
    pub y_percent: f64,
    pub body: String,
}

#[derive(Clone, serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageCaseSummary.ts",
        rename = "ImageCaseSummary"
    )
)]
pub struct ImageCaseSummary {
    pub case_id: Uuid,
    pub title: String,
    pub kind: ImageCaseKind,
    pub images: Vec<ImageRef>,
    pub modality: Option<String>,
    pub concepts: Vec<ImageConceptLink>,
    pub annotations: Vec<ImageCaseAnnotation>,
}

#[derive(serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageCaseDetail.ts",
        rename = "ImageCaseDetail"
    )
)]
pub struct ImageCaseDetail {
    #[serde(flatten)]
    pub summary: ImageCaseSummary,
    pub findings: Vec<ImageFinding>,
}

#[derive(serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageCaseListResponse.ts",
        rename = "ImageCaseListResponse"
    )
)]
pub struct ImageCaseListResponse {
    pub cases: Vec<ImageCaseSummary>,
}

#[derive(serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageCaseConceptsResponse.ts",
        rename = "ImageCaseConceptsResponse"
    )
)]
pub struct ImageCaseConceptsResponse {
    pub case_id: Uuid,
    pub concepts: Vec<ImageConceptLink>,
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/PendingImageAnnotationStatus.ts",
        rename = "PendingImageAnnotationStatus"
    )
)]
pub enum PendingImageAnnotationStatus {
    Pending,
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageAnnotationDecision.ts",
        rename = "ImageAnnotationDecision"
    )
)]
pub enum ImageAnnotationDecision {
    Approved,
    Rejected,
}

impl ImageAnnotationDecision {
    fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/AdminImageAnnotation.ts",
        rename = "AdminImageAnnotation"
    )
)]
pub struct AdminImageAnnotation {
    pub annotation_id: Uuid,
    pub case_id: Uuid,
    pub case_title: String,
    pub image_index: i32,
    pub x_percent: f64,
    pub y_percent: f64,
    pub body: String,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub review_status: PendingImageAnnotationStatus,
}

#[derive(serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/AdminImageAnnotationListResponse.ts",
        rename = "AdminImageAnnotationListResponse"
    )
)]
pub struct AdminImageAnnotationListResponse {
    pub annotations: Vec<AdminImageAnnotation>,
}

#[derive(serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageAnnotationCreatedResponse.ts",
        rename = "ImageAnnotationCreatedResponse"
    )
)]
pub struct ImageAnnotationCreatedResponse {
    pub annotation_id: Uuid,
    pub review_status: PendingImageAnnotationStatus,
}

#[derive(serde::Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageAnnotationReviewResponse.ts",
        rename = "ImageAnnotationReviewResponse"
    )
)]
pub struct ImageAnnotationReviewResponse {
    pub annotation_id: Uuid,
    pub decision: ImageAnnotationDecision,
    pub review_status: ImageAnnotationDecision,
}

#[derive(sqlx::FromRow)]
struct ImageCaseConceptRow {
    case_id: Uuid,
    concept_id: Uuid,
    canonical_key: String,
    version: i32,
    display_name: String,
    definition: String,
}

impl From<ImageCaseConceptRow> for ImageConceptLink {
    fn from(row: ImageCaseConceptRow) -> Self {
        Self {
            concept_id: row.concept_id,
            canonical_key: row.canonical_key,
            version: row.version,
            display_name: row.display_name,
            definition: row.definition,
        }
    }
}

async fn image_case_concepts_for_cases(
    connection: &mut PgConnection,
    case_ids: &[Uuid],
) -> ApiResult<HashMap<Uuid, Vec<ImageConceptLink>>> {
    if case_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, ImageCaseConceptRow>(
        r#"SELECT link.case_id, link.concept_id, concept.canonical_key,
                  cv.version, cv.display_name, cv.definition
           FROM image_case_concepts link
           JOIN concepts concept ON concept.id = link.concept_id
           JOIN concept_versions cv
             ON cv.concept_id = link.concept_id
            AND cv.version = link.concept_version
           WHERE link.case_id = ANY($1)
           ORDER BY link.case_id, concept.canonical_key, link.concept_id"#,
    )
    .bind(case_ids)
    .fetch_all(&mut *connection)
    .await?;
    let mut by_case: HashMap<Uuid, Vec<ImageConceptLink>> = HashMap::new();
    for row in rows {
        let case_id = row.case_id;
        by_case.entry(case_id).or_default().push(row.into());
    }
    Ok(by_case)
}

#[derive(sqlx::FromRow)]
struct ApprovedImageAnnotation {
    case_id: Uuid,
    id: Uuid,
    image_index: i32,
    x_percent: f64,
    y_percent: f64,
    body: String,
}

impl From<ApprovedImageAnnotation> for ImageCaseAnnotation {
    fn from(annotation: ApprovedImageAnnotation) -> Self {
        Self {
            annotation_id: annotation.id,
            image_index: annotation.image_index,
            x_percent: annotation.x_percent,
            y_percent: annotation.y_percent,
            body: annotation.body,
        }
    }
}

pub async fn list_image_cases(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<ImageCaseListResponse>> {
    let mut tx = state.pool.begin().await?;
    // Hold shared locks until this response is fully read so a concurrent
    // license revocation cannot commit between the rights check and delivery.
    let active_refs = active_display_refs(&mut tx, None).await?;
    let active_ref_list = active_refs.iter().cloned().collect::<Vec<_>>();
    let rows = sqlx::query_as::<_, ImageCaseRow>(
        r#"SELECT id, title, kind, images, findings_structured, modality
           FROM image_cases
           WHERE jsonb_typeof(images) = 'array'
             AND jsonb_array_length(
                   CASE WHEN jsonb_typeof(images) = 'array' THEN images ELSE '[]'::jsonb END
                 ) > 0
             AND NOT EXISTS (
                   SELECT 1
                   FROM jsonb_array_elements(
                       CASE WHEN jsonb_typeof(images) = 'array' THEN images ELSE '[]'::jsonb END
                   ) AS image
                   WHERE UPPER(BTRIM(COALESCE(image.value ->> 'rights_ref', ''))) <> ALL($1)
                 )
           ORDER BY created_at DESC LIMIT 100"#,
    )
    .bind(&active_ref_list)
    .fetch_all(&mut *tx)
    .await?;
    let cases = rows
        .into_iter()
        .filter_map(|row| {
            let images: Vec<ImageRef> = serde_json::from_value(row.images.clone()).ok()?;
            if images.is_empty()
                || images.iter().any(|image| {
                    !active_refs.contains(&image.rights_ref.trim().to_ascii_uppercase())
                })
            {
                return None;
            }
            Some((row, images))
        })
        .collect::<Vec<_>>();
    let case_ids = cases.iter().map(|(case, _)| case.id).collect::<Vec<_>>();
    let mut concepts_by_case = image_case_concepts_for_cases(&mut tx, &case_ids).await?;
    let approved = if case_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, ApprovedImageAnnotation>(
            r#"SELECT annotation.case_id, annotation.id, annotation.image_index,
                      annotation.x_percent, annotation.y_percent, annotation.body
               FROM image_case_annotations annotation
               JOIN image_case_annotation_reviews review
                 ON review.annotation_id = annotation.id
               WHERE annotation.case_id = ANY($1) AND review.decision = 'approved'
               ORDER BY annotation.created_at, annotation.id"#,
        )
        .bind(&case_ids)
        .fetch_all(&mut *tx)
        .await?
    };
    let mut by_case: std::collections::HashMap<Uuid, Vec<ImageCaseAnnotation>> =
        std::collections::HashMap::new();
    for annotation in approved {
        by_case
            .entry(annotation.case_id)
            .or_default()
            .push(annotation.into());
    }
    let cases = cases
        .into_iter()
        .map(|(case, images)| -> ApiResult<ImageCaseSummary> {
            let case_id = case.id;
            Ok(ImageCaseSummary {
                case_id,
                title: case.title,
                kind: case.kind.try_into()?,
                images,
                modality: case.modality,
                concepts: concepts_by_case.remove(&case_id).unwrap_or_default(),
                annotations: by_case.remove(&case_id).unwrap_or_default(),
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    tx.commit().await?;
    Ok(Json(ImageCaseListResponse { cases }))
}

pub async fn get_image_case(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(case_id): Path<Uuid>,
) -> ApiResult<Json<ImageCaseDetail>> {
    let mut tx = state.pool.begin().await?;
    let case = sqlx::query_as::<_, ImageCaseRow>(
        r#"SELECT id, title, kind, images, findings_structured, modality
           FROM image_cases WHERE id = $1 FOR SHARE"#,
    )
    .bind(case_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("image_case_not_found"))?;
    let images: Vec<ImageRef> =
        serde_json::from_value(case.images.clone()).map_err(|_| ApiError::internal())?;
    if images.is_empty() {
        return Err(ApiError::not_found("image_case_not_found"));
    }
    require_display_rights(&mut tx, &images).await?;
    let mut concepts_by_case = image_case_concepts_for_cases(&mut tx, &[case_id]).await?;
    let concepts = concepts_by_case.remove(&case_id).unwrap_or_default();
    // IMG-02: only independently approved annotations reach learners.
    let annotations = sqlx::query_as::<_, ApprovedImageAnnotation>(
        r#"SELECT a.case_id, a.id, a.image_index, a.x_percent, a.y_percent, a.body
           FROM image_case_annotations a
           JOIN image_case_annotation_reviews r ON r.annotation_id = a.id
           WHERE a.case_id = $1 AND r.decision = 'approved'
           ORDER BY a.image_index, a.created_at, a.id"#,
    )
    .bind(case_id)
    .fetch_all(&mut *tx)
    .await?;
    let summary = ImageCaseSummary {
        case_id: case.id,
        title: case.title,
        kind: case.kind.try_into()?,
        images,
        modality: case.modality,
        concepts,
        annotations: annotations.into_iter().map(Into::into).collect(),
    };
    // IMG-02: findings are disclosed on request; platform review does not
    // establish clinical validity.
    let findings: Vec<ImageFinding> =
        serde_json::from_value(case.findings_structured).map_err(|_| ApiError::internal())?;
    let detail = ImageCaseDetail { summary, findings };
    tx.commit().await?;
    Ok(Json(detail))
}

// ---- IMG-04: version-pinned concept links ----------------------------------

pub async fn admin_image_case_concepts(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(case_id): Path<Uuid>,
) -> ApiResult<Json<ImageCaseConceptsResponse>> {
    let provided = headers
        .get("x-admin-token")
        .and_then(|value| value.to_str().ok());
    state.require_admin(provided)?;
    let case_exists = sqlx::query_scalar::<_, Uuid>("SELECT id FROM image_cases WHERE id = $1")
        .bind(case_id)
        .fetch_optional(&state.pool)
        .await?;
    if case_exists.is_none() {
        return Err(ApiError::not_found("image_case_not_found"));
    }
    let mut connection = state.pool.acquire().await?;
    let mut concepts_by_case = image_case_concepts_for_cases(&mut connection, &[case_id]).await?;
    Ok(Json(ImageCaseConceptsResponse {
        case_id,
        concepts: concepts_by_case.remove(&case_id).unwrap_or_default(),
    }))
}

pub async fn set_admin_image_case_concepts(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(case_id): Path<Uuid>,
    Json(mut req): Json<SetImageCaseConceptsReq>,
) -> ApiResult<Json<ImageCaseConceptsResponse>> {
    let provided = headers
        .get("x-admin-token")
        .and_then(|value| value.to_str().ok());
    state.require_admin(provided)?;
    if req.concept_ids.len() > 50 {
        return Err(ApiError::unprocessable(
            "too_many_image_concepts",
            "an image case may link to at most 50 concepts",
        ));
    }
    req.concept_ids.sort_unstable();
    if req.concept_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ApiError::unprocessable(
            "duplicate_image_concepts",
            "concept_ids must not contain duplicates",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let case_exists =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM image_cases WHERE id = $1 FOR UPDATE")
            .bind(case_id)
            .fetch_optional(&mut *tx)
            .await?;
    if case_exists.is_none() {
        return Err(ApiError::not_found("image_case_not_found"));
    }
    let mut before_by_case = image_case_concepts_for_cases(&mut tx, &[case_id]).await?;
    let before = before_by_case.remove(&case_id).unwrap_or_default();
    let concepts = sqlx::query_as::<_, ImageConceptLink>(
        r#"SELECT concept.id AS concept_id, concept.canonical_key,
                  cv.version, cv.display_name, cv.definition
           FROM concepts concept
           JOIN concept_versions cv
             ON cv.concept_id = concept.id
            AND cv.version = concept.current_version
           WHERE concept.id = ANY($1)
           ORDER BY concept.id
           FOR SHARE OF concept, cv"#,
    )
    .bind(&req.concept_ids)
    .fetch_all(&mut *tx)
    .await?;
    if concepts.len() != req.concept_ids.len() {
        return Err(ApiError::unprocessable(
            "image_concept_not_found",
            "every concept_id must refer to an existing concept",
        ));
    }

    sqlx::query("DELETE FROM image_case_concepts WHERE case_id = $1")
        .bind(case_id)
        .execute(&mut *tx)
        .await?;
    for concept in &concepts {
        sqlx::query!(
            "INSERT INTO image_case_concepts
                (case_id, concept_id, concept_version, created_by)
             VALUES ($1, $2, $3, $4)",
            case_id,
            concept.concept_id,
            concept.version,
            user.user_id
        )
        .execute(&mut *tx)
        .await?;
    }
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "image_case_concepts_updated",
        "image_case",
        case_id,
        json!({ "before": before, "after": concepts }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(ImageCaseConceptsResponse { case_id, concepts }))
}

// ---- IMG-02: annotation authoring and independent review --------------------
// Annotations are immutable teaching notes (DB triggers refuse UPDATE/DELETE);
// their review status is derived from the presence of a decision row, and
// learners only ever see approved ones.

#[derive(Deserialize)]
pub struct ImageAnnotationReq {
    pub image_index: i32,
    pub x_percent: f64,
    pub y_percent: f64,
    pub body: String,
}

fn contains_markup_tag(text: &str) -> bool {
    text.as_bytes().windows(2).any(|pair| {
        pair[0] == b'<' && (pair[1].is_ascii_alphabetic() || matches!(pair[1], b'/' | b'!' | b'?'))
    })
}

pub async fn create_image_annotation(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(case_id): Path<Uuid>,
    Json(req): Json<ImageAnnotationReq>,
) -> ApiResult<(StatusCode, Json<ImageAnnotationCreatedResponse>)> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    if req.image_index < 0 {
        return Err(ApiError::unprocessable(
            "invalid_image_index",
            "image_index must be 0 or greater",
        ));
    }
    if !(0.0..=100.0).contains(&req.x_percent) || !(0.0..=100.0).contains(&req.y_percent) {
        return Err(ApiError::unprocessable(
            "invalid_annotation_position",
            "x_percent and y_percent must be within 0-100",
        ));
    }
    let body = req.body.trim();
    if body.is_empty() || body.chars().count() > 1000 || contains_markup_tag(body) {
        return Err(ApiError::unprocessable(
            "invalid_annotation_body",
            "annotation body must be 1-1000 plain-text characters",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let images_value = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT images FROM image_cases WHERE id = $1 FOR SHARE",
    )
    .bind(case_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("image_case_not_found"))?;
    let images: Vec<ImageRef> =
        serde_json::from_value(images_value).map_err(|_| ApiError::internal())?;
    if req.image_index as usize >= images.len() {
        return Err(ApiError::unprocessable(
            "invalid_image_index",
            "image_index must refer to an image in this case",
        ));
    }
    require_display_rights(&mut tx, &images).await?;
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO image_case_annotations
           (id, case_id, image_index, x_percent, y_percent, body, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        id,
        case_id,
        req.image_index,
        req.x_percent,
        req.y_percent,
        body,
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "image_case_annotation_created",
        "image_case_annotation",
        id,
        json!({ "case_id": case_id, "image_index": req.image_index }),
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(ImageAnnotationCreatedResponse {
            annotation_id: id,
            review_status: PendingImageAnnotationStatus::Pending,
        }),
    ))
}

pub async fn list_image_annotations(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<AdminImageAnnotationListResponse>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    let rows = sqlx::query!(
        r#"SELECT a.id, a.case_id, c.title AS case_title, a.image_index,
                  a.x_percent, a.y_percent, a.body, a.created_at,
                  r.decision AS "decision?", r.note AS "note?",
                  r.created_at AS "reviewed_at?"
           FROM image_case_annotations a
           JOIN image_cases c ON c.id = a.case_id
           LEFT JOIN image_case_annotation_reviews r ON r.annotation_id = a.id
           WHERE r.annotation_id IS NULL
           ORDER BY a.created_at, a.id LIMIT 200"#
    )
    .fetch_all(&state.pool)
    .await?;
    let annotations = rows
        .iter()
        .map(|r| AdminImageAnnotation {
            annotation_id: r.id,
            case_id: r.case_id,
            case_title: r.case_title.clone(),
            image_index: r.image_index,
            x_percent: r.x_percent,
            y_percent: r.y_percent,
            body: r.body.clone(),
            created_at: r.created_at.to_owned(),
            review_status: PendingImageAnnotationStatus::Pending,
        })
        .collect();
    Ok(Json(AdminImageAnnotationListResponse { annotations }))
}

#[derive(Deserialize)]
pub struct ImageAnnotationReviewReq {
    pub decision: String,
    #[serde(default)]
    pub note: String,
}

pub async fn review_image_annotation(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(annotation_id): Path<Uuid>,
    Json(req): Json<ImageAnnotationReviewReq>,
) -> ApiResult<Json<ImageAnnotationReviewResponse>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    let decision = match req.decision.as_str() {
        "approved" => ImageAnnotationDecision::Approved,
        "rejected" => ImageAnnotationDecision::Rejected,
        _ => {
            return Err(ApiError::unprocessable(
                "invalid_decision",
                "decision must be approved or rejected",
            ));
        }
    };
    let note = req.note.trim();
    if note.is_empty() || note.chars().count() > 500 || contains_markup_tag(note) {
        return Err(ApiError::unprocessable(
            "invalid_review_note",
            "review note must be 1-500 plain-text characters",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let annotation = sqlx::query!(
        "SELECT id, case_id, image_index, created_by
         FROM image_case_annotations WHERE id = $1 FOR UPDATE",
        annotation_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let annotation = annotation.ok_or_else(|| ApiError::not_found("image_annotation_not_found"))?;
    if annotation.created_by == user.user_id {
        return Err(ApiError::forbidden(
            "annotation_review_requires_independent_reviewer",
            "the annotation's author cannot review it themselves",
        ));
    }
    let images_value = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT images FROM image_cases WHERE id = $1 FOR SHARE",
    )
    .bind(annotation.case_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("image_case_not_found"))?;
    let images: Vec<ImageRef> =
        serde_json::from_value(images_value).map_err(|_| ApiError::internal())?;
    if annotation.image_index < 0 || annotation.image_index as usize >= images.len() {
        return Err(ApiError::conflict(
            "annotation_image_unavailable",
            "the annotation no longer points to an image in this case",
        ));
    }
    require_display_rights(&mut tx, &images).await?;
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO image_case_annotation_reviews
           (annotation_id, reviewer_id, decision, note)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (annotation_id) DO NOTHING
         RETURNING annotation_id",
    )
    .bind(annotation_id)
    .bind(user.user_id)
    .bind(decision.as_str())
    .bind(note)
    .fetch_optional(&mut *tx)
    .await?;
    if inserted.is_none() {
        return Err(ApiError::conflict(
            "annotation_already_reviewed",
            "this annotation already has a final review decision",
        ));
    }
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "image_case_annotation_reviewed",
        "image_case_annotation",
        annotation_id,
        json!({ "decision": decision.as_str() }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(ImageAnnotationReviewResponse {
        annotation_id,
        decision,
        review_status: decision,
    }))
}
