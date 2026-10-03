//! NOTE-01/02/03: source-linked private notes with backlinks, collections,
//! and portable JSON export. Notes are private by construction — every query
//! is user-scoped — and the agent never rewrites them (§12.5): it only
//! proposes, which is out of scope for this module.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "notes/NoteRequest.ts", rename = "NoteRequest")
)]
pub struct NoteReq {
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub title: Option<String>,
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub body: Option<String>,
    #[cfg_attr(feature = "type-export", ts(type = "string", optional = nullable))]
    pub source_question_version_id: Option<Uuid>,
    /// OFF-03: offline edits carry the updated_at they were based on. When
    /// it is stale, the update is refused with the server version instead of
    /// silently overwriting newer changes.
    #[cfg_attr(feature = "type-export", ts(type = "string", optional = nullable))]
    pub base_updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "notes/NoteBacklink.ts", rename = "NoteBacklink")
)]
pub struct NoteBacklink {
    note_id: Uuid,
    title: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "notes/Note.ts", rename = "Note")
)]
pub struct Note {
    note_id: Uuid,
    title: String,
    body: String,
    source_question_version_id: Option<Uuid>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    updated_at: chrono::DateTime<chrono::Utc>,
    backlinks: Vec<NoteBacklink>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "notes/CreateNoteResponse.ts",
        rename = "CreateNoteResponse"
    )
)]
pub struct CreateNoteResponse {
    note_id: Uuid,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "notes/UpdateNoteResponse.ts",
        rename = "UpdateNoteResponse"
    )
)]
pub struct UpdateNoteResponse {
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "notes/ListNotesResponse.ts",
        rename = "ListNotesResponse"
    )
)]
pub struct ListNotesResponse {
    notes: Vec<Note>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "notes/DeleteNoteResponse.ts",
        rename = "DeleteNoteResponse"
    )
)]
pub struct DeleteNoteResponse {
    #[cfg_attr(feature = "type-export", ts(type = "true"))]
    deleted: bool,
}

pub async fn create_note(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<NoteReq>,
) -> ApiResult<Json<CreateNoteResponse>> {
    let id = Uuid::new_v4();
    let created = sqlx::query!(
        "INSERT INTO notes (id, user_id, title, body, source_question_version_id)
         VALUES ($1, $2, $3, $4, $5) RETURNING updated_at",
        id,
        user.user_id,
        req.title.unwrap_or_default(),
        req.body.unwrap_or_default(),
        req.source_question_version_id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(CreateNoteResponse {
        note_id: id,
        updated_at: created.updated_at,
    }))
}

pub async fn update_note(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(note_id): Path<Uuid>,
    Json(req): Json<NoteReq>,
) -> ApiResult<Json<UpdateNoteResponse>> {
    // OFF-03: versioned note resolution — a stale base refuses with the
    // server's current version; the client then merges or force-writes.
    if let Some(base) = req.base_updated_at {
        let current = sqlx::query!(
            "SELECT updated_at FROM notes WHERE id = $1 AND user_id = $2",
            note_id,
            user.user_id
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("note_not_found"))?;
        if current.updated_at > base {
            return Err(ApiError::conflict_with_details(
                "note_conflict",
                "This note changed on another device. Merge your edit with the stored version.",
                serde_json::json!({ "server_updated_at": current.updated_at }),
            ));
        }
    }
    let result = sqlx::query!(
        "UPDATE notes SET
            title = COALESCE($3, title),
            body = COALESCE($4, body),
            updated_at = now()
         WHERE id = $1 AND user_id = $2
         RETURNING updated_at",
        note_id,
        user.user_id,
        req.title.as_deref(),
        req.body.as_deref()
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("note_not_found"))?;
    Ok(Json(UpdateNoteResponse {
        updated_at: result.updated_at,
    }))
}

pub async fn delete_note(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(note_id): Path<Uuid>,
) -> ApiResult<Json<DeleteNoteResponse>> {
    let deleted = sqlx::query!(
        "DELETE FROM notes WHERE id = $1 AND user_id = $2",
        note_id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    if deleted.rows_affected() == 0 {
        return Err(ApiError::not_found("note_not_found"));
    }
    Ok(Json(DeleteNoteResponse { deleted: true }))
}

pub async fn list_notes(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<ListNotesResponse>> {
    let rows = sqlx::query!(
        r#"SELECT n.id, n.title, n.body, n.source_question_version_id,
                  n.created_at, n.updated_at,
                  COALESCE(json_agg(json_build_object(
                      'note_id', l.to_note_id, 'title', other.title
                  )) FILTER (
                      WHERE l.to_note_id IS NOT NULL
                        AND (other.source_question_version_id IS NULL OR EXISTS (
                            SELECT 1 FROM question_versions linked_qv
                            WHERE linked_qv.id = other.source_question_version_id
                              AND linked_qv.status = 'published'
                              AND question_display_rights_active(
                                  linked_qv.rights_ref, linked_qv.source_ref,
                                  linked_qv.source_refs, linked_qv.media_refs
                              )
                        ))
                  ), '[]') AS backlinks
           FROM notes n
           LEFT JOIN note_links l ON l.from_note_id = n.id
           LEFT JOIN notes other ON other.id = l.to_note_id
           WHERE n.user_id = $1
             AND (n.source_question_version_id IS NULL OR EXISTS (
                 SELECT 1 FROM question_versions qv
                 WHERE qv.id = n.source_question_version_id
                   AND qv.status = 'published'
                   AND question_display_rights_active(
                       qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs
                   )
             ))
           GROUP BY n.id
           ORDER BY n.updated_at DESC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let notes = rows
        .into_iter()
        .map(|r| {
            Ok(Note {
                note_id: r.id,
                title: r.title,
                body: r.body,
                source_question_version_id: r.source_question_version_id,
                updated_at: r.updated_at,
                backlinks: serde_json::from_value(
                    r.backlinks.unwrap_or_else(|| serde_json::json!([])),
                )
                .map_err(|_| ApiError::internal())?,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(ListNotesResponse { notes }))
}

#[derive(Deserialize)]
pub struct LinkReq {
    pub from_note_id: Uuid,
    pub to_note_id: Uuid,
}

pub async fn link_notes(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<LinkReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.from_note_id == req.to_note_id {
        return Err(ApiError::unprocessable(
            "self_link",
            "a note cannot link to itself",
        ));
    }
    // Both notes must belong to the linker — backlinks are private context.
    let owns = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM notes
           WHERE user_id = $1 AND id IN ($2, $3)"#,
        user.user_id,
        req.from_note_id,
        req.to_note_id
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    if owns != 2 {
        return Err(ApiError::not_found("note_not_found"));
    }
    let link_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO note_links (id, from_note_id, to_note_id) VALUES ($1, $2, $3)
         ON CONFLICT DO NOTHING",
        link_id,
        req.from_note_id,
        req.to_note_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "linked": true })))
}

/// NOTE-03: portable export of the learner's own notes as JSON — human
/// readable, complete, and free of any other learner's content by
/// construction (the query is user-scoped).
pub async fn export_notes(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT n.title, n.body, n.created_at,
                  qv.source_ref AS "source_reference?"
           FROM notes n
           LEFT JOIN question_versions qv
             ON qv.id = n.source_question_version_id
           WHERE n.user_id = $1
             AND (n.source_question_version_id IS NULL OR (
                 qv.status = 'published'
                 AND question_display_rights_active(
                     qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs
                 )
             ))
           ORDER BY n.created_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let notes: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "title": r.title,
                "body": r.body,
                "source_reference": r.source_reference,
                "created_at": r.created_at,
            })
        })
        .collect();
    Ok(Json(
        json!({ "exported_at": chrono::Utc::now(), "notes": notes }),
    ))
}
