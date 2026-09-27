//! QB-08: learner issue reports on question versions, with a fixed
//! 3-distinct-learner quarantine rule. Reports never disclose other learners'
//! identities to the reporter. ADMIN-06/QB-16 resolves a whole question-version
//! report group atomically, with visible SLAs and reporter feedback.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Distinct-learner unresolved reports that quarantine a version (QB-16 item
/// statistics replace this fixed rule in Phase 2).
pub const QUARANTINE_VOTES: i64 = 3;

fn valid_category(raw: &str) -> bool {
    matches!(
        raw,
        "wrong_answer"
            | "bad_explanation"
            | "typo"
            | "duplicate"
            | "outdated"
            | "broken_image"
            | "other"
    )
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "reports/ReportQuestionRequest.ts",
        rename = "ReportQuestionRequest"
    )
)]
pub struct ReportReq {
    #[cfg_attr(
        feature = "type-export",
        ts(
            type = "\"wrong_answer\" | \"bad_explanation\" | \"typo\" | \"duplicate\" | \"outdated\" | \"broken_image\" | \"other\""
        )
    )]
    pub category: String,
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct QueueParams {
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "reports/ResolveQuestionReportRequest.ts",
        rename = "ResolveQuestionReportRequest"
    )
)]
pub struct ResolveReportReq {
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"resolved_fixed\" | \"resolved_rejected\"")
    )]
    pub status: String,
    pub resolution_note: String,
    pub correction_note: Option<String>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "reports/QuestionReportResponse.ts",
        rename = "QuestionReportResponse"
    )
)]
pub struct QuestionReportResponse {
    pub report_id: Uuid,
    pub already_recorded: bool,
    pub quarantined: bool,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"open\" | \"quarantined\" | \"resolved_fixed\" | \"resolved_rejected\"")
    )]
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub acknowledged_at: chrono::DateTime<chrono::Utc>,
    pub acknowledgement_due_at: chrono::DateTime<chrono::Utc>,
    pub resolution_due_at: chrono::DateTime<chrono::Utc>,
    pub resolution_note: Option<String>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub corrected_version_id: Option<Uuid>,
    pub corrected_version_number: Option<i32>,
    pub correction_note: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "reports/ReportFeedback.ts",
        rename = "ReportFeedback"
    )
)]
pub struct ReportFeedback {
    pub category: String,
    pub note: Option<String>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "reports/AdminReport.ts", rename = "AdminReport")
)]
pub struct AdminReport {
    pub report_id: Uuid,
    pub question_version_id: Uuid,
    pub question_id: Uuid,
    pub version: i32,
    pub vignette: String,
    pub lead_in: String,
    pub category: String,
    pub reporter_feedback: Vec<ReportFeedback>,
    pub feedback_truncated: bool,
    pub report_count: i64,
    pub first_reported_at: chrono::DateTime<chrono::Utc>,
    pub acknowledgement_due_at: chrono::DateTime<chrono::Utc>,
    pub resolution_due_at: chrono::DateTime<chrono::Utc>,
    pub acknowledgements_on_time: bool,
    pub resolution_overdue: bool,
    pub quarantined: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "reports/AdminReportsResponse.ts",
        rename = "AdminReportsResponse"
    )
)]
pub struct AdminReportsResponse {
    pub reports: Vec<AdminReport>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "reports/ResolveQuestionReportResponse.ts",
        rename = "ResolveQuestionReportResponse"
    )
)]
pub struct ResolveQuestionReportResponse {
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"resolved_fixed\" | \"resolved_rejected\"")
    )]
    pub status: String,
    pub question_version_id: Uuid,
    pub corrected_version_id: Option<Uuid>,
    pub resolved_reports: u64,
    pub notified_reporters: usize,
}

pub async fn report(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(version_id): Path<Uuid>,
    Json(req): Json<ReportReq>,
) -> ApiResult<Json<QuestionReportResponse>> {
    if !valid_category(&req.category) {
        return Err(ApiError::unprocessable(
            "invalid_category",
            "category must be wrong_answer|bad_explanation|typo|duplicate|outdated|broken_image|other",
        ));
    }
    let note = req.note.unwrap_or_default();
    if note.chars().count() > 2000 {
        return Err(ApiError::unprocessable(
            "note_too_long",
            "note must be at most 2000 characters",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let version = sqlx::query!(
        "SELECT id, status FROM question_versions WHERE id = $1 FOR UPDATE",
        version_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;

    let existing = sqlx::query!(
        "SELECT r.id, r.status, r.created_at, r.acknowledged_at,
                r.resolution_note, r.resolved_at, r.corrected_version_id,
                r.correction_note,
                (SELECT corrected.version FROM question_versions corrected
                 WHERE corrected.id = r.corrected_version_id) AS \"corrected_version_number?\"
         FROM question_reports r
         WHERE r.question_version_id = $1 AND r.reporter_id = $2",
        version_id,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(existing) = existing {
        tx.commit().await?;
        return Ok(Json(QuestionReportResponse {
            report_id: existing.id,
            already_recorded: true,
            quarantined: existing.status == "quarantined",
            status: existing.status,
            created_at: existing.created_at,
            acknowledged_at: existing.acknowledged_at,
            acknowledgement_due_at: existing.created_at + chrono::Duration::hours(24),
            resolution_due_at: existing.created_at + chrono::Duration::hours(72),
            resolution_note: existing.resolution_note,
            resolved_at: existing.resolved_at,
            corrected_version_id: existing.corrected_version_id,
            corrected_version_number: existing.corrected_version_number,
            correction_note: existing.correction_note,
        }));
    }
    if version.status != "published" {
        return Err(ApiError::not_found("question_not_found"));
    }

    let inserted = sqlx::query!(
        "INSERT INTO question_reports (id, question_version_id, reporter_id, category, note)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, created_at AS \"created_at!\", acknowledged_at AS \"acknowledged_at!\"",
        Uuid::new_v4(),
        version.id,
        user.user_id,
        req.category,
        note
    )
    .fetch_one(&mut *tx)
    .await?;
    let unresolved_votes = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!"
           FROM question_reports
           WHERE question_version_id = $1 AND status IN ('open', 'quarantined')"#,
        version_id
    )
    .fetch_one(&mut *tx)
    .await?
    .n;
    let status = if unresolved_votes >= QUARANTINE_VOTES {
        sqlx::query!(
            "UPDATE question_reports SET status = 'quarantined'
             WHERE question_version_id = $1 AND status IN ('open', 'quarantined')",
            version_id
        )
        .execute(&mut *tx)
        .await?;
        "quarantined"
    } else {
        "open"
    };
    tx.commit().await?;
    Ok(Json(QuestionReportResponse {
        report_id: inserted.id,
        already_recorded: false,
        status: status.into(),
        quarantined: status == "quarantined",
        created_at: inserted.created_at,
        acknowledged_at: inserted.acknowledged_at,
        acknowledgement_due_at: inserted.created_at + chrono::Duration::hours(24),
        resolution_due_at: inserted.created_at + chrono::Duration::hours(72),
        resolution_note: None,
        resolved_at: None,
        corrected_version_id: None,
        corrected_version_number: None,
        correction_note: None,
    }))
}

pub async fn my_reports(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(version_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query!(
        "SELECT 1 AS one FROM question_versions WHERE id = $1",
        version_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    let rows = sqlx::query!(
        "SELECT r.id, r.category, r.status, r.note, r.resolution_note, r.resolved_at,
                r.corrected_version_id, r.correction_note,
                (SELECT corrected.version FROM question_versions corrected
                 WHERE corrected.id = r.corrected_version_id) AS \"corrected_version_number?\",
                r.created_at, r.acknowledged_at
         FROM question_reports r
         WHERE r.question_version_id = $1 AND r.reporter_id = $2 ORDER BY r.created_at",
        version_id,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let quarantined = sqlx::query!(
        "SELECT 1 AS one FROM question_reports
         WHERE question_version_id = $1 AND status = 'quarantined' LIMIT 1",
        version_id
    )
    .fetch_optional(&state.pool)
    .await?
    .is_some();
    Ok(Json(serde_json::json!({
        "reports": rows.iter().map(|r| serde_json::json!({
            "id": r.id, "category": r.category,
            "status": r.status, "note": r.note,
            "resolution_note": r.resolution_note,
            "correction_note": r.correction_note,
            "resolved_at": r.resolved_at,
            "corrected_version_id": r.corrected_version_id,
            "corrected_version_number": r.corrected_version_number,
            "created_at": r.created_at,
            "acknowledged_at": r.acknowledged_at,
            "acknowledgement_due_at": r.created_at + chrono::Duration::hours(24),
            "resolution_due_at": r.created_at + chrono::Duration::hours(72),
            "resolution_overdue": r.resolved_at.is_none()
                && chrono::Utc::now() > r.created_at + chrono::Duration::hours(72),
        })).collect::<Vec<_>>(),
        "quarantined": quarantined,
    })))
}

/// ADMIN-06/QB-16: unresolved issue queue, one row per affected question version.
pub async fn review_queue(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: HeaderMap,
    Query(params): Query<QueueParams>,
) -> ApiResult<Json<AdminReportsResponse>> {
    state.require_admin(
        headers
            .get("x-admin-token")
            .and_then(|value| value.to_str().ok()),
    )?;
    let limit = params.limit.unwrap_or(100).clamp(1, 500);
    let rows = sqlx::query(
        r#"WITH ranked AS (
               SELECT r.id, r.question_version_id, r.category, r.note,
                      r.created_at, r.acknowledged_at, r.status,
                      ROW_NUMBER() OVER (
                          PARTITION BY r.question_version_id ORDER BY r.created_at, r.id
                      ) AS feedback_rank
               FROM question_reports r
               WHERE r.status IN ('open', 'quarantined')
           ),
           unresolved AS (
               SELECT r.question_version_id,
                      jsonb_agg(jsonb_build_object(
                          'category', r.category, 'note', r.note
                      ) ORDER BY r.created_at, r.id)
                          FILTER (WHERE r.feedback_rank <= 20) AS reporter_feedback,
                      COUNT(*) AS report_count,
                      COUNT(*) > 20 AS feedback_truncated,
                      MIN(r.created_at) AS first_reported_at,
                      BOOL_AND(r.acknowledged_at <= r.created_at + interval '24 hours')
                          AS acknowledgements_on_time,
                      BOOL_OR(r.status = 'quarantined') AS quarantined
               FROM ranked r
               GROUP BY r.question_version_id
           ),
           first_report AS (
               SELECT id, question_version_id, category
               FROM ranked WHERE feedback_rank = 1
           )
           SELECT f.id AS report_id, u.question_version_id, qv.question_id,
                  qv.version, qv.vignette, qv.lead_in,
                  f.category, u.reporter_feedback, u.feedback_truncated,
                  u.report_count,
                  u.first_reported_at,
                  u.first_reported_at + interval '24 hours' AS acknowledgement_due_at,
                  u.first_reported_at + interval '72 hours' AS resolution_due_at,
                  u.acknowledgements_on_time,
                  (now() > u.first_reported_at + interval '72 hours') AS resolution_overdue,
                  u.quarantined
           FROM unresolved u
           JOIN first_report f ON f.question_version_id = u.question_version_id
           JOIN question_versions qv ON qv.id = u.question_version_id
           ORDER BY resolution_overdue DESC, u.first_reported_at
           LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;
    let reports: Vec<AdminReport> = rows
        .into_iter()
        .map(|row| -> Result<AdminReport, sqlx::Error> {
            let feedback: serde_json::Value = row.try_get("reporter_feedback")?;
            let reporter_feedback = serde_json::from_value(feedback)
                .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
            Ok(AdminReport {
                report_id: row.try_get("report_id")?,
                question_version_id: row.try_get("question_version_id")?,
                question_id: row.try_get("question_id")?,
                version: row.try_get("version")?,
                vignette: row.try_get("vignette")?,
                lead_in: row.try_get("lead_in")?,
                category: row.try_get("category")?,
                reporter_feedback,
                feedback_truncated: row.try_get("feedback_truncated")?,
                report_count: row.try_get("report_count")?,
                first_reported_at: row.try_get("first_reported_at")?,
                acknowledgement_due_at: row.try_get("acknowledgement_due_at")?,
                resolution_due_at: row.try_get("resolution_due_at")?,
                acknowledgements_on_time: row.try_get("acknowledgements_on_time")?,
                resolution_overdue: row.try_get("resolution_overdue")?,
                quarantined: row.try_get("quarantined")?,
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(Json(AdminReportsResponse { reports }))
}

/// Resolve every learner report for one question version as a single editorial decision.
pub async fn resolve(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(report_id): Path<Uuid>,
    Json(req): Json<ResolveReportReq>,
) -> ApiResult<Json<ResolveQuestionReportResponse>> {
    state.require_admin(
        headers
            .get("x-admin-token")
            .and_then(|value| value.to_str().ok()),
    )?;
    if !matches!(req.status.as_str(), "resolved_fixed" | "resolved_rejected") {
        return Err(ApiError::unprocessable(
            "invalid_resolution_status",
            "status must be resolved_fixed or resolved_rejected",
        ));
    }
    let resolution_note = req.resolution_note.trim();
    if resolution_note.is_empty() || resolution_note.chars().count() > 2000 {
        return Err(ApiError::unprocessable(
            "invalid_resolution_note",
            "resolution_note must be 1-2000 characters",
        ));
    }
    let correction_note = if req.status == "resolved_fixed" {
        let note = req.correction_note.as_deref().unwrap_or("").trim();
        if note.is_empty() || note.chars().count() > 2000 {
            return Err(ApiError::unprocessable(
                "invalid_correction_note",
                "correction_note must be 1-2000 characters when marking a report fixed",
            ));
        }
        Some(note)
    } else {
        None
    };

    let mut tx = state.pool.begin().await?;
    let question_version_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT question_version_id FROM question_reports WHERE id = $1",
    )
    .bind(report_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::conflict("report_already_resolved", "report is no longer open"))?;
    let question = sqlx::query!(
        "SELECT question_id, version FROM question_versions WHERE id = $1 FOR UPDATE",
        question_version_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::conflict(
            "report_already_resolved",
            "question version no longer exists",
        )
    })?;
    let target_is_open = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM question_reports
         WHERE id = $1 AND question_version_id = $2 AND status IN ('open', 'quarantined')
         FOR UPDATE",
    )
    .bind(report_id)
    .bind(question_version_id)
    .fetch_optional(&mut *tx)
    .await?
    .is_some();
    if !target_is_open {
        return Err(ApiError::conflict(
            "report_already_resolved",
            "report is no longer open",
        ));
    }
    let corrected_version_id = if req.status == "resolved_fixed" {
        Some(
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM question_versions
                 WHERE question_id = $1 AND version > $2 AND status = 'published'
                 ORDER BY version DESC LIMIT 1 FOR UPDATE",
            )
            .bind(question.question_id)
            .bind(question.version)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                ApiError::conflict(
                    "correction_version_required",
                    "publish a newer version of this question before resolving it as fixed",
                )
            })?,
        )
    } else {
        None
    };
    let reporters = sqlx::query_scalar::<_, Uuid>(
        "SELECT reporter_id FROM question_reports
         WHERE question_version_id = $1 AND status IN ('open', 'quarantined')
         ORDER BY id FOR UPDATE",
    )
    .bind(question_version_id)
    .fetch_all(&mut *tx)
    .await?;
    let updated = sqlx::query(
        "UPDATE question_reports
         SET status = $2, resolution_note = $3, resolved_at = now(),
             resolved_by = $4, corrected_version_id = $5, correction_note = $6
         WHERE question_version_id = $1 AND status IN ('open', 'quarantined')",
    )
    .bind(question_version_id)
    .bind(&req.status)
    .bind(resolution_note)
    .bind(user.user_id)
    .bind(corrected_version_id)
    .bind(correction_note)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if req.status == "resolved_fixed" {
        sqlx::query(
            "UPDATE question_versions SET status = 'archived'
             WHERE id = $1 AND status = 'published'",
        )
        .bind(question_version_id)
        .execute(&mut *tx)
        .await?;
    }
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "question_reports_resolved",
        "question_version",
        question_version_id,
        json!({
            "status": req.status,
            "resolution_note": resolution_note,
            "correction_note": correction_note,
            "corrected_version_id": corrected_version_id,
            "report_count": updated
        }),
    )
    .await?;
    let outcome = if req.status == "resolved_fixed" {
        "marked corrected"
    } else {
        "reviewed and left unchanged"
    };
    let notification_body = format!("Your report was {outcome}. {resolution_note}");
    let notified_reporters = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO notifications (id, user_id, category, title, body, deep_link)
           SELECT gen_random_uuid(), reporter.user_id, 'report',
                  'Question report reviewed', $2, '/practice'
           FROM UNNEST($1::uuid[]) AS reporter(user_id)
           LEFT JOIN notification_preferences preference
             ON preference.user_id = reporter.user_id
           WHERE COALESCE(preference.reports, TRUE)
           RETURNING user_id"#,
    )
    .bind(&reporters)
    .bind(notification_body)
    .fetch_all(&mut *tx)
    .await?
    .len();
    tx.commit().await?;
    Ok(Json(ResolveQuestionReportResponse {
        status: req.status,
        question_version_id,
        corrected_version_id,
        resolved_reports: updated,
        notified_reporters,
    }))
}
