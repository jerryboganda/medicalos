//! LIB-01/02/03/04: versioned articles, source citations, and explicit
//! jurisdiction/date resolution. Published versions only are visible to learners.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::authz::Permission;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const PRIVATE_IMPORT_MAX_BYTES: usize = 1024 * 1024;
const PRIVATE_IMPORT_MAX_DOCUMENTS: i64 = 25;
const PRIVATE_IMPORT_MAX_TOTAL_BYTES: i64 = 10 * 1024 * 1024;
const MAX_MEDIA_DURATION_MS: u64 = 86_400_000;

#[derive(Clone, Debug, Deserialize, Serialize, sqlx::FromRow)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/ArticleCitation.ts",
        rename = "ArticleCitation"
    )
)]
pub struct ArticleCitation {
    pub kind: String,
    pub anchor: String,
    pub target: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/MediaCaptionCue.ts",
        rename = "MediaCaptionCue"
    )
)]
pub struct MediaCaptionCue {
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub start_ms: u64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub end_ms: u64,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/MediaChapterMarker.ts",
        rename = "MediaChapterMarker"
    )
)]
pub struct MediaChapterMarker {
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub at_ms: u64,
    pub title: String,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "library/ArticleMedia.ts", rename = "ArticleMedia")
)]
pub struct ArticleMedia {
    pub media_id: Uuid,
    pub url: String,
    #[cfg_attr(feature = "type-export", ts(type = "\"audio\" | \"video\""))]
    pub kind: String,
    pub duration_seconds: Option<i32>,
    pub captions: Vec<MediaCaptionCue>,
    pub chapters: Vec<MediaChapterMarker>,
    pub rights_ref: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/CreateArticleRequest.ts",
        rename = "CreateArticleRequest"
    )
)]
#[serde(deny_unknown_fields)]
pub struct CreateArticleRequest {
    pub slug: String,
    pub title: String,
    pub body: String,
    pub source_ref: String,
    pub jurisdiction: Option<String>,
    pub effective_from: Option<String>,
    pub effective_to: Option<String>,
    pub citations: Vec<ArticleCitation>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/UpdateArticleDraftRequest.ts",
        rename = "UpdateArticleDraftRequest"
    )
)]
#[serde(deny_unknown_fields)]
pub struct UpdateArticleDraftRequest {
    pub body: String,
    pub source_ref: String,
    pub jurisdiction: Option<String>,
    pub effective_from: Option<String>,
    pub effective_to: Option<String>,
    pub citations: Vec<ArticleCitation>,
}

#[derive(Debug)]
struct ArticleDraftFields {
    body: String,
    source_ref: String,
    jurisdiction: Option<String>,
    effective_from: Option<chrono::NaiveDate>,
    effective_to: Option<chrono::NaiveDate>,
    citations: Vec<ArticleCitation>,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/AdminArticleSummary.ts",
        rename = "AdminArticleSummary"
    )
)]
pub struct AdminArticleSummary {
    pub article_id: Uuid,
    pub slug: String,
    pub title: String,
    pub version_id: Uuid,
    pub version: i32,
    pub status: String,
    pub jurisdiction: Option<String>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    pub effective_from: Option<chrono::NaiveDate>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    pub effective_to: Option<chrono::NaiveDate>,
    pub is_latest: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/AdminArticleListResponse.ts",
        rename = "AdminArticleListResponse"
    )
)]
pub struct AdminArticleListResponse {
    pub articles: Vec<AdminArticleSummary>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/AdminArticleVersion.ts",
        rename = "AdminArticleVersion"
    )
)]
pub struct AdminArticleVersion {
    pub article_id: Uuid,
    pub version_id: Uuid,
    pub slug: String,
    pub title: String,
    pub version: i32,
    pub status: String,
    pub body: String,
    pub source_ref: String,
    pub jurisdiction: Option<String>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    pub effective_from: Option<chrono::NaiveDate>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    pub effective_to: Option<chrono::NaiveDate>,
    pub citations: Vec<ArticleCitation>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/PublishArticleResponse.ts",
        rename = "PublishArticleResponse"
    )
)]
pub struct PublishArticleResponse {
    pub article_id: Uuid,
    pub version_id: Uuid,
    pub version: i32,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/LibrarySearchResult.ts",
        rename = "LibrarySearchResult"
    )
)]
pub struct LibrarySearchResult {
    pub content_type: String,
    pub article_id: Uuid,
    pub slug: String,
    pub title: String,
    pub version: i32,
    pub jurisdiction: Option<String>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    pub effective_from: Option<chrono::NaiveDate>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    pub effective_to: Option<chrono::NaiveDate>,
    pub as_of: String,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub score: i64,
    pub source_ref: String,
    pub excerpt: String,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/PrivateDocumentSearchResult.ts",
        rename = "PrivateDocumentSearchResult"
    )
)]
pub struct PrivateDocumentSearchResult {
    pub document_id: Uuid,
    pub content_type: String,
    pub title: String,
    pub media_type: String,
    pub rights_ref: String,
    pub sha256: String,
    pub created_at: String,
    pub available: bool,
    pub excerpt: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/LibrarySearchResponse.ts",
        rename = "LibrarySearchResponse"
    )
)]
pub struct LibrarySearchResponse {
    pub results: Vec<LibrarySearchResult>,
    pub private_documents: Vec<PrivateDocumentSearchResult>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/LibraryArticleResponse.ts",
        rename = "LibraryArticleResponse"
    )
)]
pub struct LibraryArticleResponse {
    pub slug: String,
    pub title: String,
    pub version: i32,
    pub body: String,
    pub source_ref: String,
    pub selected_jurisdiction: Option<String>,
    pub as_of: String,
    pub jurisdiction: Option<String>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    pub effective_from: Option<chrono::NaiveDate>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    pub effective_to: Option<chrono::NaiveDate>,
    pub citations: Vec<ArticleCitation>,
    pub media: Vec<ArticleMedia>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryScopeQuery {
    jurisdiction: Option<String>,
    as_of: Option<String>,
}

#[derive(Debug)]
struct LibraryScope {
    jurisdiction: Option<String>,
    as_of: chrono::NaiveDate,
}

#[derive(sqlx::FromRow)]
struct ArticleVersionRow {
    article_id: Uuid,
    version_id: Uuid,
    slug: String,
    title: String,
    version: i32,
    status: String,
    body: String,
    source_ref: String,
    jurisdiction: Option<String>,
    effective_from: Option<chrono::NaiveDate>,
    effective_to: Option<chrono::NaiveDate>,
}

fn require_permission(
    state: &AppState,
    user: &crate::auth::AuthUser,
    headers: &HeaderMap,
    permission: Permission,
) -> ApiResult<()> {
    state.require_permission(
        user,
        headers.get("x-admin-token").and_then(|v| v.to_str().ok()),
        permission,
    )
}

fn normalize_jurisdiction(value: Option<&str>) -> ApiResult<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() != 2 || !value.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(ApiError::unprocessable(
            "invalid_jurisdiction",
            "jurisdiction must be a two-letter country code",
        ));
    }
    Ok(Some(value.to_ascii_uppercase()))
}

fn parse_article_date(value: Option<&str>) -> ApiResult<Option<chrono::NaiveDate>> {
    value
        .map(|value| {
            chrono::NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|_| {
                ApiError::unprocessable("invalid_effective_date", "dates must use YYYY-MM-DD")
            })
        })
        .transpose()
}

fn library_scope(query: LibraryScopeQuery) -> ApiResult<LibraryScope> {
    let jurisdiction = normalize_jurisdiction(query.jurisdiction.as_deref())?;
    let as_of = match query.as_of.as_deref() {
        Some(value) => {
            chrono::NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|_| {
                ApiError::unprocessable("invalid_as_of_date", "as_of must use YYYY-MM-DD")
            })?
        }
        None => chrono::Utc::now().date_naive(),
    };
    Ok(LibraryScope {
        jurisdiction,
        as_of,
    })
}

fn valid_article_slug(slug: &str) -> bool {
    (1..=120).contains(&slug.len())
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && slug
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && slug
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
}

fn clean_article_text(value: &str, field: &'static str, max: usize) -> ApiResult<String> {
    let value = value.trim();
    let length = value.chars().count();
    if length == 0 || length > max {
        return Err(ApiError::unprocessable(
            "invalid_article_field",
            format!("{field} must be 1-{max} characters"),
        ));
    }
    Ok(value.to_string())
}

fn valid_timestamp(value: &str) -> bool {
    let parts: Vec<_> = value.split(':').collect();
    if !(2..=3).contains(&parts.len())
        || !parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return false;
    }
    match parts.as_slice() {
        [minutes, seconds] => {
            minutes.parse::<u32>().is_ok() && seconds.parse::<u8>().is_ok_and(|value| value < 60)
        }
        [hours, minutes, seconds] => {
            hours.parse::<u32>().is_ok()
                && minutes.parse::<u8>().is_ok_and(|value| value < 60)
                && seconds.parse::<u8>().is_ok_and(|value| value < 60)
        }
        _ => false,
    }
}

fn clean_article_draft(
    body: &str,
    source_ref: &str,
    jurisdiction: Option<&str>,
    effective_from: Option<&str>,
    effective_to: Option<&str>,
    mut citations: Vec<ArticleCitation>,
) -> ApiResult<ArticleDraftFields> {
    let body = clean_article_text(body, "body", 200_000)?;
    let source_ref = clean_article_text(source_ref, "source_ref", 512)?;
    let jurisdiction = normalize_jurisdiction(jurisdiction)?;
    let effective_from = parse_article_date(effective_from)?;
    let effective_to = parse_article_date(effective_to)?;
    if effective_from
        .zip(effective_to)
        .is_some_and(|(from, to)| from > to)
    {
        return Err(ApiError::unprocessable(
            "invalid_effective_range",
            "effective_from must be on or before effective_to",
        ));
    }
    if citations.len() > 100 {
        return Err(ApiError::unprocessable(
            "too_many_citations",
            "an article can have at most 100 citations",
        ));
    }
    for citation in &mut citations {
        citation.kind = citation.kind.trim().to_ascii_lowercase();
        citation.anchor = clean_article_text(&citation.anchor, "citation anchor", 160)?;
        citation.target = clean_article_text(&citation.target, "citation target", 512)?;
        if !matches!(
            citation.kind.as_str(),
            "source" | "page" | "figure" | "timestamp"
        ) {
            return Err(ApiError::unprocessable(
                "invalid_citation_kind",
                "citation kind must be source, page, figure, or timestamp",
            ));
        }
        if citation.kind == "timestamp" && !valid_timestamp(&citation.target) {
            return Err(ApiError::unprocessable(
                "invalid_citation_timestamp",
                "timestamp targets must use MM:SS or HH:MM:SS",
            ));
        }
        if !body
            .to_lowercase()
            .contains(&citation.anchor.to_lowercase())
        {
            return Err(ApiError::unprocessable(
                "citation_anchor_not_found",
                "each citation anchor must appear in the article body",
            ));
        }
    }
    Ok(ArticleDraftFields {
        body,
        source_ref,
        jurisdiction,
        effective_from,
        effective_to,
        citations,
    })
}

async fn replace_article_citations(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    version_id: Uuid,
    citations: &[ArticleCitation],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM article_citations WHERE version_id = $1")
        .bind(version_id)
        .execute(&mut **tx)
        .await?;
    for citation in citations {
        sqlx::query(
            "INSERT INTO article_citations (id, version_id, anchor, target, kind)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(version_id)
        .bind(&citation.anchor)
        .bind(&citation.target)
        .bind(&citation.kind)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn fetch_admin_article_version(
    state: &AppState,
    article_id: Uuid,
    version_id: Uuid,
) -> ApiResult<AdminArticleVersion> {
    let row = sqlx::query_as::<_, ArticleVersionRow>(
        r#"SELECT a.id AS article_id, av.id AS version_id, a.slug, a.title,
                  av.version, av.status, av.body, av.source_ref, av.jurisdiction,
                  av.effective_from, av.effective_to
           FROM articles a
           JOIN article_versions av ON av.article_id = a.id
           WHERE a.id = $1 AND av.id = $2"#,
    )
    .bind(article_id)
    .bind(version_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("article_version_not_found"))?;
    let citations = sqlx::query_as::<_, ArticleCitation>(
        "SELECT kind, anchor, target FROM article_citations WHERE version_id = $1 ORDER BY anchor, kind, target",
    )
    .bind(version_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(AdminArticleVersion {
        article_id: row.article_id,
        version_id: row.version_id,
        slug: row.slug,
        title: row.title,
        version: row.version,
        status: row.status,
        body: row.body,
        source_ref: row.source_ref,
        jurisdiction: row.jurisdiction,
        effective_from: row.effective_from,
        effective_to: row.effective_to,
        citations,
    })
}

pub async fn admin_list_articles(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<AdminArticleListResponse>> {
    require_permission(&state, &user, &headers, Permission::ContentAuthor)?;
    let articles = sqlx::query_as::<_, AdminArticleSummary>(
        r#"SELECT a.id AS article_id, a.slug, a.title, av.id AS version_id,
                  av.version, av.status, av.jurisdiction, av.effective_from,
                  av.effective_to,
                  av.version = (SELECT MAX(version) FROM article_versions
                                WHERE article_id = a.id) AS is_latest
           FROM articles a
           JOIN article_versions av ON av.article_id = a.id
           ORDER BY a.created_at DESC, av.version DESC"#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(AdminArticleListResponse { articles }))
}

pub async fn create_article(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Json(req): Json<CreateArticleRequest>,
) -> ApiResult<Json<AdminArticleVersion>> {
    require_permission(&state, &user, &headers, Permission::ContentAuthor)?;
    let slug = req.slug.trim().to_string();
    if !valid_article_slug(&slug) {
        return Err(ApiError::unprocessable(
            "invalid_article_slug",
            "slug must be 1-120 lowercase letters, digits, or hyphens and start/end with a letter or digit",
        ));
    }
    let title = clean_article_text(&req.title, "title", 200)?;
    let fields = clean_article_draft(
        &req.body,
        &req.source_ref,
        req.jurisdiction.as_deref(),
        req.effective_from.as_deref(),
        req.effective_to.as_deref(),
        req.citations,
    )?;
    let article_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let mut tx = state.pool.begin().await?;
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO articles (id, slug, title) VALUES ($1, $2, $3)
         ON CONFLICT (slug) DO NOTHING RETURNING id",
    )
    .bind(article_id)
    .bind(&slug)
    .bind(&title)
    .fetch_optional(&mut *tx)
    .await?;
    if inserted.is_none() {
        return Err(ApiError::conflict(
            "article_slug_exists",
            "an article already uses this slug",
        ));
    }
    sqlx::query(
        "INSERT INTO article_versions
         (id, article_id, version, status, body, source_ref, jurisdiction, effective_from, effective_to)
         VALUES ($1, $2, 1, 'draft', $3, $4, $5, $6, $7)",
    )
    .bind(version_id)
    .bind(article_id)
    .bind(&fields.body)
    .bind(&fields.source_ref)
    .bind(&fields.jurisdiction)
    .bind(fields.effective_from)
    .bind(fields.effective_to)
    .execute(&mut *tx)
    .await?;
    replace_article_citations(&mut tx, version_id, &fields.citations).await?;
    super::admin::audit(
        &mut *tx,
        user.user_id,
        "article_draft_created",
        "article_version",
        version_id,
        json!({
            "article_id": article_id,
            "slug": slug,
            "version": 1,
            "status": "draft",
            "jurisdiction": fields.jurisdiction,
            "citation_count": fields.citations.len()
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(
        fetch_admin_article_version(&state, article_id, version_id).await?,
    ))
}

pub async fn create_article_version(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(article_id): Path<Uuid>,
) -> ApiResult<Json<AdminArticleVersion>> {
    require_permission(&state, &user, &headers, Permission::ContentAuthor)?;
    let mut tx = state.pool.begin().await?;
    let exists = sqlx::query_scalar::<_, Uuid>("SELECT id FROM articles WHERE id = $1 FOR UPDATE")
        .bind(article_id)
        .fetch_optional(&mut *tx)
        .await?;
    if exists.is_none() {
        return Err(ApiError::not_found("article_not_found"));
    }
    let has_draft = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM article_versions WHERE article_id = $1 AND status = 'draft')",
    )
    .bind(article_id)
    .fetch_one(&mut *tx)
    .await?;
    if has_draft {
        return Err(ApiError::conflict(
            "article_draft_exists",
            "publish or finish the current draft before creating another",
        ));
    }
    let source = sqlx::query_as::<_, ArticleVersionRow>(
        r#"SELECT a.id AS article_id, av.id AS version_id, a.slug, a.title,
                  av.version, av.status, av.body, av.source_ref, av.jurisdiction,
                  av.effective_from, av.effective_to
           FROM articles a
           JOIN article_versions av ON av.article_id = a.id
           WHERE a.id = $1 AND av.status = 'published'
           ORDER BY av.version DESC LIMIT 1"#,
    )
    .bind(article_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::conflict(
            "article_has_no_published_version",
            "publish the initial draft before creating a revision",
        )
    })?;
    let version: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) + 1 FROM article_versions WHERE article_id = $1",
    )
    .bind(article_id)
    .fetch_one(&mut *tx)
    .await?;
    let version_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO article_versions
         (id, article_id, version, status, body, source_ref, jurisdiction, effective_from, effective_to)
         VALUES ($1, $2, $3, 'draft', $4, $5, $6, $7, $8)",
    )
    .bind(version_id)
    .bind(article_id)
    .bind(version)
    .bind(&source.body)
    .bind(&source.source_ref)
    .bind(&source.jurisdiction)
    .bind(source.effective_from)
    .bind(source.effective_to)
    .execute(&mut *tx)
    .await?;
    let citations = sqlx::query_as::<_, ArticleCitation>(
        "SELECT kind, anchor, target FROM article_citations WHERE version_id = $1 ORDER BY anchor, kind, target",
    )
    .bind(source.version_id)
    .fetch_all(&mut *tx)
    .await?;
    replace_article_citations(&mut tx, version_id, &citations).await?;
    super::admin::audit(
        &mut *tx,
        user.user_id,
        "article_draft_created",
        "article_version",
        version_id,
        json!({
            "article_id": article_id,
            "slug": source.slug,
            "version": version,
            "status": "draft",
            "jurisdiction": source.jurisdiction,
            "citation_count": citations.len()
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(
        fetch_admin_article_version(&state, article_id, version_id).await?,
    ))
}

pub async fn get_admin_article_version(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path((article_id, version_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<AdminArticleVersion>> {
    require_permission(&state, &user, &headers, Permission::ContentAuthor)?;
    Ok(Json(
        fetch_admin_article_version(&state, article_id, version_id).await?,
    ))
}

pub async fn update_article_draft(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path((article_id, version_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateArticleDraftRequest>,
) -> ApiResult<Json<AdminArticleVersion>> {
    require_article_admin(&state, &user, &headers)?;
    let fields = clean_article_draft(
        &req.body,
        &req.source_ref,
        req.jurisdiction.as_deref(),
        req.effective_from.as_deref(),
        req.effective_to.as_deref(),
        req.citations,
    )?;
    let mut tx = state.pool.begin().await?;
    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM article_versions WHERE article_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(article_id)
    .bind(version_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("article_version_not_found"))?;
    if status != "draft" {
        return Err(ApiError::conflict(
            "article_version_not_draft",
            "published article versions are immutable",
        ));
    }
    sqlx::query(
        "UPDATE article_versions SET body = $3, source_ref = $4, jurisdiction = $5,
                effective_from = $6, effective_to = $7
         WHERE article_id = $1 AND id = $2",
    )
    .bind(article_id)
    .bind(version_id)
    .bind(&fields.body)
    .bind(&fields.source_ref)
    .bind(&fields.jurisdiction)
    .bind(fields.effective_from)
    .bind(fields.effective_to)
    .execute(&mut *tx)
    .await?;
    replace_article_citations(&mut tx, version_id, &fields.citations).await?;
    super::admin::audit(
        &mut *tx,
        user.user_id,
        "article_draft_updated",
        "article_version",
        version_id,
        json!({
            "article_id": article_id,
            "status": "draft",
            "jurisdiction": fields.jurisdiction,
            "citation_count": fields.citations.len()
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(
        fetch_admin_article_version(&state, article_id, version_id).await?,
    ))
}

pub async fn publish_article_draft(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path((article_id, version_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<PublishArticleResponse>> {
    require_article_admin(&state, &user, &headers)?;
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query_as::<_, ArticleVersionRow>(
        r#"SELECT a.id AS article_id, av.id AS version_id, a.slug, a.title,
                  av.version, av.status, av.body, av.source_ref, av.jurisdiction,
                  av.effective_from, av.effective_to
           FROM articles a
           JOIN article_versions av ON av.article_id = a.id
           WHERE a.id = $1 AND av.id = $2 FOR UPDATE OF av"#,
    )
    .bind(article_id)
    .bind(version_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("article_version_not_found"))?;
    if row.status != "draft" {
        return Err(ApiError::conflict(
            "article_version_not_draft",
            "only a draft article version can be published",
        ));
    }
    let citation_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM article_citations WHERE version_id = $1")
            .bind(version_id)
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query("UPDATE article_versions SET status = 'published' WHERE id = $1")
        .bind(version_id)
        .execute(&mut *tx)
        .await?;
    super::admin::audit(
        &mut *tx,
        user.user_id,
        "article_version_published",
        "article_version",
        version_id,
        json!({
            "article_id": article_id,
            "slug": row.slug,
            "version": row.version,
            "status": "published",
            "jurisdiction": row.jurisdiction,
            "citation_count": citation_count
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(PublishArticleResponse {
        article_id,
        version_id,
        version: row.version,
        status: "published".to_string(),
    }))
}

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
) -> ApiResult<Json<LibrarySearchResponse>> {
    let scope = library_scope(LibraryScopeQuery {
        jurisdiction: q.get("jurisdiction").cloned(),
        as_of: q.get("as_of").cloned(),
    })?;
    // LIB-02 hybrid ranking: the whole-phrase match anchors the query, then
    // per-token hits add weighted signal (title hits outweigh body hits).
    // Pure lexical by design — no vector index is claimed or faked.
    let query_text = q
        .get("q")
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();
    if query_text.is_empty() {
        return Ok(Json(LibrarySearchResponse {
            results: Vec::new(),
            private_documents: Vec::new(),
        }));
    }
    require_library_allowance(&state, user.user_id).await?;
    let tokens: Vec<String> = query_text
        .split_whitespace()
        .filter(|t| t.len() >= 2)
        .map(String::from)
        .collect();
    let patterns: Vec<String> = tokens
        .iter()
        .map(|t| format!("%{t}%"))
        .chain(std::iter::once(format!("%{query_text}%")))
        .collect();
    #[derive(sqlx::FromRow)]
    struct EditorialArticleHit {
        article_id: Uuid,
        slug: String,
        title: String,
        version: i32,
        body: String,
        source_ref: String,
        jurisdiction: Option<String>,
        effective_from: Option<chrono::NaiveDate>,
        effective_to: Option<chrono::NaiveDate>,
    }
    let rows = sqlx::query_as::<_, EditorialArticleHit>(
        r#"WITH applicable AS (
               SELECT a.id AS article_id, a.slug, a.title, av.version, av.body,
                      av.source_ref, av.jurisdiction, av.effective_from, av.effective_to,
                      ROW_NUMBER() OVER (
                          PARTITION BY a.id
                          ORDER BY CASE WHEN av.jurisdiction = $2 THEN 0 ELSE 1 END,
                                   av.effective_from DESC NULLS LAST, av.version DESC
                      ) AS scope_rank
               FROM articles a
               JOIN article_versions av ON av.article_id = a.id
               WHERE av.status = 'published'
                 AND (av.jurisdiction IS NULL OR av.jurisdiction = $2)
                 AND (av.effective_from IS NULL OR av.effective_from <= $3)
                 AND (av.effective_to IS NULL OR av.effective_to >= $3)
           )
           SELECT article_id, slug, title, version, body, source_ref,
                  jurisdiction, effective_from, effective_to
           FROM applicable
           WHERE scope_rank = 1 AND (title ILIKE ANY($1) OR body ILIKE ANY($1))
           LIMIT 100"#,
    )
    .bind(&patterns)
    .bind(scope.jurisdiction.as_deref())
    .bind(scope.as_of)
    .fetch_all(&state.pool)
    .await?;
    let as_of = scope.as_of.to_string();
    let mut results: Vec<LibrarySearchResult> = rows
        .into_iter()
        .map(|r| {
            let title_lower = r.title.to_lowercase();
            let body_lower = r.body.to_lowercase();
            let mut score: i64 = 0;
            if body_lower.contains(&query_text) {
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
            LibrarySearchResult {
                content_type: "editorial_article".into(),
                article_id: r.article_id,
                slug: r.slug,
                title: r.title,
                version: r.version,
                jurisdiction: r.jurisdiction,
                effective_from: r.effective_from,
                effective_to: r.effective_to,
                as_of: as_of.clone(),
                score,
                source_ref: r.source_ref,
                excerpt: r.body.chars().take(200).collect(),
            }
        })
        .collect();
    results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.title.cmp(&b.title)));
    results.truncate(25);
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
    let private_documents: Vec<PrivateDocumentSearchResult> = private_documents
        .into_iter()
        .map(|document| PrivateDocumentSearchResult {
            document_id: document.document_id,
            content_type: "private_document".into(),
            title: document.title,
            media_type: document.media_type,
            rights_ref: document.rights_ref,
            sha256: document.sha256,
            created_at: document.created_at.to_rfc3339(),
            available: true,
            excerpt: document.content.chars().take(200).collect(),
        })
        .collect();
    Ok(Json(LibrarySearchResponse {
        results,
        private_documents,
    }))
}

pub async fn get_article(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(slug): Path<String>,
    Query(query): Query<LibraryScopeQuery>,
) -> ApiResult<Json<LibraryArticleResponse>> {
    let scope = library_scope(query)?;
    let article = sqlx::query_as::<_, ArticleVersionRow>(
        r#"SELECT a.id AS article_id, av.id AS version_id, a.slug, a.title,
                  av.version, av.status, av.body, av.source_ref, av.jurisdiction,
                  av.effective_from, av.effective_to
           FROM articles a
           JOIN article_versions av ON av.article_id = a.id
           WHERE a.slug = $1
             AND av.status = 'published'
             AND (av.jurisdiction IS NULL OR av.jurisdiction = $2)
             AND (av.effective_from IS NULL OR av.effective_from <= $3)
             AND (av.effective_to IS NULL OR av.effective_to >= $3)
           ORDER BY CASE WHEN av.jurisdiction = $2 THEN 0 ELSE 1 END,
                    av.effective_from DESC NULLS LAST, av.version DESC
           LIMIT 1"#,
    )
    .bind(slug)
    .bind(scope.jurisdiction.as_deref())
    .bind(scope.as_of)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("article_not_available_for_region"))?;
    require_library_allowance(&state, _user.user_id).await?;
    sqlx::query!(
        "INSERT INTO article_reads (user_id, article_version_id)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
        _user.user_id,
        article.version_id
    )
    .execute(&state.pool)
    .await?;
    let citations = sqlx::query_as::<_, ArticleCitation>(
        "SELECT kind, anchor, target FROM article_citations WHERE version_id = $1 ORDER BY anchor, kind, target",
    )
    .bind(article.version_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(LibraryArticleResponse {
        slug: article.slug,
        title: article.title,
        version: article.version,
        body: article.body,
        source_ref: article.source_ref,
        selected_jurisdiction: scope.jurisdiction,
        as_of: scope.as_of.to_string(),
        jurisdiction: article.jurisdiction,
        effective_from: article.effective_from,
        effective_to: article.effective_to,
        citations,
        media: media_list(&state, article.article_id).await?,
    }))
}

#[derive(Serialize, sqlx::FromRow)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/PrivateImportRight.ts",
        rename = "PrivateImportRight"
    )
)]
pub struct PrivateImportRight {
    rights_id: Uuid,
    ref_code: String,
    licensor: String,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    valid_to: Option<chrono::NaiveDate>,
    search_allowed: bool,
}

#[derive(sqlx::FromRow, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/PrivateImportSummary.ts",
        rename = "PrivateImportSummary"
    )
)]
pub struct PrivateDocumentSummary {
    document_id: Uuid,
    title: String,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"text/plain\" | \"text/markdown\"")
    )]
    media_type: String,
    rights_ref: String,
    sha256: String,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    created_at: chrono::DateTime<chrono::Utc>,
    available: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/PrivateImportRightsResponse.ts",
        rename = "PrivateImportRightsResponse"
    )
)]
pub struct PrivateImportRightsResponse {
    pub rights: Vec<PrivateImportRight>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/PrivateImportListResponse.ts",
        rename = "PrivateImportListResponse"
    )
)]
pub struct PrivateImportListResponse {
    pub documents: Vec<PrivateDocumentSummary>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/PrivateImportResponse.ts",
        rename = "PrivateImportResponse"
    )
)]
pub struct PrivateImportResponse {
    pub document_id: Uuid,
    pub title: String,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"text/plain\" | \"text/markdown\"")
    )]
    pub media_type: String,
    pub rights_ref: String,
    pub sha256: String,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub available: bool,
    pub content: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/PrivateImportDeletedResponse.ts",
        rename = "PrivateImportDeletedResponse"
    )
)]
pub struct PrivateImportDeletedResponse {
    pub deleted: bool,
}

#[derive(sqlx::FromRow)]
struct PrivateImportQuota {
    document_count: i64,
    total_bytes: i64,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "library/CreatePrivateImportRequest.ts",
        rename = "CreatePrivateImportRequest"
    )
)]
pub struct CreatePrivateImportReq {
    pub title: String,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"text/plain\" | \"text/markdown\"")
    )]
    pub media_type: String,
    pub content: String,
    pub rights_ref: String,
}

pub async fn private_import_rights(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<PrivateImportRightsResponse>> {
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
    Ok(Json(PrivateImportRightsResponse { rights }))
}

pub async fn list_private_imports(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<PrivateImportListResponse>> {
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
    Ok(Json(PrivateImportListResponse { documents }))
}

pub async fn create_private_import(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CreatePrivateImportReq>,
) -> ApiResult<(StatusCode, Json<PrivateDocumentSummary>)> {
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
        Json(PrivateDocumentSummary {
            document_id,
            title: title.into(),
            media_type,
            rights_ref,
            sha256,
            created_at,
            available: true,
        }),
    ))
}

pub async fn get_private_import(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> ApiResult<Json<PrivateImportResponse>> {
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
    Ok(Json(PrivateImportResponse {
        document_id: metadata.document_id,
        title: metadata.title,
        media_type: metadata.media_type,
        rights_ref: metadata.rights_ref,
        sha256: metadata.sha256,
        created_at: metadata.created_at,
        available: metadata.available,
        content,
    }))
}

pub async fn delete_private_import(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> ApiResult<Json<PrivateImportDeletedResponse>> {
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
    Ok(Json(PrivateImportDeletedResponse { deleted: true }))
}

// ---- LIB-08: media with captions and chapters (rights reference required) ----

pub async fn media_list(state: &AppState, article_id: Uuid) -> ApiResult<Vec<ArticleMedia>> {
    let rows = sqlx::query!(
        r#"SELECT id, url, kind, duration_seconds AS "duration_seconds?",
                  captions, chapters, rights_ref
           FROM media_assets WHERE article_id = $1 ORDER BY created_at"#,
        article_id
    )
    .fetch_all(&state.pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(ArticleMedia {
                media_id: row.id,
                url: row.url,
                kind: row.kind,
                duration_seconds: row.duration_seconds,
                captions: serde_json::from_value(row.captions).map_err(|_| ApiError::internal())?,
                chapters: serde_json::from_value(row.chapters).map_err(|_| ApiError::internal())?,
                rights_ref: row.rights_ref,
            })
        })
        .collect()
}

#[derive(Deserialize)]
pub struct MediaReq {
    pub url: String,
    pub kind: String, // audio | video
    pub duration_seconds: Option<i32>,
    #[serde(default)]
    pub captions: Vec<MediaCaptionCue>,
    #[serde(default)]
    pub chapters: Vec<MediaChapterMarker>,
    pub rights_ref: Option<String>,
}

fn validate_media_metadata(req: &mut MediaReq) -> ApiResult<()> {
    if req
        .duration_seconds
        .is_some_and(|seconds| !(1..=86_400).contains(&seconds))
    {
        return Err(ApiError::unprocessable(
            "invalid_media_duration",
            "duration_seconds must be between 1 and 86400",
        ));
    }
    let duration_ms = req.duration_seconds.map(|seconds| seconds as u64 * 1000);
    if req.captions.len() > 500
        || req.captions.iter().any(|cue| {
            cue.start_ms >= cue.end_ms
                || cue.end_ms > MAX_MEDIA_DURATION_MS
                || cue.text.trim().is_empty()
                || cue.text.len() > 2_000
                || cue.text.contains('\0')
                || duration_ms.is_some_and(|duration| cue.end_ms > duration)
        })
    {
        return Err(ApiError::unprocessable(
            "invalid_media_captions",
            "captions need non-empty text, valid increasing times, and at most 500 cues",
        ));
    }
    req.captions.sort_by_key(|cue| (cue.start_ms, cue.end_ms));

    req.chapters.sort_by_key(|chapter| chapter.at_ms);
    if req.chapters.len() > 100
        || req.chapters.iter().any(|chapter| {
            chapter.title.trim().is_empty()
                || chapter.title.len() > 200
                || chapter.at_ms >= MAX_MEDIA_DURATION_MS
                || duration_ms.is_some_and(|duration| chapter.at_ms >= duration)
        })
        || req
            .chapters
            .windows(2)
            .any(|pair| pair[0].at_ms == pair[1].at_ms)
    {
        return Err(ApiError::unprocessable(
            "invalid_media_chapters",
            "chapters need unique in-range times and non-empty titles, with at most 100 markers",
        ));
    }
    Ok(())
}

pub async fn attach_media(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(article_id): Path<Uuid>,
    Json(mut req): Json<MediaReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_permission(&user, provided, Permission::ContentAuthor)?;
    let url = req.url.trim().to_owned();
    let parsed_url = url::Url::parse(&url).ok();
    if parsed_url.as_ref().is_none_or(|parsed| {
        parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
    }) {
        return Err(ApiError::unprocessable(
            "invalid_url",
            "media url must be a credential-free https URL",
        ));
    }
    if !matches!(req.kind.as_str(), "audio" | "video") {
        return Err(ApiError::unprocessable(
            "invalid_media_kind",
            "kind must be audio or video",
        ));
    }
    validate_media_metadata(&mut req)?;
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
    let captions = serde_json::to_value(req.captions).map_err(|_| ApiError::internal())?;
    let chapters = serde_json::to_value(req.chapters).map_err(|_| ApiError::internal())?;
    sqlx::query!(
        "INSERT INTO media_assets
           (id, article_id, url, kind, duration_seconds, captions, chapters, rights_ref)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        id,
        article_id,
        url,
        req.kind,
        req.duration_seconds,
        captions,
        chapters,
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
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageCaseConceptsRequest.ts",
        rename = "ImageCaseConceptsRequest"
    )
)]
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
    state.require_permission(&user, provided, Permission::ContentAuthor)?;
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
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(case_id): Path<Uuid>,
) -> ApiResult<Json<ImageCaseConceptsResponse>> {
    let provided = headers
        .get("x-admin-token")
        .and_then(|value| value.to_str().ok());
    state.require_permission(&user, provided, Permission::ContentAuthor)?;
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
    state.require_permission(&user, provided, Permission::ContentAuthor)?;
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
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageAnnotationRequest.ts",
        rename = "ImageAnnotationRequest"
    )
)]
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
    state.require_permission(&user, provided, Permission::ContentAuthor)?;
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
    user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<AdminImageAnnotationListResponse>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_permission(&user, provided, Permission::ClinicalApprove)?;
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
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "image/ImageAnnotationReviewRequest.ts",
        rename = "ImageAnnotationReviewRequest"
    )
)]
pub struct ImageAnnotationReviewReq {
    #[cfg_attr(feature = "type-export", ts(type = "\"approved\" | \"rejected\""))]
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
    state.require_permission(&user, provided, Permission::ClinicalApprove)?;
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
