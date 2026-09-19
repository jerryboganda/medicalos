//! LIB-01/02/03: versioned library articles with search and citation
//! anchors. Published versions only for learners; the editorial console's
//! authoring surface arrives with ADMIN-06 iteration 2. Jurisdiction/date
//! overlays (LIB-04) and hybrid semantic search (LIB-02 full) follow in the
//! Phase 2 library slice — this is the versioned + text-searched base.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::json;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub async fn search(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    let pattern = format!("%{}%", q.get("q").map(String::as_str).unwrap_or(""));
    // Only the latest published version of each article is searchable.
    let rows = sqlx::query!(
        r#"SELECT a.id, a.slug, a.title, av.version, av.body
           FROM articles a
           JOIN article_versions av ON av.article_id = a.id AND av.status = 'published'
           WHERE (a.title ILIKE $1 OR av.body ILIKE $1)
             AND av.version = (
                 SELECT MAX(version) FROM article_versions
                 WHERE article_id = a.id AND status = 'published')
           ORDER BY a.title LIMIT 25"#,
        pattern
    )
    .fetch_all(&state.pool)
    .await?;
    let results: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "article_id": r.id,
                "slug": r.slug,
                "title": r.title,
                "version": r.version,
                // A body excerpt keeps the list honest about what matched.
                "excerpt": r.body.chars().take(200).collect::<String>(),
            })
        })
        .collect();
    Ok(Json(json!({ "results": results })))
}

pub async fn get_article(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(slug): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let article = sqlx::query!(
        r#"SELECT a.id, a.slug, a.title, av.version, av.body, av.source_ref
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
        "SELECT anchor, target FROM article_citations WHERE version_id = $1",
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
        "citations": citations.iter().map(|c| json!({
            "anchor": c.anchor, "target": c.target
        })).collect::<Vec<_>>(),
    })))
}
