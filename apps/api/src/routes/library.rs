//! LIB-01/02/03: versioned library articles with search and citation
//! anchors. Published versions only for learners; the editorial console's
//! authoring surface arrives with ADMIN-06 iteration 2. Jurisdiction/date
//! overlays (LIB-04) and hybrid semantic search (LIB-02 full) follow in the
//! Phase 2 library slice — this is the versioned + text-searched base.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub async fn search(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
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
        return Ok(Json(json!({ "results": [] })));
    }
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
    Ok(Json(json!({ "results": results })))
}

pub async fn get_article(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(slug): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let article = sqlx::query!(
        r#"SELECT a.id, a.slug, a.title, av.version, av.body, av.source_ref,
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
    let citations = sqlx::query!(
        "SELECT anchor, target, kind FROM article_citations WHERE version_id = $1",
        article.id
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
pub struct ImageCaseReq {
    pub title: String,
    pub kind: String, // still | stack
    /// Ordered images; stacks render as a scrollable series (IMG-02).
    pub images: Vec<ImageRef>,
    pub findings: String,
    pub modality: Option<String>,
}

#[derive(Deserialize, serde::Serialize)]
pub struct ImageRef {
    pub url: String,
    pub rights_ref: String,
}

pub async fn create_image_case(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<ImageCaseReq>,
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
    for img in &req.images {
        if !img.url.starts_with("https://") || img.rights_ref.trim().is_empty() {
            return Err(ApiError::unprocessable(
                "invalid_image",
                "every image needs an https URL and a rights reference (IMG-01)",
            ));
        }
    }
    let findings = req.findings.trim();
    if findings.is_empty() {
        return Err(ApiError::unprocessable(
            "invalid_findings",
            "findings are required",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO image_cases
           (id, title, kind, images, findings, modality, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        id,
        title,
        req.kind,
        serde_json::to_value(&req.images).map_err(|_| ApiError::internal())?,
        findings,
        req.modality,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "case_id": id })))
}

pub async fn list_image_cases(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT id, title, kind, images, findings, modality AS "modality?"
           FROM image_cases ORDER BY created_at DESC LIMIT 100"#
    )
    .fetch_all(&state.pool)
    .await?;
    let cases: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "case_id": r.id,
                "title": r.title,
                "kind": r.kind,
                "images": r.images,
                "modality": r.modality,
                "findings": r.findings,
            })
        })
        .collect();
    Ok(Json(json!({ "cases": cases })))
}
