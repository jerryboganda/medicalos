//! LIB-01: authenticated, read-only versioned knowledge library.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use chrono::NaiveDate;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Serialize)]
pub struct LibraryItemView {
    item_id: Uuid,
    kind: String,
    version: i32,
    title: String,
    provenance_class: String,
    source_label: String,
    source_url: Option<String>,
    effective_date: NaiveDate,
    jurisdiction: String,
}

#[derive(Serialize)]
pub struct LibraryListResponse {
    items: Vec<LibraryItemView>,
}

#[derive(Serialize)]
pub struct LibraryVersionView {
    item_id: Uuid,
    kind: String,
    version: i32,
    title: String,
    body: String,
    provenance_class: String,
    source_label: String,
    source_url: Option<String>,
    effective_date: NaiveDate,
    jurisdiction: String,
    concept_version_ids: Vec<Uuid>,
    question_version_ids: Vec<Uuid>,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<LibraryListResponse>> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            i32,
            String,
            String,
            String,
            Option<String>,
            NaiveDate,
            String,
        ),
    >(
        r#"SELECT item_id, kind, version, title, provenance_class, source_label,
                  source_url, effective_date, jurisdiction
           FROM (
               SELECT li.id AS item_id, li.kind, lv.version, lv.title,
                      lv.provenance_class, lv.source_label, lv.source_url,
                      lv.effective_date, lv.jurisdiction,
                      row_number() OVER (
                          PARTITION BY li.id ORDER BY lv.version DESC
                      ) AS rank
               FROM library_items li
               JOIN library_versions lv ON lv.library_item_id = li.id
               WHERE lv.status = 'published'
           ) latest
           WHERE rank = 1
           ORDER BY title, item_id"#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(LibraryListResponse {
        items: rows
            .into_iter()
            .map(
                |(
                    item_id,
                    kind,
                    version,
                    title,
                    provenance_class,
                    source_label,
                    source_url,
                    effective_date,
                    jurisdiction,
                )| LibraryItemView {
                    item_id,
                    kind,
                    version,
                    title,
                    provenance_class,
                    source_label,
                    source_url,
                    effective_date,
                    jurisdiction,
                },
            )
            .collect(),
    }))
}

pub async fn version(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path((item_id, version)): Path<(Uuid, i32)>,
) -> ApiResult<Json<LibraryVersionView>> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            Uuid,
            i32,
            String,
            String,
            String,
            String,
            Option<String>,
            NaiveDate,
            String,
        ),
    >(
        r#"SELECT li.id, li.kind, lv.id, lv.version, lv.title, lv.body,
                  lv.provenance_class, lv.source_label, lv.source_url,
                  lv.effective_date, lv.jurisdiction
           FROM library_items li
           JOIN library_versions lv ON lv.library_item_id = li.id
           WHERE li.id = $1 AND lv.version = $2 AND lv.status = 'published'"#,
    )
    .bind(item_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("library_version_not_found"))?;

    let concept_version_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT concept_version_id FROM library_version_concepts
         WHERE library_version_id = $1 ORDER BY concept_version_id",
    )
    .bind(row.2)
    .fetch_all(&state.pool)
    .await?;
    let question_version_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT question_version_id FROM library_version_questions
         WHERE library_version_id = $1 ORDER BY question_version_id",
    )
    .bind(row.2)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(LibraryVersionView {
        item_id: row.0,
        kind: row.1,
        version: row.3,
        title: row.4,
        body: row.5,
        provenance_class: row.6,
        source_label: row.7,
        source_url: row.8,
        effective_date: row.9,
        jurisdiction: row.10,
        concept_version_ids,
        question_version_ids,
    }))
}
