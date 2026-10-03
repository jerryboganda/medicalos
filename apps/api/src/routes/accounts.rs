//! CORE-07: learner-controlled devices and account lifecycle.

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{hash_password, AccountRecoveryUser, AuthSession, AuthUser};
use crate::error::{ApiError, ApiResult};
use crate::routes::admin::{admin_headers, audit};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct DeviceReq {
    pub device_key: String,
    pub label: Option<String>,
}

pub async fn register_device(
    State(state): State<Arc<AppState>>,
    user: AccountRecoveryUser,
    Extension(session): Extension<AuthSession>,
    Json(req): Json<DeviceReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let key = req.device_key.trim();
    if key.is_empty() || key.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_device_key",
            "device_key must be 1-200 characters",
        ));
    }
    let label = req
        .label
        .as_deref()
        .unwrap_or("device")
        .trim()
        .chars()
        .take(100)
        .collect::<String>();
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;

    let auth_session = sqlx::query!(
        "SELECT device_id, created_at FROM auth_sessions
         WHERE token_hash = $1 AND user_id = $2 AND expires_at > now()
           AND revoked_at IS NULL FOR UPDATE",
        &session.token_hash,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;

    let existing = sqlx::query!(
        "SELECT id, revoked_at FROM user_devices
         WHERE user_id = $1 AND device_key = $2 FOR UPDATE",
        user.user_id,
        key
    )
    .fetch_optional(&mut *tx)
    .await?;

    // A revoked row can be reactivated only by a session created after the
    // revocation. Keep this check ahead of the limit check so a legacy bearer
    // cannot obscure the reason its device registration was refused.
    if existing
        .as_ref()
        .and_then(|row| row.revoked_at.as_ref())
        .is_some_and(|revoked_at| auth_session.created_at <= *revoked_at)
    {
        return Err(ApiError::conflict(
            "device_revoked",
            "sign in again before registering this revoked device",
        ));
    }

    // CORE-07: reactivating a revoked device consumes an active-device slot.
    if existing.is_none()
        || existing
            .as_ref()
            .is_some_and(|row| row.revoked_at.is_some())
    {
        let limit = sqlx::query_scalar!(
            r#"SELECT max_devices AS "max_devices!" FROM users WHERE id = $1"#,
            user.user_id
        )
        .fetch_one(&mut *tx)
        .await?;
        let active = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "active!" FROM user_devices
               WHERE user_id = $1 AND revoked_at IS NULL"#,
            user.user_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if active >= i64::from(limit) {
            return Err(ApiError::forbidden_with_details(
                "devices_exhausted",
                format!("device limit reached ({limit}) - revoke a device first"),
                serde_json::json!({ "device_limit": { "limit": limit, "active": active } }),
            ));
        }
    }

    if auth_session
        .device_id
        .as_deref()
        .is_some_and(|bound| bound != key)
    {
        return Err(ApiError::conflict(
            "device_session_conflict",
            "this session is already bound to another device",
        ));
    }
    let row = sqlx::query!(
        r#"INSERT INTO user_devices (id, user_id, device_key, label)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (user_id, device_key) DO UPDATE
             SET label = EXCLUDED.label, last_seen_at = now(), revoked_at = NULL
           RETURNING id, device_key, label, created_at, last_seen_at"#,
        Uuid::new_v4(),
        user.user_id,
        key,
        label
    )
    .fetch_one(&mut *tx)
    .await?;

    if auth_session.device_id.is_none() {
        let bound = sqlx::query!(
            "UPDATE auth_sessions SET device_id = $1
             WHERE token_hash = $2 AND user_id = $3 AND device_id IS NULL
               AND expires_at > now() AND revoked_at IS NULL",
            key,
            &session.token_hash,
            user.user_id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if bound != 1 {
            return Err(ApiError::unauthorized());
        }
    }

    tx.commit().await?;
    Ok(Json(json!({
        "device_id": row.id,
        "device_key": row.device_key,
        "label": row.label,
        "created_at": row.created_at,
        "last_seen_at": row.last_seen_at,
    })))
}

pub async fn list_devices(
    State(state): State<Arc<AppState>>,
    user: AccountRecoveryUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT id, device_key, label, created_at, last_seen_at, revoked_at
           FROM user_devices
           WHERE user_id = $1
           ORDER BY last_seen_at DESC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let devices: Vec<_> = rows
        .into_iter()
        .map(|r| {
            json!({
                "device_id": r.id,
                "device_key": r.device_key,
                "label": r.label,
                "created_at": r.created_at,
                "last_seen_at": r.last_seen_at,
                "revoked_at": r.revoked_at,
            })
        })
        .collect();
    Ok(Json(json!({ "devices": devices })))
}

pub async fn revoke_device(
    State(state): State<Arc<AppState>>,
    user: AccountRecoveryUser,
    Path(device_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    let row = sqlx::query!(
        "UPDATE user_devices SET revoked_at = now()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
         RETURNING device_key",
        device_id,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("device_not_found"))?;
    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE user_id = $1 AND device_id = $2 AND revoked_at IS NULL",
        user.user_id,
        row.device_key
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "revoked": true })))
}

pub async fn delete_account(
    State(state): State<Arc<AppState>>,
    user: AccountRecoveryUser,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now() WHERE user_id = $1",
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE user_devices SET revoked_at = now() WHERE user_id = $1",
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE users SET deleted_at = now() WHERE id = $1",
        user.user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "deleted": true })))
}

// ---- CORE-07: the single-active-session policy toggle ------------------------

#[derive(Deserialize)]
pub struct SessionPolicyReq {
    pub single_active_session: bool,
}

pub async fn get_session_policy(
    State(state): State<Arc<AppState>>,
    user: AccountRecoveryUser,
) -> ApiResult<Json<serde_json::Value>> {
    let single_active_session = sqlx::query_scalar!(
        r#"SELECT single_active_session AS "single_active_session!"
           FROM users WHERE id = $1 AND deleted_at IS NULL"#,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    Ok(Json(
        json!({ "single_active_session": single_active_session }),
    ))
}

/// When on, the next login retires every prior session for this account.
pub async fn set_session_policy(
    State(state): State<Arc<AppState>>,
    user: AccountRecoveryUser,
    Json(req): Json<SessionPolicyReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    sqlx::query!(
        "UPDATE users SET single_active_session = $2 WHERE id = $1 AND deleted_at IS NULL",
        user.user_id,
        req.single_active_session
    )
    .execute(&mut *tx)
    .await?;
    if req.single_active_session {
        // Turning the policy on takes effect immediately for this account.
        sqlx::query!(
            "UPDATE auth_sessions SET revoked_at = now()
             WHERE user_id = $1 AND revoked_at IS NULL",
            user.user_id
        )
        .execute(&mut *tx)
        .await?;
    }
    audit(
        &mut *tx,
        user.user_id,
        "single_active_session_policy_changed",
        "user",
        user.user_id,
        json!({ "single_active_session": req.single_active_session }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(
        json!({ "single_active_session": req.single_active_session }),
    ))
}

#[derive(Deserialize)]
pub struct AdminResetPasswordReq {
    pub new_password: String,
}

/// Operator-assisted password reset (pilot stop-gap until an email
/// delivery seam exists — see .scratch/auth-hardening/spec.md). Admin-gated
/// through the shared seam (role session or legacy token), hashes the new
/// password like registration does, and revokes every live session so a
/// stolen session does not survive the reset. Audited with the target,
/// never with the password.
pub async fn admin_reset_password(
    State(state): State<Arc<AppState>>,
    operator: AuthUser,
    headers: axum::http::HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(req): Json<AdminResetPasswordReq>,
) -> ApiResult<Json<serde_json::Value>> {
    state.require_admin(&operator, admin_headers(&headers))?;
    if req.new_password.len() < 8 {
        return Err(ApiError::unprocessable(
            "weak_password",
            "password must be at least 8 characters",
        ));
    }
    let hash = hash_password(&req.new_password)?;
    let mut tx = state.pool.begin().await?;
    let updated = sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2 AND deleted_at IS NULL",
        hash,
        user_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(ApiError::not_found("user_not_found"));
    }
    sqlx::query!(
        "UPDATE auth_sessions SET revoked_at = now()
         WHERE user_id = $1 AND revoked_at IS NULL",
        user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    audit(
        &state.pool,
        operator.user_id,
        "admin_password_reset",
        "user",
        user_id,
        json!({ "sessions_revoked": true }),
    )
    .await?;
    Ok(Json(json!({ "reset": true })))
}

const MAX_ACCOUNT_EXPORT_ROWS: i64 = 50_000;
const MAX_ACCOUNT_EXPORT_BYTES: usize = 8 * 1024 * 1024;

// Render source rows for the preflight byte estimate so TOASTed text cannot be
// mistaken for its small on-row pointer. These fields are never returned.
const ACCOUNT_EXPORT_SIZE_SQL: &str = r#"
SELECT COUNT(*)::BIGINT, COALESCE(SUM(row_bytes), 0)::BIGINT
FROM (
    SELECT octet_length(to_jsonb(ROW(u.id, u.email, u.tier, u.idp_subject,
      u.single_active_session, u.max_devices, u.created_at))::text)::BIGINT AS row_bytes
      FROM users u WHERE u.id = $1 AND u.deleted_at IS NULL
    UNION ALL
    SELECT octet_length(to_jsonb(r)::text)::BIGINT AS row_bytes FROM (
      SELECT id, user_id, preset, chapter_id, source_session_id, mock_id, form_id, ai_allowed,
        time_limit_seconds, per_question_seconds, deadline, plan_task_key, late_sync_grace_seconds,
        away_timeout_seconds, integrity_policy, auto_submitted_by_policy, status, created_at,
        submitted_at FROM practice_sessions WHERE user_id = $1
    ) r
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM session_items r JOIN practice_sessions s ON s.id = r.session_id WHERE s.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM attempts r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM learner_concept_state r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM mock_attempts r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM question_marks r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM (
      SELECT r.id, r.question_version_id, r.category,
        CASE WHEN EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = r.question_version_id
          AND qv.status = 'published' AND question_display_rights_active(
            qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN r.note ELSE NULL END AS note,
        CASE WHEN EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = r.question_version_id
          AND qv.status = 'published' AND question_display_rights_active(
            qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs))
          THEN 'available' ELSE 'withheld_rights_inactive' END AS content_disposition,
        r.status, r.resolved_at, r.created_at FROM question_reports r WHERE r.reporter_id = $1
    ) r
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM retest_cards r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM retest_history r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM decks r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM cards r
      WHERE r.user_id = $1 AND (r.source_question_version_id IS NULL OR EXISTS (
        SELECT 1 FROM question_versions qv WHERE qv.id = r.source_question_version_id
          AND qv.status = 'published' AND question_display_rights_active(
            qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM review_events r
      JOIN cards c ON c.id = r.card_id
      WHERE r.user_id = $1 AND c.user_id = $1 AND (c.source_question_version_id IS NULL OR EXISTS (
        SELECT 1 FROM question_versions qv WHERE qv.id = c.source_question_version_id
          AND qv.status = 'published' AND question_display_rights_active(
            qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM notes r
      WHERE r.user_id = $1 AND (r.source_question_version_id IS NULL OR EXISTS (
        SELECT 1 FROM question_versions qv WHERE qv.id = r.source_question_version_id
          AND qv.status = 'published' AND question_display_rights_active(
            qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM note_concepts r
      JOIN notes n ON n.id = r.note_id
      WHERE n.user_id = $1 AND (n.source_question_version_id IS NULL OR EXISTS (
        SELECT 1 FROM question_versions qv WHERE qv.id = n.source_question_version_id
          AND qv.status = 'published' AND question_display_rights_active(
            qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM note_links r
      JOIN notes f ON f.id = r.from_note_id JOIN notes t ON t.id = r.to_note_id
      WHERE f.user_id = $1 AND t.user_id = $1
        AND (f.source_question_version_id IS NULL OR EXISTS (
          SELECT 1 FROM question_versions qv WHERE qv.id = f.source_question_version_id
            AND qv.status = 'published' AND question_display_rights_active(
              qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))
        AND (t.source_question_version_id IS NULL OR EXISTS (
          SELECT 1 FROM question_versions qv WHERE qv.id = t.source_question_version_id
            AND qv.status = 'published' AND question_display_rights_active(
              qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM note_collections r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM note_collection_items r
      JOIN note_collections c ON c.id = r.collection_id JOIN notes n ON n.id = r.note_id
      WHERE c.user_id = $1 AND n.user_id = $1
        AND (n.source_question_version_id IS NULL OR EXISTS (
          SELECT 1 FROM question_versions qv WHERE qv.id = n.source_question_version_id
            AND qv.status = 'published' AND question_display_rights_active(
              qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM goals r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM protected_commitments r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM plans r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM plan_tasks r JOIN plans p ON p.id = r.plan_id WHERE p.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM plan_revisions r JOIN plans p ON p.id = r.plan_id WHERE p.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM intervention_outcomes r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM (
      SELECT c.id, c.question_version_id, c.prompt_type,
        CASE WHEN c.question_version_id IS NULL OR EXISTS (
          SELECT 1 FROM question_versions qv WHERE qv.id = c.question_version_id
            AND qv.status = 'published' AND question_display_rights_active(
              qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN c.message ELSE NULL END AS message,
        CASE WHEN c.question_version_id IS NULL OR EXISTS (
          SELECT 1 FROM question_versions qv WHERE qv.id = c.question_version_id
            AND qv.status = 'published' AND question_display_rights_active(
              qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN c.answer ELSE NULL END AS answer,
        c.adapter, c.model, c.grounded_on,
        CASE WHEN c.question_version_id IS NULL OR EXISTS (
          SELECT 1 FROM question_versions qv WHERE qv.id = c.question_version_id
            AND qv.status = 'published' AND question_display_rights_active(
              qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs))
          THEN 'available' ELSE 'withheld_rights_inactive' END AS content_disposition,
        c.created_at FROM coach_turns c WHERE c.user_id = $1
    ) r
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM coach_memory r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM notification_preferences r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM notifications r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM learner_accommodations r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM engagement_settings r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM engagement_days r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM qotd_answers r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM xp_ledger r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM achievements r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM article_reads r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM source_change_task_learners r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(r.id, r.exam_id, r.filename, r.status, r.created_at))::text)::BIGINT FROM import_batches r WHERE r.created_by = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(r.id, r.rights_id, r.title, r.media_type, r.sha256, r.created_at))::text)::BIGINT FROM private_documents r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM (
      SELECT r.id, r.scenario_id, r.scenario_version_id, r.current_state, r.started_at, r.finished_at,
        CASE WHEN r.user_id = $1 AND NOT EXISTS (
          SELECT 1 FROM scenario_team_members m WHERE m.run_id = r.id AND m.user_id <> $1
        ) THEN r.transcript ELSE '[]'::jsonb END AS solo_transcript
      FROM scenario_runs r
      WHERE r.user_id = $1 OR EXISTS (
        SELECT 1 FROM scenario_team_members m WHERE m.run_id = r.id AND m.user_id = $1
      )
    ) r
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM scenario_team_members r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM (
      SELECT a.id, a.question_version_id,
        CASE WHEN EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = a.question_version_id
          AND qv.status = 'published' AND question_display_rights_active(
            qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN a.reason ELSE NULL END AS reason,
        a.status,
        CASE WHEN EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = a.question_version_id
          AND qv.status = 'published' AND question_display_rights_active(
            qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs))
          THEN 'available' ELSE 'withheld_rights_inactive' END AS content_disposition,
        a.created_at FROM appeals a WHERE a.user_id = $1
    ) r
    UNION ALL SELECT octet_length(to_jsonb(ROW(i.id, i.run_id, i.role, i.expires_at, i.accepted_at, i.created_at))::text)::BIGINT
      FROM scenario_team_invites i WHERE i.created_by = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(h.id, h.run_id, h.situation, h.background, h.assessment, h.recommendation, h.created_at))::text)::BIGINT
      FROM scenario_handovers h JOIN scenario_team_members m ON m.id = h.from_member_id AND m.run_id = h.run_id
      WHERE m.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(a.handover_id, a.acknowledged_at))::text)::BIGINT
      FROM scenario_handover_acknowledgements a WHERE a.acknowledged_by = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM supervised_feedback r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM portfolio_entries r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM ce_activities r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM exam_outcomes r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM community_profiles r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM community_groups r WHERE r.created_by = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM (
      SELECT id, group_id, body, status, created_at FROM community_posts WHERE author = $1
    ) r
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM community_post_reports r WHERE r.reporter_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM duel_sessions r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(d.id, d.exam_id, d.status, d.chapter_id, d.question_count,
      CASE WHEN d.challenger = $1 THEN 'challenger' ELSE 'opponent' END,
      CASE WHEN d.status <> 'done' THEN NULL WHEN d.winner = $1 THEN 'won' WHEN d.winner IS NULL THEN 'draw' ELSE 'lost' END,
      d.created_at))::text)::BIGINT FROM duels d WHERE d.challenger = $1 OR d.opponent = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(c.id, c.group_id, c.competition_id,
      CASE WHEN c.challenger_id = $1 THEN 'challenger' ELSE 'challenged' END, c.status, c.created_at))::text)::BIGINT
      FROM community_challenges c WHERE c.challenger_id = $1 OR c.challenged_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM competition_entries r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM competition_attempts r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM competition_league_players r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM competition_league_memberships r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(m.institution_id, i.name, m.role, i.created_at))::text)::BIGINT
      FROM institution_members m JOIN institutions i ON i.id = m.institution_id WHERE m.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(m.cohort_id, c.name, c.institution_id))::text)::BIGINT
      FROM cohort_members m JOIN cohorts c ON c.id = m.cohort_id WHERE m.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(a.id, a.cohort_id, c.name, a.title, a.due_at, a.created_at))::text)::BIGINT
      FROM assignments a JOIN cohort_members m ON m.cohort_id = a.cohort_id JOIN cohorts c ON c.id = a.cohort_id WHERE m.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(m.group_id, g.name, m.role, p.handle, m.joined_at))::text)::BIGINT
      FROM community_group_members m JOIN community_groups g ON g.id = m.group_id
      LEFT JOIN community_profiles p ON p.user_id = m.user_id WHERE m.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM integrity_events r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM user_devices r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM external_identities r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM lti_identities r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(r)::text)::BIGINT FROM entitlement_usage r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(r.owner_user_id, r.uses, r.active, r.created_at))::text)::BIGINT FROM referral_codes r WHERE r.owner_user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(r.id, r.user_id, r.device_id, r.exam_id, r.chapters, r.expires_at, r.created_at, r.updated_at))::text)::BIGINT FROM pack_leases r WHERE r.user_id = $1
    UNION ALL SELECT octet_length(to_jsonb(ROW(r.id, r.user_id, r.exam_id, r.device_id, r.created_at))::text)::BIGINT FROM pack_download_receipts r WHERE r.user_id = $1
    LIMIT $2
) AS export_rows
"#;

const ACCOUNT_EXPORT_SQL: &str = r#"
SELECT jsonb_build_object(
    'archive', jsonb_build_object(
        'format', 'medical-os-account-export',
        'version', 1,
        'maximum_inline_bytes', 8388608,
        'included_categories', jsonb_build_array(
            'profile_and_settings', 'learning_evidence', 'study_materials', 'planning',
            'coach_and_memory', 'notifications_and_engagement', 'library_activity_and_import_metadata',
            'professional_learning', 'community_and_competition', 'institution_memberships',
            'identity_and_device_metadata', 'entitlements_and_offline_metadata'
        ),
        'excluded_categories', jsonb_build_array(
            jsonb_build_object('id', 'credentials_and_sessions', 'reason', 'Passwords, session hashes, one-use tickets, shared invitation tokens, device keys, pack keys, and referral codes are never exported.'),
            jsonb_build_object('id', 'protected_learning_content', 'reason', 'Question, course, source, and licensed media content are not copied into the archive.'),
            jsonb_build_object('id', 'private_document_content', 'reason', 'Private-import text and files are excluded; safe document metadata is included.'),
            jsonb_build_object('id', 'other_learners_private_records', 'reason', 'Only this learner’s own records and explicit membership metadata are included.'),
            jsonb_build_object('id', 'rights_inactive_question_linked_text', 'reason', 'Question-linked notes, flashcards and reviews, and related links/concepts plus Coach, report, and appeal text are withheld when current display rights are inactive.'),
            jsonb_build_object('id', 'provider_and_signed_proof_material', 'reason', 'Provider secrets, signing material, pack checksums, and receipt signatures are excluded.'),
            jsonb_build_object('id', 'shared_simulation_transcripts', 'reason', 'Team transcripts and peer-authored handovers are excluded; only your own authored handovers and your membership, invite, and acknowledgement metadata are included.'),
            jsonb_build_object('id', 'operator_only_records', 'reason', 'Internal audit, editorial, and moderation notes are not learner-owned export records.')
        )
    ),
    'exported_at', transaction_timestamp(),
    'account', jsonb_build_object(
        'id', u.id, 'email', u.email, 'tier', u.tier, 'idp_subject', u.idp_subject,
        'single_active_session', u.single_active_session, 'max_devices', u.max_devices,
        'created_at', u.created_at
    ),
    'learner_accommodations', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.accommodation_key) FROM learner_accommodations r WHERE r.user_id = u.id), '[]'::jsonb),
    'notification_preferences', COALESCE((SELECT to_jsonb(r) FROM notification_preferences r WHERE r.user_id = u.id), '{}'::jsonb),
    'engagement_settings', COALESCE((SELECT to_jsonb(r) FROM engagement_settings r WHERE r.user_id = u.id), '{}'::jsonb),
    'practice_sessions', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, user_id, preset, chapter_id, source_session_id, mock_id, form_id, ai_allowed, time_limit_seconds, per_question_seconds, deadline, plan_task_key, late_sync_grace_seconds, away_timeout_seconds, integrity_policy, auto_submitted_by_policy, status, created_at, submitted_at FROM practice_sessions WHERE user_id = u.id) r), '[]'::jsonb),
    'session_items', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.session_id, r.item_index) FROM (SELECT i.id, i.session_id, i.item_index, i.question_version_id FROM session_items i JOIN practice_sessions s ON s.id = i.session_id WHERE s.user_id = u.id) r), '[]'::jsonb),
    'attempts', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, session_id, item_index, question_version_id, chosen_index, correct, confidence, assisted, elapsed_ms, answer_changes, hint_used, offline_recorded_at, created_at FROM attempts WHERE user_id = u.id) r), '[]'::jsonb),
    'learner_concept_state', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.chapter_id) FROM (SELECT chapter_id, ability, evidence_count, independent_count, updated_at FROM learner_concept_state WHERE user_id = u.id) r), '[]'::jsonb),
    'mock_attempts', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, mock_id, session_id, score_percent, passed, percentile, ranked, created_at FROM mock_attempts WHERE user_id = u.id) r), '[]'::jsonb),
    'question_marks', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.question_version_id) FROM (SELECT question_version_id, created_at FROM question_marks WHERE user_id = u.id) r), '[]'::jsonb),
    'question_reports', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT r.id, r.question_version_id, r.category, CASE WHEN EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = r.question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN r.note ELSE NULL END AS note, CASE WHEN EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = r.question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN 'available' ELSE 'withheld_rights_inactive' END AS content_disposition, r.status, r.resolved_at, r.created_at FROM question_reports r WHERE r.reporter_id = u.id) r), '[]'::jsonb),
    'retest_cards', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.due, r.question_version_id) FROM (SELECT question_version_id, passes, due, created_at, updated_at FROM retest_cards WHERE user_id = u.id) r), '[]'::jsonb),
    'retest_history', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, question_version_id, correct, created_at FROM retest_history WHERE user_id = u.id) r), '[]'::jsonb),
    'decks', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, name, created_at FROM decks WHERE user_id = u.id) r), '[]'::jsonb),
    'cards', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, deck_id, source_question_version_id, front, back, state, suspended, card_type, trust, cloze, created_at FROM cards WHERE user_id = u.id AND (source_question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = cards.source_question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))) r), '[]'::jsonb),
    'card_reviews', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.reviewed_at, r.id) FROM (SELECT r.id, r.card_id, r.rating, r.reviewed_at, r.created_at FROM review_events r JOIN cards c ON c.id = r.card_id WHERE r.user_id = u.id AND c.user_id = u.id AND (c.source_question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = c.source_question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))) r), '[]'::jsonb),
    'notes', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT n.id, n.title, n.body, n.source_question_version_id, n.created_at, n.updated_at FROM notes n WHERE n.user_id = u.id AND (n.source_question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = n.source_question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))) r), '[]'::jsonb),
    'note_concepts', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.note_id, r.concept) FROM (SELECT nc.note_id, nc.concept FROM note_concepts nc JOIN notes n ON n.id = nc.note_id WHERE n.user_id = u.id AND (n.source_question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = n.source_question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))) r), '[]'::jsonb),
    'note_links', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.id) FROM (SELECT l.id, l.from_note_id, l.to_note_id FROM note_links l JOIN notes f ON f.id = l.from_note_id JOIN notes t ON t.id = l.to_note_id WHERE f.user_id = u.id AND t.user_id = u.id AND (f.source_question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = f.source_question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs))) AND (t.source_question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = t.source_question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))) r), '[]'::jsonb),
    'note_collections', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.id) FROM (SELECT id, name FROM note_collections WHERE user_id = u.id) r), '[]'::jsonb),
    'note_collection_items', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.collection_id, r.note_id) FROM (SELECT i.collection_id, i.note_id FROM note_collection_items i JOIN note_collections c ON c.id = i.collection_id JOIN notes n ON n.id = i.note_id WHERE c.user_id = u.id AND n.user_id = u.id AND (n.source_question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = n.source_question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)))) r), '[]'::jsonb),
    'goals', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, exam_id, target_note, exam_date, version, retired, created_at FROM goals WHERE user_id = u.id) r), '[]'::jsonb),
    'protected_commitments', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.starts_at, r.id) FROM (SELECT id, label, starts_at, ends_at, created_at FROM protected_commitments WHERE user_id = u.id) r), '[]'::jsonb),
    'plans', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.plan_date, r.version, r.id) FROM (SELECT id, plan_date, version, status FROM plans WHERE user_id = u.id) r), '[]'::jsonb),
    'plan_tasks', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT t.id, t.plan_id, t.kind, t.title, t.chapter_id, t.question_count, t.source_session_id, t.status, t.added_by_revision, t.created_at FROM plan_tasks t JOIN plans p ON p.id = t.plan_id WHERE p.user_id = u.id) r), '[]'::jsonb),
    'plan_revisions', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT r.id, r.plan_id, r.from_version, r.to_version, r.reason_code, r.explanation, r.automatic, r.undone, r.receipt, r.created_at FROM plan_revisions r JOIN plans p ON p.id = r.plan_id WHERE p.user_id = u.id) r), '[]'::jsonb),
    'intervention_outcomes', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, intervention_type, concept_key, triggered_at, measured_at, outcome, created_at FROM intervention_outcomes WHERE user_id = u.id) r), '[]'::jsonb),
    'coach_turns', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT c.id, c.question_version_id, c.prompt_type, CASE WHEN c.question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = c.question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN c.message ELSE NULL END AS message, CASE WHEN c.question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = c.question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN c.answer ELSE NULL END AS answer, c.adapter, c.model, c.grounded_on, CASE WHEN c.question_version_id IS NULL OR EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = c.question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN 'available' ELSE 'withheld_rights_inactive' END AS content_disposition, c.created_at FROM coach_turns c WHERE c.user_id = u.id) r), '[]'::jsonb),
    'coach_memory', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.memory_key) FROM (SELECT memory_key, value, updated_at FROM coach_memory WHERE user_id = u.id) r), '[]'::jsonb),
    'notifications', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, category, title, body, read_at, created_at FROM notifications WHERE user_id = u.id) r), '[]'::jsonb),
    'engagement_days', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.day) FROM (SELECT day, questions_answered, goal_met, streak_count FROM engagement_days WHERE user_id = u.id) r), '[]'::jsonb),
    'qotd_answers', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.day) FROM (SELECT day, question_version_id, chosen_index, answered_at FROM qotd_answers WHERE user_id = u.id) r), '[]'::jsonb),
    'xp_ledger', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, points, reason, created_at FROM xp_ledger WHERE user_id = u.id) r), '[]'::jsonb),
    'achievements', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.earned_at, r.code) FROM (SELECT code, earned_at FROM achievements WHERE user_id = u.id) r), '[]'::jsonb),
    'article_reads', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.first_read_at, r.article_version_id) FROM (SELECT article_version_id, first_read_at FROM article_reads WHERE user_id = u.id) r), '[]'::jsonb),
    'source_change_task_learners', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.task_id) FROM (SELECT task_id, created_at FROM source_change_task_learners WHERE user_id = u.id) r), '[]'::jsonb),
    'import_batches', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, exam_id, filename, status, created_at FROM import_batches WHERE created_by = u.id) r), '[]'::jsonb),
    'private_documents', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, rights_id, title, media_type, sha256, created_at FROM private_documents WHERE user_id = u.id) r), '[]'::jsonb),
    'scenario_runs', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.started_at, r.id) FROM (SELECT r.id, r.scenario_id, r.scenario_version_id, r.current_state, r.started_at, r.finished_at, CASE WHEN r.user_id = u.id AND NOT EXISTS (SELECT 1 FROM scenario_team_members m WHERE m.run_id = r.id AND m.user_id <> u.id) THEN r.transcript ELSE '[]'::jsonb END AS solo_transcript FROM scenario_runs r WHERE r.user_id = u.id OR EXISTS (SELECT 1 FROM scenario_team_members m WHERE m.run_id = r.id AND m.user_id = u.id)) r), '[]'::jsonb),
    'scenario_team_members', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.joined_at, r.id) FROM (SELECT id, run_id, role, joined_at FROM scenario_team_members WHERE user_id = u.id) r), '[]'::jsonb),
    'scenario_team_invites', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, run_id, role, expires_at, accepted_at, created_at FROM scenario_team_invites WHERE created_by = u.id) r), '[]'::jsonb),
    'scenario_handovers', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT h.id, h.run_id, h.situation, h.background, h.assessment, h.recommendation, h.created_at FROM scenario_handovers h JOIN scenario_team_members m ON m.id = h.from_member_id AND m.run_id = h.run_id WHERE m.user_id = u.id) r), '[]'::jsonb),
    'scenario_handover_acknowledgements', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.acknowledged_at, r.handover_id) FROM (SELECT handover_id, acknowledged_at FROM scenario_handover_acknowledgements WHERE acknowledged_by = u.id) r), '[]'::jsonb),
    'appeals', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT a.id, a.question_version_id, CASE WHEN EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = a.question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN a.reason ELSE NULL END AS reason, a.status, CASE WHEN EXISTS (SELECT 1 FROM question_versions qv WHERE qv.id = a.question_version_id AND qv.status = 'published' AND question_display_rights_active(qv.rights_ref, qv.source_ref, qv.source_refs, qv.media_refs)) THEN 'available' ELSE 'withheld_rights_inactive' END AS content_disposition, a.created_at FROM appeals a WHERE a.user_id = u.id) r), '[]'::jsonb),
    'supervised_feedback', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, portfolio_entry_id, scenario_run_id, feedback, signed_off, created_at FROM supervised_feedback WHERE user_id = u.id) r), '[]'::jsonb),
    'portfolio', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, kind, title, detail, occurred_on, created_at FROM portfolio_entries WHERE user_id = u.id) r), '[]'::jsonb),
    'ce_activities', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.completed_at, r.id) FROM (SELECT id, activity, hours, completed_at FROM ce_activities WHERE user_id = u.id) r), '[]'::jsonb),
    'exam_outcomes', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, exam_id, sat_on, outcome, consented, verified, created_at FROM exam_outcomes WHERE user_id = u.id) r), '[]'::jsonb),
    'community_profiles', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at) FROM (SELECT handle, created_at FROM community_profiles WHERE user_id = u.id) r), '[]'::jsonb),
    'community_groups', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, name, created_at FROM community_groups WHERE created_by = u.id) r), '[]'::jsonb),
    'community_group_memberships', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.joined_at, r.group_id) FROM (SELECT m.group_id, g.name AS group_name, m.role, p.handle, m.joined_at FROM community_group_members m JOIN community_groups g ON g.id = m.group_id LEFT JOIN community_profiles p ON p.user_id = m.user_id WHERE m.user_id = u.id) r), '[]'::jsonb),
    'community_posts', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, group_id, body, status, created_at FROM community_posts WHERE author = u.id) r), '[]'::jsonb),
    'community_post_reports', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, group_id, post_id, reason, note, status, resolved_at, created_at FROM community_post_reports WHERE reporter_id = u.id) r), '[]'::jsonb),
    'duel_sessions', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.duel_id) FROM (SELECT duel_id, session_id, score, total_ms FROM duel_sessions WHERE user_id = u.id) r), '[]'::jsonb),
    'duels', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT d.id, d.exam_id, d.status, d.chapter_id, d.question_count, CASE WHEN d.challenger = u.id THEN 'challenger' ELSE 'opponent' END AS learner_role, CASE WHEN d.status <> 'done' THEN NULL WHEN d.winner = u.id THEN 'won' WHEN d.winner IS NULL THEN 'draw' ELSE 'lost' END AS outcome, d.created_at FROM duels d WHERE d.challenger = u.id OR d.opponent = u.id) r), '[]'::jsonb),
    'community_challenges', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT c.id, c.group_id, c.competition_id, CASE WHEN c.challenger_id = u.id THEN 'challenger' ELSE 'challenged' END AS learner_role, c.status, c.created_at FROM community_challenges c WHERE c.challenger_id = u.id OR c.challenged_id = u.id) r), '[]'::jsonb),
    'competition_entries', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.submitted_at, r.id) FROM (SELECT id, competition_id, handle, answers, score, total_time_ms, submitted_order, submitted_at FROM competition_entries WHERE user_id = u.id) r), '[]'::jsonb),
    'competition_attempts', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, competition_id, question_ids, current_index, option_order, answers, question_started_at, status, created_at, submitted_at FROM competition_attempts WHERE user_id = u.id) r), '[]'::jsonb),
    'competition_league_players', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.exam_id) FROM (SELECT exam_id, handle, division, active, joined_at FROM competition_league_players WHERE user_id = u.id) r), '[]'::jsonb),
    'competition_league_memberships', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.week_start, r.exam_id) FROM (SELECT cohort_id, exam_id, week_start, handle, joined_at, left_at FROM competition_league_memberships WHERE user_id = u.id) r), '[]'::jsonb),
    'institution_memberships', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.institution_id) FROM (SELECT m.institution_id, i.name AS institution_name, m.role, i.created_at AS institution_created_at FROM institution_members m JOIN institutions i ON i.id = m.institution_id WHERE m.user_id = u.id) r), '[]'::jsonb),
    'cohort_memberships', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.cohort_id) FROM (SELECT m.cohort_id, c.name AS cohort_name, c.institution_id FROM cohort_members m JOIN cohorts c ON c.id = m.cohort_id WHERE m.user_id = u.id) r), '[]'::jsonb),
    'assignments', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.due_at NULLS LAST, r.id) FROM (SELECT a.id, a.cohort_id, c.name AS cohort_name, a.title, a.due_at, a.created_at FROM assignments a JOIN cohort_members m ON m.cohort_id = a.cohort_id JOIN cohorts c ON c.id = a.cohort_id WHERE m.user_id = u.id) r), '[]'::jsonb),
    'integrity_events', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, session_id, signal_type, detail, client_time, server_time, created_at FROM integrity_events WHERE user_id = u.id) r), '[]'::jsonb),
    'user_devices', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, label, created_at, last_seen_at, revoked_at FROM user_devices WHERE user_id = u.id) r), '[]'::jsonb),
    'external_identities', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.institution_id, r.provider) FROM (SELECT institution_id, provider, subject, created_at FROM external_identities WHERE user_id = u.id) r), '[]'::jsonb),
    'lti_identities', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.linked_at, r.platform_id) FROM (SELECT platform_id, subject, linked_at FROM lti_identities WHERE user_id = u.id) r), '[]'::jsonb),
    'entitlement_usage', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.day, r.key) FROM (SELECT key, day, count FROM entitlement_usage WHERE user_id = u.id) r), '[]'::jsonb),
    'referral_ownership', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at) FROM (SELECT uses, active, created_at FROM referral_codes WHERE owner_user_id = u.id) r), '[]'::jsonb),
    'pack_leases', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, device_id, exam_id, chapters, expires_at, created_at, updated_at FROM pack_leases WHERE user_id = u.id) r), '[]'::jsonb),
    'pack_download_receipts', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY r.created_at, r.id) FROM (SELECT id, exam_id, device_id, created_at FROM pack_download_receipts WHERE user_id = u.id) r), '[]'::jsonb)
)
FROM users u
WHERE u.id = $1 AND u.deleted_at IS NULL
"#;

pub async fn export_account(
    State(state): State<Arc<AppState>>,
    user: AccountRecoveryUser,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;

    let (rows, estimated_bytes): (i64, i64) = sqlx::query_as(ACCOUNT_EXPORT_SIZE_SQL)
        .bind(user.user_id)
        .bind(MAX_ACCOUNT_EXPORT_ROWS + 1)
        .fetch_one(&mut *tx)
        .await?;
    if rows > MAX_ACCOUNT_EXPORT_ROWS || estimated_bytes > MAX_ACCOUNT_EXPORT_BYTES as i64 {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "account_export_too_large",
            message:
                "This account is too large for a direct export. No partial archive was created."
                    .into(),
            details: None,
        });
    }

    let archive = sqlx::query_scalar::<_, serde_json::Value>(ACCOUNT_EXPORT_SQL)
        .bind(user.user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(ApiError::unauthorized)?;
    let encoded = serde_json::to_vec(&archive).map_err(|_| ApiError::internal())?;
    if encoded.len() > MAX_ACCOUNT_EXPORT_BYTES {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "account_export_too_large",
            message:
                "This account is too large for a direct export. No partial archive was created."
                    .into(),
            details: None,
        });
    }

    tx.commit().await?;
    Ok(Json(archive))
}
