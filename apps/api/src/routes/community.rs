//! COMMUNITY-01/02/03 + COMP-01/03/04 + GROW-01: moderated groups, private
//! duels, integrity-gated prizes, and share tokens. §16/§18.1: community
//! participation is strictly opt-in (profile + handle) and moderation lives
//! with the group's own moderators.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::routes::engagement::{CompetitionLeaderboardEntry, CompetitionLeaderboardResponse};
use crate::state::AppState;

fn valid_handle(handle: &str) -> bool {
    let h = handle.as_bytes();
    (3..=30).contains(&h.len())
        && h.iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

// ---- COMMUNITY-03: opt-in identity ------------------------------------------

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CommunityProfileRequest.ts",
        rename = "CommunityProfileRequest"
    )
)]
pub struct ProfileReq {
    pub handle: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CreateCommunityProfileResponse.ts",
        rename = "CreateCommunityProfileResponse"
    )
)]
pub struct CreateCommunityProfileResponse {
    pub handle: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CommunityProfileResponse.ts",
        rename = "CommunityProfileResponse"
    )
)]
pub struct CommunityProfileResponse {
    pub opted_in: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CommunityProfileByHandleResponse.ts",
        rename = "CommunityProfileByHandleResponse"
    )
)]
pub struct CommunityProfileByHandleResponse {
    pub user_id: Uuid,
    pub handle: String,
}

/// Creating a profile IS the opt-in. Nothing about a learner is shared
/// before this point, and leaderboards never show non-participants.
pub async fn create_profile(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<ProfileReq>,
) -> ApiResult<Json<CreateCommunityProfileResponse>> {
    let handle = req.handle.trim().to_lowercase();
    if !valid_handle(&handle) {
        return Err(ApiError::unprocessable(
            "invalid_handle",
            "handle must be 3-30 characters of a-z, 0-9, or '-'",
        ));
    }
    let taken = sqlx::query!(
        "SELECT 1 AS one FROM community_profiles WHERE handle = $1",
        handle
    )
    .fetch_optional(&state.pool)
    .await?;
    if taken.is_some() {
        return Err(ApiError::conflict("handle_taken", "that handle is in use"));
    }
    sqlx::query!(
        "INSERT INTO community_profiles (user_id, handle) VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET handle = EXCLUDED.handle",
        user.user_id,
        handle
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(CreateCommunityProfileResponse { handle }))
}

pub async fn my_profile(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<CommunityProfileResponse>> {
    let row = sqlx::query!(
        "SELECT handle FROM community_profiles WHERE user_id = $1",
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(Json(match row {
        Some(r) => CommunityProfileResponse {
            opted_in: true,
            handle: Some(r.handle),
        },
        None => CommunityProfileResponse {
            opted_in: false,
            handle: None,
        },
    }))
}

// ---- COMMUNITY-01: moderated groups -----------------------------------------

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CreateCommunityGroupRequest.ts",
        rename = "CreateCommunityGroupRequest"
    )
)]
pub struct GroupReq {
    pub name: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CreateCommunityGroupResponse.ts",
        rename = "CreateCommunityGroupResponse"
    )
)]
pub struct CreateCommunityGroupResponse {
    pub group_id: Uuid,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/JoinCommunityGroupResponse.ts",
        rename = "JoinCommunityGroupResponse"
    )
)]
pub struct JoinCommunityGroupResponse {
    pub joined: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CreateCommunityPostResponse.ts",
        rename = "CreateCommunityPostResponse"
    )
)]
pub struct CreateCommunityPostResponse {
    pub post_id: Uuid,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CommunityPost.ts",
        rename = "CommunityPost"
    )
)]
pub struct CommunityPost {
    pub post_id: Uuid,
    pub body: String,
    pub status: String,
    pub handle: String,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CommunityPostListResponse.ts",
        rename = "CommunityPostListResponse"
    )
)]
pub struct CommunityPostListResponse {
    pub posts: Vec<CommunityPost>,
    pub is_moderator: bool,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CreateCommunityPostRequest.ts",
        rename = "CreateCommunityPostRequest"
    )
)]
pub struct PostReq {
    pub body: String,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/ReportCommunityPostRequest.ts",
        rename = "ReportCommunityPostRequest"
    )
)]
#[serde(deny_unknown_fields)]
pub struct ReportPostReq {
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"spam\" | \"harassment\" | \"medical_misinformation\" | \"other\"")
    )]
    pub reason: String,
    pub note: Option<String>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/ReportCommunityPostResponse.ts",
        rename = "ReportCommunityPostResponse"
    )
)]
pub struct ReportCommunityPostResponse {
    pub report_id: Uuid,
    pub status: String,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CommunityPostReportSummary.ts",
        rename = "CommunityPostReportSummary"
    )
)]
pub struct CommunityPostReportSummary {
    pub report_id: Uuid,
    pub group_id: Uuid,
    pub group_name: String,
    pub post_id: Uuid,
    pub reason: String,
    pub status: String,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/MyCommunityPostReportsResponse.ts",
        rename = "MyCommunityPostReportsResponse"
    )
)]
pub struct MyCommunityPostReportsResponse {
    pub reports: Vec<CommunityPostReportSummary>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/GroupPostReport.ts",
        rename = "GroupPostReport"
    )
)]
pub struct GroupPostReport {
    pub report_id: Uuid,
    pub post_id: Uuid,
    pub reason: String,
    pub note: Option<String>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub post_body: String,
    pub author_handle: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/GroupPostReportQueueResponse.ts",
        rename = "GroupPostReportQueueResponse"
    )
)]
pub struct GroupPostReportQueueResponse {
    pub reports: Vec<GroupPostReport>,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/ResolveCommunityPostReportRequest.ts",
        rename = "ResolveCommunityPostReportRequest"
    )
)]
#[serde(deny_unknown_fields)]
pub struct ResolvePostReportReq {
    #[cfg_attr(feature = "type-export", ts(type = "\"dismiss\" | \"remove\""))]
    pub action: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/ResolveCommunityPostReportResponse.ts",
        rename = "ResolveCommunityPostReportResponse"
    )
)]
pub struct ResolveCommunityPostReportResponse {
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub resolved_reports: u64,
    pub status: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/RemoveCommunityPostResponse.ts",
        rename = "RemoveCommunityPostResponse"
    )
)]
pub struct RemoveCommunityPostResponse {
    pub removed: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CommunityGroupSummary.ts",
        rename = "CommunityGroupSummary"
    )
)]
pub struct CommunityGroupSummary {
    pub group_id: Uuid,
    pub name: String,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    pub members: i64,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CommunityGroupsResponse.ts",
        rename = "CommunityGroupsResponse"
    )
)]
pub struct CommunityGroupsResponse {
    pub groups: Vec<CommunityGroupSummary>,
}

pub async fn create_group(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<GroupReq>,
) -> ApiResult<Json<CreateCommunityGroupResponse>> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::unprocessable(
            "invalid_name",
            "group name must be 1-100 characters",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO community_groups (id, name, created_by) VALUES ($1, $2, $3)",
        id,
        name,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    sqlx::query!(
        "INSERT INTO community_group_members (group_id, user_id, role)
         VALUES ($1, $2, 'moderator')",
        id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(CreateCommunityGroupResponse { group_id: id }))
}

async fn require_member(state: &AppState, group_id: Uuid, user_id: Uuid) -> ApiResult<()> {
    sqlx::query!(
        "SELECT 1 AS one FROM community_group_members
         WHERE group_id = $1 AND user_id = $2",
        group_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::forbidden("membership_required", "join the group first"))?;
    Ok(())
}

pub async fn join_group(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> ApiResult<Json<JoinCommunityGroupResponse>> {
    let exists = sqlx::query!(
        "SELECT 1 AS one FROM community_groups WHERE id = $1",
        group_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("group_not_found"))?;
    let _ = exists;
    sqlx::query!(
        "INSERT INTO community_group_members (group_id, user_id, role)
         VALUES ($1, $2, 'member')
         ON CONFLICT (group_id, user_id) DO NOTHING",
        group_id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(JoinCommunityGroupResponse { joined: true }))
}

pub async fn create_post(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
    Json(req): Json<PostReq>,
) -> ApiResult<Json<CreateCommunityPostResponse>> {
    require_member(&state, group_id, user.user_id).await?;
    let body = req.body.trim();
    if body.is_empty() || body.len() > 2000 {
        return Err(ApiError::unprocessable(
            "invalid_body",
            "post body must be 1-2000 characters",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO community_posts (id, group_id, author, body) VALUES ($1, $2, $3, $4)",
        id,
        group_id,
        user.user_id,
        body
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(CreateCommunityPostResponse { post_id: id }))
}

/// COMMUNITY-01: members see visible posts only; moderation removes content,
/// it does not hide the fact that content was removed (honest tombstone).
pub async fn list_posts(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> ApiResult<Json<CommunityPostListResponse>> {
    require_member(&state, group_id, user.user_id).await?;
    let is_moderator = is_group_moderator(&state, group_id, user.user_id).await?;
    let rows = sqlx::query!(
        r#"SELECT p.id, p.body, p.status, p.created_at, cp.handle AS "handle!"
           FROM community_posts p
           JOIN community_profiles cp ON cp.user_id = p.author
           WHERE p.group_id = $1
           ORDER BY p.created_at DESC LIMIT 100"#,
        group_id
    )
    .fetch_all(&state.pool)
    .await?;
    // Authors without a community profile are shown as "member", never by
    // email or identity — COMMUNITY-03's non-coercive defaults run both ways.
    let anonymous = sqlx::query!(
        r#"SELECT p.id, p.body, p.status, p.created_at
           FROM community_posts p
           WHERE p.group_id = $1
             AND NOT EXISTS (
                 SELECT 1 FROM community_profiles cp WHERE cp.user_id = p.author)
           ORDER BY p.created_at DESC LIMIT 100"#,
        group_id
    )
    .fetch_all(&state.pool)
    .await?;
    let mut posts: Vec<CommunityPost> = rows
        .into_iter()
        .map(|r| CommunityPost {
            post_id: r.id,
            body: r.body,
            status: r.status,
            handle: r.handle,
            at: r.created_at,
        })
        .collect();
    posts.extend(anonymous.into_iter().map(|r| CommunityPost {
        post_id: r.id,
        body: r.body,
        status: r.status,
        handle: "member".into(),
        at: r.created_at,
    }));
    Ok(Json(CommunityPostListResponse {
        posts,
        is_moderator,
    }))
}

pub async fn report_post(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((group_id, post_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ReportPostReq>,
) -> ApiResult<(StatusCode, Json<ReportCommunityPostResponse>)> {
    require_member(&state, group_id, user.user_id).await?;
    if !matches!(
        req.reason.as_str(),
        "spam" | "harassment" | "medical_misinformation" | "other"
    ) {
        return Err(ApiError::unprocessable(
            "invalid_reason",
            "choose a listed report reason",
        ));
    }
    let note = req
        .note
        .map(|note| note.trim().to_owned())
        .filter(|note| !note.is_empty());
    if note.as_ref().is_some_and(|note| note.chars().count() > 500) {
        return Err(ApiError::unprocessable(
            "note_too_long",
            "report notes must be at most 500 characters",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let post_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM community_posts WHERE id = $1 AND group_id = $2 FOR UPDATE",
    )
    .bind(post_id)
    .bind(group_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("post_not_found"))?;
    if post_status != "visible" {
        return Err(ApiError::conflict(
            "post_not_reportable",
            "only visible posts can be reported",
        ));
    }

    let row = sqlx::query(
        "INSERT INTO community_post_reports (id, group_id, post_id, reporter_id, reason, note)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (post_id, reporter_id) DO NOTHING
         RETURNING id, status, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(group_id)
    .bind(post_id)
    .bind(user.user_id)
    .bind(req.reason)
    .bind(note)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::conflict("already_reported", "you already reported this post"))?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(ReportCommunityPostResponse {
            report_id: row.try_get("id")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
        }),
    ))
}

/// A reporter can see their own report state, without their private note or
/// any other member's identity or report details.
pub async fn my_post_reports(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<MyCommunityPostReportsResponse>> {
    let rows = sqlx::query(
        "SELECT r.id, r.group_id, g.name AS group_name, r.post_id, r.reason,
                r.status, r.created_at
         FROM community_post_reports r
         JOIN community_groups g ON g.id = r.group_id
         WHERE r.reporter_id = $1
         ORDER BY r.created_at DESC LIMIT 100",
    )
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;
    let reports = rows
        .iter()
        .map(|row| {
            Ok(CommunityPostReportSummary {
                report_id: row.try_get("id")?,
                group_id: row.try_get("group_id")?,
                group_name: row.try_get("group_name")?,
                post_id: row.try_get("post_id")?,
                reason: row.try_get("reason")?,
                status: row.try_get("status")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(Json(MyCommunityPostReportsResponse { reports }))
}

pub async fn group_report_queue(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> ApiResult<Json<GroupPostReportQueueResponse>> {
    require_moderator(&state, group_id, user.user_id).await?;
    let rows = sqlx::query(
        "SELECT r.id, r.post_id, r.reason, r.note, r.created_at, p.body,
                COALESCE(cp.handle, 'member') AS author_handle
         FROM community_post_reports r
         JOIN community_posts p ON p.id = r.post_id AND p.group_id = r.group_id
         LEFT JOIN community_profiles cp ON cp.user_id = p.author
         WHERE r.group_id = $1 AND r.status = 'open' AND p.status = 'visible'
         ORDER BY r.created_at ASC LIMIT 100",
    )
    .bind(group_id)
    .fetch_all(&state.pool)
    .await?;
    let reports = rows
        .iter()
        .map(|row| {
            Ok(GroupPostReport {
                report_id: row.try_get("id")?,
                post_id: row.try_get("post_id")?,
                reason: row.try_get("reason")?,
                note: row.try_get("note")?,
                created_at: row.try_get("created_at")?,
                post_body: row.try_get("body")?,
                author_handle: row.try_get("author_handle")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(Json(GroupPostReportQueueResponse { reports }))
}

async fn is_group_moderator(state: &AppState, group_id: Uuid, user_id: Uuid) -> ApiResult<bool> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM community_group_members
         WHERE group_id = $1 AND user_id = $2 AND role = 'moderator')",
    )
    .bind(group_id)
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?)
}

async fn require_moderator(state: &AppState, group_id: Uuid, user_id: Uuid) -> ApiResult<()> {
    if !is_group_moderator(state, group_id, user_id).await? {
        return Err(ApiError::forbidden(
            "moderator_required",
            "only group moderators can review reports",
        ));
    }
    Ok(())
}

async fn tombstone_post_and_resolve_reports(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    group_id: Uuid,
    post_id: Uuid,
    moderator_id: Uuid,
) -> ApiResult<u64> {
    sqlx::query(
        "UPDATE community_posts SET status = 'removed', removed_reason = 'moderator_removed'
         WHERE id = $1 AND group_id = $2",
    )
    .bind(post_id)
    .bind(group_id)
    .execute(&mut **tx)
    .await?;
    Ok(sqlx::query(
        "UPDATE community_post_reports
         SET status = 'post_removed', resolved_by = $3, resolved_at = now()
         WHERE post_id = $1 AND group_id = $2 AND status = 'open'",
    )
    .bind(post_id)
    .bind(group_id)
    .bind(moderator_id)
    .execute(&mut **tx)
    .await?
    .rows_affected())
}

pub async fn resolve_post_report(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((group_id, report_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ResolvePostReportReq>,
) -> ApiResult<Json<ResolveCommunityPostReportResponse>> {
    require_moderator(&state, group_id, user.user_id).await?;
    if !matches!(req.action.as_str(), "dismiss" | "remove") {
        return Err(ApiError::unprocessable(
            "invalid_action",
            "action must be dismiss or remove",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let report = sqlx::query(
        "SELECT r.post_id, r.status FROM community_post_reports r
         JOIN community_posts p ON p.id = r.post_id AND p.group_id = r.group_id
         WHERE r.id = $1 AND r.group_id = $2
         FOR UPDATE OF r, p",
    )
    .bind(report_id)
    .bind(group_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("report_not_found"))?;
    let post_id: Uuid = report.try_get("post_id")?;
    let status: String = report.try_get("status")?;
    if status != "open" {
        return Err(ApiError::conflict(
            "report_resolved",
            "this report is already resolved",
        ));
    }

    let resolved_reports = if req.action == "remove" {
        tombstone_post_and_resolve_reports(&mut tx, group_id, post_id, user.user_id).await?
    } else {
        sqlx::query(
            "UPDATE community_post_reports
             SET status = 'dismissed', resolved_by = $3, resolved_at = now()
             WHERE id = $1 AND group_id = $2 AND status = 'open'",
        )
        .bind(report_id)
        .bind(group_id)
        .bind(user.user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
    };
    sqlx::query(
        "INSERT INTO audit_events (id, actor, action, entity, entity_id, new_value)
         VALUES ($1, $2, 'community_post_report_resolved', 'community_post_report', $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user.user_id)
    .bind(report_id)
    .bind(json!({
        "decision": req.action.as_str(),
        "post_id": post_id,
        "resolved_reports": resolved_reports,
    }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(ResolveCommunityPostReportResponse {
        resolved_reports,
        status: if req.action == "remove" {
            "post_removed".into()
        } else {
            "dismissed".into()
        },
    }))
}

pub async fn remove_post(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((group_id, post_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<RemoveCommunityPostResponse>> {
    require_moderator(&state, group_id, user.user_id).await?;
    let mut tx = state.pool.begin().await?;
    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM community_posts WHERE id = $1 AND group_id = $2 FOR UPDATE",
    )
    .bind(post_id)
    .bind(group_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("post_not_found"))?;
    if status == "visible" {
        let resolved =
            tombstone_post_and_resolve_reports(&mut tx, group_id, post_id, user.user_id).await?;
        sqlx::query(
            "INSERT INTO audit_events (id, actor, action, entity, entity_id, new_value)
             VALUES ($1, $2, 'community_post_removed', 'community_post', $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(user.user_id)
        .bind(post_id)
        .bind(json!({ "group_id": group_id, "resolved_reports": resolved }))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(RemoveCommunityPostResponse { removed: true }))
}

// ---- COMP-03/GROW-01: private duels with share tokens ------------------------

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CreateDuelRequest.ts",
        rename = "CreateDuelRequest"
    )
)]
pub struct DuelReq {
    pub opponent: Uuid,
    pub exam_id: Uuid,
    pub chapter_id: Option<Uuid>,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    pub question_count: Option<i64>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/CreateDuelResponse.ts",
        rename = "CreateDuelResponse"
    )
)]
pub struct CreateDuelResponse {
    pub duel_id: Uuid,
    pub share_token: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/DuelByTokenResponse.ts",
        rename = "DuelByTokenResponse"
    )
)]
pub struct DuelByTokenResponse {
    pub duel_id: Uuid,
    pub status: String,
    pub question_count: i32,
    pub chapter: Option<String>,
    pub challenger: String,
    pub opponent: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/AcceptDuelResponse.ts",
        rename = "AcceptDuelResponse"
    )
)]
pub struct AcceptDuelResponse {
    pub accepted: bool,
    pub your_session_id: Uuid,
    pub question_count: usize,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "community/DuelSide.ts", rename = "DuelSide")
)]
pub struct DuelSide {
    pub user_id: Uuid,
    pub score: Option<i32>,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    pub total_ms: Option<i64>,
    pub session_id: Uuid,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/DuelStateResponse.ts",
        rename = "DuelStateResponse"
    )
)]
pub struct DuelStateResponse {
    pub status: String,
    pub winner: Option<Uuid>,
    pub sides: Vec<DuelSide>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/DeclineDuelResponse.ts",
        rename = "DeclineDuelResponse"
    )
)]
pub struct DeclineDuelResponse {
    pub declined: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "community/DuelSummary.ts", rename = "DuelSummary")
)]
pub struct DuelSummary {
    pub duel_id: Uuid,
    pub status: String,
    pub question_count: i32,
    pub winner: Option<Uuid>,
    pub sent_by_me: bool,
    pub challenger: String,
    pub opponent: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/MyDuelsResponse.ts",
        rename = "MyDuelsResponse"
    )
)]
pub struct MyDuelsResponse {
    pub duels: Vec<DuelSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/ShareCardKind.ts",
        rename = "ShareCardKind"
    )
)]
pub enum ShareCardKind {
    Score,
    Consistency,
    League,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "community/ShareCard.ts", rename = "ShareCard")
)]
pub struct ShareCard {
    pub kind: ShareCardKind,
    pub headline: String,
    pub subline: String,
    pub detail: String,
    pub share_text: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/UnavailableShareCard.ts",
        rename = "UnavailableShareCard"
    )
)]
pub struct UnavailableShareCard {
    pub kind: ShareCardKind,
    pub reason: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "community/ShareCardsResponse.ts",
        rename = "ShareCardsResponse"
    )
)]
pub struct ShareCardsResponse {
    pub cards: Vec<ShareCard>,
    pub unavailable: Vec<UnavailableShareCard>,
}

fn share_token() -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

pub async fn create_duel(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<DuelReq>,
) -> ApiResult<Json<CreateDuelResponse>> {
    if req.opponent == user.user_id {
        return Err(ApiError::unprocessable(
            "invalid_opponent",
            "you cannot duel yourself",
        ));
    }
    let count = req.question_count.unwrap_or(5);
    if !(3..=20).contains(&count) {
        return Err(ApiError::unprocessable(
            "invalid_question_count",
            "duels use 3-20 questions",
        ));
    }
    let opponent_exists = sqlx::query!("SELECT 1 AS one FROM users WHERE id = $1", req.opponent)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("user_not_found"))?;
    let _ = opponent_exists;
    let id = Uuid::new_v4();
    let token = share_token();
    sqlx::query!(
        "INSERT INTO duels (id, exam_id, challenger, opponent, chapter_id, question_count, share_token)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        id,
        req.exam_id,
        user.user_id,
        req.opponent,
        req.chapter_id,
        count as i32,
        token
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(CreateDuelResponse {
        duel_id: id,
        share_token: token,
    }))
}

/// GROW-01: the payload a deferred deep link resolves. Participation still
/// requires the opponent's explicit accept — the link never auto-enters.
pub async fn duel_by_token(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(token): Path<String>,
) -> ApiResult<Json<DuelByTokenResponse>> {
    let duel = sqlx::query!(
        r#"SELECT d.id, d.status, d.question_count, c.name AS "chapter?",
                  ch.handle AS "challenger_handle?", oh.handle AS "opponent_handle?"
           FROM duels d
           LEFT JOIN curriculum_nodes c ON c.id = d.chapter_id
           LEFT JOIN community_profiles ch ON ch.user_id = d.challenger
           LEFT JOIN community_profiles oh ON oh.user_id = d.opponent
           WHERE d.share_token = $1"#,
        token
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("duel_not_found"))?;
    Ok(Json(DuelByTokenResponse {
        duel_id: duel.id,
        status: duel.status,
        question_count: duel.question_count,
        chapter: duel.chapter,
        challenger: duel.challenger_handle.unwrap_or_else(|| "member".into()),
        opponent: duel.opponent_handle.unwrap_or_else(|| "member".into()),
    }))
}

async fn pick_duel_questions(
    state: &AppState,
    duel_id: Uuid,
    chapter_id: Option<Uuid>,
    count: i32,
) -> ApiResult<Vec<Uuid>> {
    let rows = sqlx::query!(
        r#"SELECT qv.id FROM question_versions qv
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE qv.status = 'published'
             AND c.exam_id = (SELECT exam_id FROM duels WHERE id = $1)
             AND ($2::uuid IS NULL OR qv.chapter_id = $2)
             AND NOT EXISTS (SELECT 1 FROM question_reports r
                             WHERE r.question_version_id = qv.id AND r.status = 'quarantined')
             AND NOT EXISTS (SELECT 1 FROM reserved_questions rq
                             WHERE rq.question_version_id = qv.id)
           ORDER BY random() LIMIT $3"#,
        duel_id,
        chapter_id,
        count as i64
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.id).collect())
}

pub async fn accept_duel(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(duel_id): Path<Uuid>,
) -> ApiResult<Json<AcceptDuelResponse>> {
    let duel = sqlx::query!(
        "SELECT id, challenger, opponent, status, chapter_id, question_count
         FROM duels WHERE id = $1",
        duel_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("duel_not_found"))?;
    if duel.opponent != user.user_id {
        return Err(ApiError::forbidden(
            "not_your_duel",
            "only the challenged opponent can accept",
        ));
    }
    if duel.status != "pending" {
        return Err(ApiError::conflict(
            "invalid_state",
            "this duel is no longer pending",
        ));
    }
    // Same selection rules for both sides: random published, non-reserved,
    // non-quarantined questions from the chapter (or exam-wide without one).
    let challenger_qs =
        pick_duel_questions(&state, duel.id, duel.chapter_id, duel.question_count).await?;
    let opponent_qs =
        pick_duel_questions(&state, duel.id, duel.chapter_id, duel.question_count).await?;
    if challenger_qs.is_empty() || opponent_qs.is_empty() {
        return Err(ApiError::unprocessable(
            "empty_pool",
            "no questions available for this duel",
        ));
    }
    let challenger_session = Uuid::new_v4();
    let opponent_session = Uuid::new_v4();
    for (sid, uid, qs) in [
        (challenger_session, duel.challenger, &challenger_qs),
        (opponent_session, duel.opponent, &opponent_qs),
    ] {
        sqlx::query!(
            "INSERT INTO practice_sessions (id, user_id, preset, chapter_id)
             VALUES ($1, $2, 'duel', $3)",
            sid,
            uid,
            duel.chapter_id
        )
        .execute(&state.pool)
        .await?;
        for (i, vid) in qs.iter().enumerate() {
            let idx = i as i16;
            sqlx::query!(
                "INSERT INTO session_items (id, session_id, item_index, question_version_id)
                 VALUES ($1, $2, $3, $4)",
                Uuid::new_v4(),
                sid,
                idx,
                vid
            )
            .execute(&state.pool)
            .await?;
        }
        sqlx::query!(
            "INSERT INTO duel_sessions (duel_id, user_id, session_id) VALUES ($1, $2, $3)",
            duel.id,
            uid,
            sid
        )
        .execute(&state.pool)
        .await?;
    }
    sqlx::query!("UPDATE duels SET status = 'active' WHERE id = $1", duel.id)
        .execute(&state.pool)
        .await?;
    Ok(Json(AcceptDuelResponse {
        accepted: true,
        your_session_id: opponent_session,
        question_count: opponent_qs.len(),
    }))
}

/// Called from the practice submit pipeline: score a duel participant and
/// settle the duel once both sides are in. Correct count wins; total time
/// breaks ties; a true draw stays a draw (winner NULL, never fabricated).
pub async fn on_session_submitted(state: &AppState, session_id: Uuid) -> ApiResult<()> {
    let duel = sqlx::query!(
        "SELECT duel_id, user_id FROM duel_sessions WHERE session_id = $1",
        session_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let Some(d) = duel else {
        return Ok(());
    };
    let score = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*) FILTER (WHERE correct), 0) AS "c!",
                  COALESCE(SUM(elapsed_ms), 0)::bigint AS "ms!"
           FROM attempts WHERE session_id = $1"#,
        session_id
    )
    .fetch_one(&state.pool)
    .await?;
    sqlx::query!(
        "UPDATE duel_sessions SET score = $3, total_ms = $4
         WHERE duel_id = $1 AND user_id = $2",
        d.duel_id,
        d.user_id,
        score.c as i32,
        score.ms
    )
    .execute(&state.pool)
    .await?;
    // The other side may not have submitted yet — scores are nullable here.
    let sides = sqlx::query!(
        "SELECT user_id, score, total_ms FROM duel_sessions WHERE duel_id = $1",
        d.duel_id
    )
    .fetch_all(&state.pool)
    .await?;
    if sides.len() == 2
        && sides
            .iter()
            .all(|s| s.score.is_some() && s.total_ms.is_some())
    {
        let (a, b) = (&sides[0], &sides[1]);
        let (a_score, b_score) = (a.score.unwrap(), b.score.unwrap());
        let (a_ms, b_ms) = (a.total_ms.unwrap(), b.total_ms.unwrap());
        let winner = match a_score.cmp(&b_score) {
            std::cmp::Ordering::Greater => Some(a.user_id),
            std::cmp::Ordering::Less => Some(b.user_id),
            std::cmp::Ordering::Equal => match a_ms.cmp(&b_ms) {
                std::cmp::Ordering::Greater => Some(b.user_id),
                std::cmp::Ordering::Less => Some(a.user_id),
                std::cmp::Ordering::Equal => None,
            },
        };
        sqlx::query!(
            "UPDATE duels SET status = 'done', winner = $2 WHERE id = $1",
            d.duel_id,
            winner
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(())
}

pub async fn duel_state(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(duel_id): Path<Uuid>,
) -> ApiResult<Json<DuelStateResponse>> {
    let duel = sqlx::query!(
        "SELECT status, challenger, opponent, winner, question_count FROM duels WHERE id = $1",
        duel_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("duel_not_found"))?;
    if duel.challenger != user.user_id && duel.opponent != user.user_id {
        return Err(ApiError::forbidden(
            "not_your_duel",
            "duel state is private to its participants",
        ));
    }
    let sides = sqlx::query!(
        "SELECT user_id, score, total_ms, session_id FROM duel_sessions WHERE duel_id = $1",
        duel_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(DuelStateResponse {
        status: duel.status,
        winner: duel.winner,
        sides: sides
            .into_iter()
            .map(|side| DuelSide {
                user_id: side.user_id,
                score: side.score,
                total_ms: side.total_ms,
                session_id: side.session_id,
            })
            .collect(),
    }))
}

pub async fn decline_duel(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(duel_id): Path<Uuid>,
) -> ApiResult<Json<DeclineDuelResponse>> {
    let updated = sqlx::query!(
        "UPDATE duels SET status = 'declined'
         WHERE id = $1 AND opponent = $2 AND status = 'pending'",
        duel_id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::not_found("duel_not_found"));
    }
    Ok(Json(DeclineDuelResponse { declined: true }))
}

// ---- COMP-01/04: competition leaderboard + integrity-gated prizes -----------

/// Ranked entries for one competition. Only opted-in handles are named;
/// flagged entries stay hidden pending review.
pub async fn competition_leaderboard(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(comp_id): Path<Uuid>,
) -> ApiResult<Json<CompetitionLeaderboardResponse>> {
    let comp =
        sqlx::query("SELECT prize_reviewed, status, ends_at FROM competitions WHERE id = $1")
            .bind(comp_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| ApiError::not_found("competition_not_found"))?;
    let prize_reviewed: bool = comp.try_get("prize_reviewed")?;
    let competition_status: String = comp.try_get("status")?;
    let ends_at: chrono::DateTime<chrono::Utc> = comp.try_get("ends_at")?;
    let prize_window_closed = competition_status == "closed" || chrono::Utc::now() >= ends_at;
    let rows = sqlx::query(
        r#"WITH ranked AS (
               SELECT ce.id, ce.user_id, ce.handle, ce.score, ce.total_time_ms,
                      ce.correct_count, ce.attempted_count,
                      ce.average_response_time_ms,
                      ROW_NUMBER() OVER (
                          ORDER BY ce.score DESC,
                              COALESCE(
                                  ce.correct_count::NUMERIC / NULLIF(ce.attempted_count, 0),
                                  0::NUMERIC
                              ) DESC,
                              ce.total_time_ms ASC,
                              ce.submitted_at ASC,
                              ce.id ASC
                      ) AS leaderboard_rank
               FROM competition_entries ce
               WHERE ce.competition_id = $1 AND ce.flagged = false
           )
           SELECT id, user_id, handle, score, total_time_ms,
                  correct_count, attempted_count, average_response_time_ms,
                  leaderboard_rank
           FROM ranked
           WHERE leaderboard_rank <= 50 OR user_id = $2
           ORDER BY leaderboard_rank"#,
    )
    .bind(comp_id)
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;
    let entries: Vec<CompetitionLeaderboardEntry> = rows
        .iter()
        .map(|row| {
            let user_id: Uuid = row.try_get("user_id")?;
            let correct_count: i64 = row.try_get("correct_count")?;
            let attempted_count: i64 = row.try_get("attempted_count")?;
            let accuracy = if attempted_count == 0 {
                0.0
            } else {
                correct_count as f64 / attempted_count as f64
            };
            let leaderboard_rank: i64 = row.try_get("leaderboard_rank")?;
            let handle: String = row.try_get("handle")?;
            let score: f32 = row.try_get("score")?;
            let total_time_ms: i64 = row.try_get("total_time_ms")?;
            let average_response_time_ms: f64 = row.try_get("average_response_time_ms")?;
            Ok(CompetitionLeaderboardEntry {
                rank: leaderboard_rank,
                handle,
                score,
                accuracy,
                questions_attempted: attempted_count,
                average_response_time_ms,
                total_time_ms,
                is_me: user_id == user.user_id,
                prize_eligible: prize_reviewed && prize_window_closed,
            })
        })
        .collect::<Result<_, sqlx::Error>>()?;
    Ok(Json(CompetitionLeaderboardResponse {
        prize_reviewed,
        status: competition_status,
        entries,
    }))
}

#[derive(Deserialize)]
pub struct PrizeReviewReq {
    pub flag: Option<Vec<String>>,
}

/// COMP-04: prizes only after integrity review. The admin marks the review
/// done (optionally flagging handles) — until then nobody can claim.
pub async fn prize_review(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(comp_id): Path<Uuid>,
    Json(req): Json<PrizeReviewReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    for handle in req.flag.unwrap_or_default() {
        sqlx::query!(
            "UPDATE competition_entries SET flagged = true
             WHERE competition_id = $1 AND handle = $2",
            comp_id,
            handle
        )
        .execute(&state.pool)
        .await?;
    }
    sqlx::query!(
        "UPDATE competitions SET prize_reviewed = true WHERE id = $1",
        comp_id
    )
    .execute(&state.pool)
    .await?;
    audit_pub(&state, user.user_id, comp_id).await?;
    Ok(Json(json!({ "prize_reviewed": true })))
}

async fn audit_pub(state: &AppState, actor: Uuid, comp_id: Uuid) -> ApiResult<()> {
    sqlx::query!(
        "INSERT INTO audit_events (id, actor, action, entity, entity_id, new_value)
         VALUES ($1, $2, 'competition_prize_reviewed', 'competition', $3, $4)",
        Uuid::new_v4(),
        actor,
        comp_id,
        json!({})
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}

/// A participant's prize claim: honest outcome only — eligibility requires
/// a submitted entry, a closed competition, completed review, no flag.
pub async fn claim_prize(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(comp_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let entry = sqlx::query!(
        "SELECT flagged, handle FROM competition_entries
         WHERE competition_id = $1 AND user_id = $2",
        comp_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("entry_not_found"))?;
    let comp = sqlx::query!(
        "SELECT prize_reviewed, status, ends_at FROM competitions WHERE id = $1",
        comp_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("competition_not_found"))?;
    let deny = |code: &'static str, msg: &str| {
        Err(ApiError::forbidden(code, msg)) as ApiResult<Json<serde_json::Value>>
    };
    if comp.status != "closed" && chrono::Utc::now() < comp.ends_at {
        return deny("competition_open", "the competition has not closed yet");
    }
    if !comp.prize_reviewed {
        return deny("prize_review_pending", "integrity review has not completed");
    }
    if entry.flagged {
        return deny("entry_flagged", "this entry is under integrity review");
    }
    Ok(Json(json!({
        "prize_claimable": true,
        "handle": entry.handle,
    })))
}

/// Resolve a handle to a duel opponent. Opt-in info only: the handle exists
/// publicly precisely so duelists can find each other; nothing else leaks.
pub async fn profile_by_handle(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(handle): Path<String>,
) -> ApiResult<Json<CommunityProfileByHandleResponse>> {
    let row = sqlx::query!(
        "SELECT user_id, handle FROM community_profiles WHERE handle = $1",
        handle.to_lowercase()
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("profile_not_found"))?;
    Ok(Json(CommunityProfileByHandleResponse {
        user_id: row.user_id,
        handle: row.handle,
    }))
}

/// My duels: challenges I sent or received, newest first. The client renders
/// accept/decline on pending ones it received and state on the rest.
pub async fn my_duels(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<MyDuelsResponse>> {
    let rows = sqlx::query!(
        r#"SELECT d.id, d.status, d.question_count, d.winner AS "winner?",
                  d.challenger = $1 AS "mine_sent!",
                  ch.handle AS "challenger_handle?", oh.handle AS "opponent_handle?"
           FROM duels d
           LEFT JOIN community_profiles ch ON ch.user_id = d.challenger
           LEFT JOIN community_profiles oh ON oh.user_id = d.opponent
           WHERE d.challenger = $1 OR d.opponent = $1
           ORDER BY d.created_at DESC LIMIT 50"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(MyDuelsResponse {
        duels: rows
            .into_iter()
            .map(|row| DuelSummary {
                duel_id: row.id,
                status: row.status,
                question_count: row.question_count,
                winner: row.winner,
                sent_by_me: row.mine_sent,
                challenger: row.challenger_handle.unwrap_or_else(|| "member".into()),
                opponent: row.opponent_handle.unwrap_or_else(|| "member".into()),
            })
            .collect(),
    }))
}

/// Group discovery: id and name only. Content stays members-only.
pub async fn list_groups(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<CommunityGroupsResponse>> {
    let rows = sqlx::query!(
        r#"SELECT g.id, g.name,
                  (SELECT COUNT(*) FROM community_group_members m
                   WHERE m.group_id = g.id) AS "members!"
           FROM community_groups g ORDER BY g.created_at DESC LIMIT 100"#
    )
    .fetch_all(&state.pool)
    .await?;
    let groups = rows
        .into_iter()
        .map(|row| CommunityGroupSummary {
            group_id: row.id,
            name: row.name,
            members: row.members,
        })
        .collect();
    Ok(Json(CommunityGroupsResponse { groups }))
}

// ---- GROW-01: share cards ----------------------------------------------------
// Score, consistency, and league cards built only from real records (§26.1).
// Cards never carry question content; unavailable cards state the honest
// reason instead of inventing numbers (TRUST-01).

pub async fn share_cards(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<ShareCardsResponse>> {
    let min_sample = crate::routes::settings::current_bounded_i64(
        &state.pool,
        "community_min_sample",
        state.community_min_sample,
        1,
        1_000_000,
    )
    .await?;
    let mut cards = Vec::new();
    let mut unavailable = Vec::new();

    // Score: real answered attempts over the last 30 days, gated like the
    // community statistics (QB-15) so no small-sample number is shareable.
    // Nothing for speed (§17): only accuracy is reported.
    let score = sqlx::query!(
        r#"SELECT COUNT(*) AS "total!",
                  COALESCE(SUM(CASE WHEN a.correct THEN 1 ELSE 0 END), 0) AS "correct!"
           FROM attempts a
           WHERE a.user_id = $1
             AND a.chosen_index IS NOT NULL
             AND a.created_at >= now() - interval '30 days'"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    if score.total >= min_sample {
        let percent = (score.correct * 100) / score.total;
        cards.push(ShareCard {
            kind: ShareCardKind::Score,
            headline: format!("{percent}%"),
            subline: "accuracy over the last 30 days".into(),
            detail: format!("{} of {} answered correctly", score.correct, score.total),
            share_text: format!(
                "My 30-day accuracy on Medical Learning OS: {percent}% ({} of {} questions answered correctly).",
                score.correct, score.total
            ),
        });
    } else {
        unavailable.push(UnavailableShareCard {
            kind: ShareCardKind::Score,
            reason: format!(
                "a shareable accuracy needs at least {min_sample} answered questions in the last 30 days"
            ),
        });
    }

    // Consistency: today's streak and last week's met goals (ENG-01 records).
    let consistency = sqlx::query!(
        r#"SELECT
               (SELECT streak_count FROM engagement_days
                WHERE user_id = $1 AND day = CURRENT_DATE) AS "streak?",
               (SELECT COUNT(*) FROM engagement_days
                WHERE user_id = $1 AND day >= CURRENT_DATE - interval '6 days'
                  AND goal_met) AS "met_week!"
        "#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    match consistency.streak {
        Some(streak) if streak > 0 => {
            cards.push(ShareCard {
                kind: ShareCardKind::Consistency,
                headline: format!("{streak}-day streak"),
                subline: "daily practice".into(),
                detail: format!("goal met on {} of the last 7 days", consistency.met_week),
                share_text: format!(
                    "My practice streak on Medical Learning OS: {streak} days in a row (goal met {} of the last 7 days).",
                    consistency.met_week
                ),
            });
        }
        _ => unavailable.push(UnavailableShareCard {
            kind: ShareCardKind::Consistency,
            reason: "no active streak — answer a question today to start one".into(),
        }),
    }

    // League: the learner's most recent competition entry, ranked by the same
    // ordering the leaderboard publishes (score desc, time asc).
    let entry = sqlx::query!(
        r#"SELECT ce.competition_id, ce.handle, ce.score, ce.total_time_ms, c.title
           FROM competition_entries ce
           JOIN competitions c ON c.id = ce.competition_id
           WHERE ce.user_id = $1
           ORDER BY ce.submitted_at DESC
           LIMIT 1"#,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    match entry {
        Some(e) => {
            let rank = sqlx::query_scalar!(
                r#"SELECT COUNT(*) + 1 AS "rank!"
                   FROM competition_entries
                   WHERE competition_id = $1
                     AND (score > $2 OR (score = $2 AND total_time_ms < $3))"#,
                e.competition_id,
                e.score,
                e.total_time_ms
            )
            .fetch_one(&state.pool)
            .await?;
            cards.push(ShareCard {
                kind: ShareCardKind::League,
                headline: format!("Rank {rank}"),
                subline: format!("{} — {}", e.handle, e.title),
                detail: format!("score {} in the latest competition", e.score),
                share_text: format!(
                    "{} finished rank {rank} in the {} competition on Medical Learning OS with a score of {}.",
                    e.handle, e.title, e.score
                ),
            });
        }
        None => unavailable.push(UnavailableShareCard {
            kind: ShareCardKind::League,
            reason: "no competition entries yet — join a competition to share a rank".into(),
        }),
    }

    Ok(Json(ShareCardsResponse { cards, unavailable }))
}
