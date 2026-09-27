//! QB-03/QB-04/QB-05/EX-04: practice sessions, tutor feedback, idempotent
//! durable answers, submission with evidence updates and plan revision.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use crate::agent;
use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::routes::mock::MockType;
use crate::seed::QuestionOption;
use crate::state::AppState;

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/BlueprintSlice.ts",
        rename = "BlueprintSlice"
    )
)]
pub struct BlueprintSlice {
    pub chapter_id: Uuid,
    pub count: i32,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/CreateSessionRequest.ts",
        rename = "CreateSessionRequest"
    )
)]
pub struct CreateSessionReq {
    pub preset: String,
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub chapter_id: Option<Uuid>,
    /// QB-06 targeted pool: several chapters at once (overrides chapter_id).
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub chapter_ids: Option<Vec<Uuid>>,
    /// QB-07 blueprint-balanced random pool; counts are exact per chapter.
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub blueprint: Option<Vec<BlueprintSlice>>,
    /// QB-06 pool filter: any (default) | unseen | incorrect | marked.
    #[cfg_attr(
        feature = "type-export",
        ts(
            type = "\"any\" | \"unseen\" | \"incorrect\" | \"marked\"",
            optional = nullable
        )
    )]
    pub source: Option<String>,
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub question_count: Option<i32>,
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub source_session_id: Option<Uuid>,
    /// AI-08: stable identity for a task launched from today's plan.
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub plan_task_key: Option<Uuid>,
    /// EX-08: required for the timed preset, validated server-side.
    #[cfg_attr(feature = "type-export", ts(type = "number", optional = nullable))]
    pub time_limit_seconds: Option<i64>,
    /// QB-03: optional per-question budget for untimed sessions (seconds).
    #[cfg_attr(feature = "type-export", ts(type = "number", optional = nullable))]
    pub per_question_seconds: Option<i64>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/SessionOption.ts",
        rename = "SessionOption"
    )
)]
pub struct SessionOption {
    text: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/CreateSessionItem.ts",
        rename = "CreateSessionItem"
    )
)]
pub struct CreateSessionItem {
    item_index: i16,
    question_version_id: Uuid,
    vignette: String,
    lead_in: String,
    difficulty: String,
    options: Vec<SessionOption>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/CreateSessionResponse.ts",
        rename = "CreateSessionResponse"
    )
)]
pub struct CreateSessionResponse {
    session_id: Uuid,
    items: Vec<CreateSessionItem>,
    per_question_seconds: Option<i32>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/SessionDetailOption.ts",
        rename = "SessionDetailOption"
    )
)]
pub struct SessionDetailOption {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    rationale: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/SessionReportStatus.ts",
        rename = "SessionReportStatus"
    )
)]
pub enum SessionReportStatus {
    Open,
    Quarantined,
    ResolvedFixed,
    ResolvedRejected,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/SessionReportReceipt.ts",
        rename = "SessionReportReceipt"
    )
)]
pub struct SessionReportReceipt {
    status: String,
    resolution_note: Option<String>,
    correction_note: Option<String>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    corrected_version_id: Option<Uuid>,
    corrected_version_number: Option<i32>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    acknowledged_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    acknowledgement_due_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    resolution_due_at: chrono::DateTime<chrono::Utc>,
    resolution_overdue: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/SessionItem.ts",
        rename = "SessionItem"
    )
)]
pub struct SessionDetailItem {
    item_index: i16,
    question_version_id: Uuid,
    vignette: String,
    lead_in: String,
    difficulty: String,
    hint_available: bool,
    hint_used: bool,
    options: Vec<SessionDetailOption>,
    answered: bool,
    chosen_index: Option<i16>,
    correct: Option<bool>,
    correct_index: Option<i16>,
    key_learning_point: Option<String>,
    exam_tip: Option<String>,
    report_status: Option<SessionReportStatus>,
    corrected_version_id: Option<Uuid>,
    corrected: bool,
    correction_note: Option<String>,
    my_report: Option<SessionReportReceipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    tutoring_cards: Option<Vec<crate::routes::program::TutoringCard>>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/PracticeSession.ts",
        rename = "PracticeSession"
    )
)]
pub struct PracticeSessionResponse {
    session_id: Uuid,
    user_id: Uuid,
    preset: String,
    chapter_id: Option<Uuid>,
    source_session_id: Option<Uuid>,
    status: String,
    mock_id: Option<Uuid>,
    time_limit_seconds: Option<i32>,
    per_question_seconds: Option<i32>,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    deadline: Option<chrono::DateTime<chrono::Utc>>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    server_now: chrono::DateTime<chrono::Utc>,
    items: Vec<SessionDetailItem>,
}

pub(crate) struct PoolQuestion {
    pub id: Uuid,
    pub vignette: String,
    pub lead_in: String,
    pub difficulty: String,
    pub options: serde_json::Value,
}

fn free_allowance_reached(limit: i64, used: i64) -> ApiError {
    ApiError::forbidden_with_details(
        "free_allowance_reached",
        format!("Daily free allowance of {limit} questions reached — it resets tomorrow."),
        serde_json::json!({
            "allowance": {
                "limit": limit,
                "used": used,
                "remaining": limit.saturating_sub(used).max(0),
            }
        }),
    )
}

fn free_allowance_insufficient(limit: i64, used: i64, required: i64) -> ApiError {
    ApiError::forbidden_with_details(
        "free_allowance_insufficient",
        format!(
            "Only {} free questions remain today; this planned task needs {required}.",
            limit.saturating_sub(used).max(0)
        ),
        serde_json::json!({
            "allowance": {
                "limit": limit,
                "used": used,
                "remaining": limit.saturating_sub(used).max(0),
                "required": required,
            }
        }),
    )
}

async fn validate_plan_task_key(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    plan_task_key: Option<Uuid>,
    preset: &str,
    chapter_id: Option<Uuid>,
    source_session_id: Option<Uuid>,
    question_count: Option<i32>,
) -> ApiResult<Option<i32>> {
    let Some(plan_task_key) = plan_task_key else {
        return Ok(None);
    };
    let kind = match preset {
        "tutor" | "timed" => "practice",
        "revision" => "revision",
        _ => {
            return Err(ApiError::unprocessable(
                "invalid_plan_task",
                "this session type cannot be linked to a plan task",
            ))
        }
    };
    let task_question_count = sqlx::query_scalar::<_, i32>(
        r#"SELECT t.question_count FROM plan_tasks t
           WHERE t.plan_id = (
                   SELECT id FROM plans
                   WHERE user_id = $1 AND plan_date = CURRENT_DATE
                   ORDER BY version DESC LIMIT 1
               )
               AND t.task_key = $2 AND t.kind = $3 AND t.status = 'pending'
               AND t.chapter_id IS NOT DISTINCT FROM $4
               AND t.source_session_id IS NOT DISTINCT FROM $5
               AND ($6::INTEGER IS NULL OR t.question_count = $6)
           LIMIT 1"#,
    )
    .bind(user_id)
    .bind(plan_task_key)
    .bind(kind)
    .bind(chapter_id)
    .bind(source_session_id)
    .bind(question_count)
    .fetch_optional(pool)
    .await?;
    task_question_count.map(Some).ok_or_else(|| {
        ApiError::unprocessable(
            "invalid_plan_task",
            "the linked task is not pending in your current plan",
        )
    })
}

// 9 args is the session's honest shape; grouping would add a type with one
// caller (same precedent as the extractive answer fn in coach.rs).
#[allow(clippy::too_many_arguments)]
async fn insert_session(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    preset: &str,
    chapter_id: Option<Uuid>,
    source_session_id: Option<Uuid>,
    time_limit_seconds: Option<i32>,
    per_question_seconds: Option<i32>,
    plan_task_key: Option<Uuid>,
    pool_questions: &[PoolQuestion],
) -> ApiResult<Json<CreateSessionResponse>> {
    // EX-08: the server issues the deadline — the client never sets it, and
    // answer acceptance is checked against it server-side.
    let deadline = time_limit_seconds
        .map(|limit| chrono::Utc::now() + chrono::Duration::seconds(limit as i64));
    let sid = Uuid::new_v4();
    let mut tx = pool.begin().await?;
    sqlx::query!(
        "INSERT INTO practice_sessions
           (id, user_id, preset, chapter_id, source_session_id, time_limit_seconds,
            deadline, per_question_seconds, plan_task_key, late_sync_grace_seconds,
            integrity_policy)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'log_only')",
        sid,
        user_id,
        preset,
        chapter_id,
        source_session_id,
        time_limit_seconds,
        deadline,
        per_question_seconds,
        plan_task_key,
        if matches!(preset, "tutor" | "timed") {
            600
        } else {
            0
        }
    )
    .execute(&mut *tx)
    .await?;
    let mut items = Vec::with_capacity(pool_questions.len());
    for (i, q) in pool_questions.iter().enumerate() {
        let idx = i as i16;
        sqlx::query!(
            "INSERT INTO session_items (id, session_id, item_index, question_version_id)
             VALUES ($1, $2, $3, $4)",
            Uuid::new_v4(),
            sid,
            idx,
            q.id
        )
        .execute(&mut *tx)
        .await?;
        let opts: Vec<QuestionOption> =
            serde_json::from_value(q.options.clone()).map_err(|_| ApiError::internal())?;
        items.push(CreateSessionItem {
            item_index: idx,
            question_version_id: q.id,
            vignette: q.vignette.clone(),
            lead_in: q.lead_in.clone(),
            difficulty: q.difficulty.clone(),
            // §11.3: no answer keys or rationales before they are permitted.
            options: opts
                .into_iter()
                .map(|o| SessionOption { text: o.text })
                .collect(),
        });
    }
    tx.commit().await?;
    Ok(Json(CreateSessionResponse {
        session_id: sid,
        items,
        per_question_seconds,
    }))
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<CreateSessionReq>,
) -> ApiResult<Json<CreateSessionResponse>> {
    let free_daily_questions = crate::routes::settings::current_bounded_i64(
        &state.pool,
        "free_daily_questions",
        state.free_daily_questions,
        0,
        5000,
    )
    .await?;
    // COM-01: the free-tier daily allowance is an entitlement check (§26.1) —
    // upgrade prompts may originate only from here, never from the Coach.
    // # ponytail: revision sessions are exempt (they re-practice already-
    // served items); resource-specific rights still depend on billing links.
    if req.preset.as_str() != "revision" {
        let tier = sqlx::query_scalar!("SELECT tier FROM users WHERE id = $1", user.user_id)
            .fetch_one(&state.pool)
            .await?;
        if tier == "free" {
            let used = sqlx::query!(
                r#"SELECT COALESCE(COUNT(*), 0) AS "n!"
                   FROM attempts a
                   JOIN practice_sessions s ON s.id = a.session_id
                   WHERE a.user_id = $1 AND a.created_at::date = CURRENT_DATE
                     AND s.preset <> 'revision'"#,
                user.user_id
            )
            .fetch_one(&state.pool)
            .await?
            .n;
            if used >= free_daily_questions {
                return Err(free_allowance_reached(free_daily_questions, used));
            }
        }
    }
    // Replanning and linked task launch serialize on the learner row. Keep
    // this lock until the task identity is validated and the session/items
    // are inserted, so a replan cannot remove a task between those steps.
    let plan_task_lock = if req.plan_task_key.is_some() {
        let mut tx = state.pool.begin().await?;
        let _: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE id = $1 FOR NO KEY UPDATE")
            .bind(user.user_id)
            .fetch_one(&mut *tx)
            .await?;
        Some(tx)
    } else {
        None
    };
    let result = match req.preset.as_str() {
        "tutor" | "timed" => {
            // QB-06 targeted pool: chapter_ids wins over a single chapter_id.
            let chapter_id = match (&req.chapter_ids, req.chapter_id) {
                (Some(ids), _) if !ids.is_empty() => {
                    if ids.len() > 20 {
                        return Err(ApiError::unprocessable(
                            "too_many_chapters",
                            "at most 20 chapters per targeted session",
                        ));
                    }
                    // Any member proves hierarchy ownership; unknown ids fail
                    // selection with an honest empty pool below.
                    ids[0]
                }
                (_, Some(id)) => id,
                _ => {
                    return Err(ApiError::unprocessable(
                        "chapter_required",
                        "practice sessions need a chapter_id",
                    ))
                }
            };
            // Preserve the planned workload: a linked session cannot claim a
            // task complete after serving a different requested question count.
            let count = req.question_count.unwrap_or(10).clamp(1, 50) as i64;
            let chapters: Vec<Uuid> = match &req.chapter_ids {
                Some(ids) if !ids.is_empty() => ids.clone(),
                _ => vec![chapter_id],
            };
            if req.plan_task_key.is_some() && (chapters.len() != 1 || chapters[0] != chapter_id) {
                return Err(ApiError::unprocessable(
                    "plan_task_chapter_mismatch",
                    "a linked plan task must serve only its planned chapter",
                ));
            }
            let planned_count = validate_plan_task_key(
                &state.pool,
                user.user_id,
                req.plan_task_key,
                &req.preset,
                Some(chapter_id),
                None,
                Some(count as i32),
            )
            .await?;
            if plan_task_lock.is_some() {
                if let Some(required) = planned_count {
                    let tier =
                        sqlx::query_scalar!("SELECT tier FROM users WHERE id = $1", user.user_id)
                            .fetch_one(&state.pool)
                            .await?;
                    if tier == "free" {
                        let used = sqlx::query!(
                            r#"SELECT COUNT(*) AS "n!"
                               FROM attempts a
                               JOIN practice_sessions s ON s.id = a.session_id
                               WHERE a.user_id = $1 AND a.created_at::date = CURRENT_DATE
                                 AND s.preset <> 'revision'"#,
                            user.user_id
                        )
                        .fetch_one(&state.pool)
                        .await?
                        .n;
                        if used.saturating_add(i64::from(required)) > free_daily_questions {
                            return Err(free_allowance_insufficient(
                                free_daily_questions,
                                used,
                                i64::from(required),
                            ));
                        }
                    }
                }
            }
            let source = req.source.as_deref().unwrap_or("any");
            if !matches!(source, "any" | "unseen" | "incorrect" | "marked") {
                return Err(ApiError::unprocessable(
                    "invalid_source",
                    "source must be any|unseen|incorrect|marked",
                ));
            }
            // LIMIT binds as i64 in sqlx.
            // EX-08: timed sessions carry a server-issued deadline. The floor
            // is configurable so CI/E2E can run short timed sessions.
            // QB-03: the per-question budget is an untimed-session option.
            let per_question_seconds = match req.per_question_seconds {
                Some(budget) => {
                    if req.preset == "timed" {
                        return Err(ApiError::unprocessable(
                            "invalid_per_question_budget",
                            "per-question budgets apply to untimed sessions only",
                        ));
                    }
                    if !(5..=3600).contains(&budget) {
                        return Err(ApiError::unprocessable(
                            "invalid_per_question_budget",
                            "per_question_seconds must be 5-3600",
                        ));
                    }
                    Some(budget as i32)
                }
                None => None,
            };
            let time_limit_seconds = if req.preset == "timed" {
                let limit = req.time_limit_seconds.ok_or_else(|| {
                    ApiError::unprocessable(
                        "time_limit_required",
                        "timed sessions need time_limit_seconds",
                    )
                })?;
                if !(state.min_time_limit_seconds..=14_400).contains(&limit) {
                    return Err(ApiError::unprocessable(
                        "time_limit_out_of_range",
                        format!(
                            "time_limit_seconds must be between {} and 14400",
                            state.min_time_limit_seconds
                        ),
                    ));
                }
                Some(limit as i32)
            } else {
                None
            };
            // QB-06: pool filter in one static query — $3 selects the source
            // predicate; $2 is the learner for unseen/incorrect/marked checks.
            let pool_qs = sqlx::query!(
                r#"SELECT id, vignette, lead_in, difficulty, options
                   FROM question_versions qv
                   WHERE status = 'published' AND chapter_id = ANY($1)
                     AND NOT EXISTS (
                         SELECT 1 FROM question_reports r
                         WHERE r.question_version_id = qv.id
                           AND r.status = 'quarantined'
                     )
                     AND NOT EXISTS (
                         SELECT 1 FROM reserved_questions rq
                         WHERE rq.question_version_id = qv.id
                     )
                     AND (
                         $3 = 'any'
                         OR ($3 = 'unseen' AND NOT EXISTS (
                             SELECT 1 FROM attempts a
                             WHERE a.question_version_id = qv.id AND a.user_id = $2))
                         OR ($3 = 'incorrect' AND EXISTS (
                             SELECT 1 FROM attempts a
                             WHERE a.question_version_id = qv.id AND a.user_id = $2
                               AND a.correct = FALSE))
                         OR ($3 = 'marked' AND EXISTS (
                             SELECT 1 FROM question_marks m
                             WHERE m.question_version_id = qv.id AND m.user_id = $2))
                     )
                   ORDER BY random() LIMIT $4"#,
                &chapters,
                user.user_id,
                source,
                count
            )
            .fetch_all(&state.pool)
            .await?;
            if pool_qs.is_empty() {
                return Err(ApiError::unprocessable(
                    "empty_pool",
                    "No questions are available for this selection.",
                ));
            }
            if req.plan_task_key.is_some() && pool_qs.len() < count as usize {
                return Err(ApiError::unprocessable(
                    "plan_task_pool_shortfall",
                    "There are not enough eligible questions to complete this planned task.",
                ));
            }
            let qs: Vec<PoolQuestion> = pool_qs
                .into_iter()
                .map(|r| PoolQuestion {
                    id: r.id,
                    vignette: r.vignette,
                    lead_in: r.lead_in,
                    difficulty: r.difficulty,
                    options: r.options,
                })
                .collect();
            insert_session(
                &state.pool,
                user.user_id,
                &req.preset,
                Some(chapter_id),
                None,
                time_limit_seconds,
                per_question_seconds,
                req.plan_task_key,
                &qs,
            )
            .await
        }
        "blueprint" => {
            let _planned_count = validate_plan_task_key(
                &state.pool,
                user.user_id,
                req.plan_task_key,
                "blueprint",
                None,
                None,
                None,
            )
            .await?;
            let slices = req
                .blueprint
                .as_ref()
                .filter(|slices| !slices.is_empty())
                .ok_or_else(|| {
                    ApiError::unprocessable(
                        "blueprint_required",
                        "blueprint sessions need at least one chapter/count slice",
                    )
                })?;
            if slices.len() > 20
                || slices.iter().any(|slice| !(1..=50).contains(&slice.count))
                || slices.iter().map(|slice| slice.count).sum::<i32>() > 50
            {
                return Err(ApiError::unprocessable(
                    "invalid_blueprint",
                    "blueprint must contain at most 20 chapters, 1-50 questions per chapter, and 50 questions total",
                ));
            }

            let mut seen = HashSet::with_capacity(slices.len());
            if slices.iter().any(|slice| !seen.insert(slice.chapter_id)) {
                return Err(ApiError::unprocessable(
                    "invalid_blueprint",
                    "blueprint chapter_id values must be unique",
                ));
            }

            let chapter_ids: Vec<Uuid> = slices.iter().map(|slice| slice.chapter_id).collect();
            let hierarchy = sqlx::query!(
                r#"SELECT COUNT(*) AS "chapters!", COUNT(DISTINCT exam_id) AS "exams!"
                   FROM curriculum_nodes
                   WHERE id = ANY($1) AND kind = 'chapter'"#,
                &chapter_ids
            )
            .fetch_one(&state.pool)
            .await?;
            if hierarchy.chapters != slices.len() as i64 || hierarchy.exams != 1 {
                return Err(ApiError::unprocessable(
                    "invalid_blueprint",
                    "all blueprint chapter_id values must be valid chapters from one exam",
                ));
            }

            let source = req.source.as_deref().unwrap_or("any");
            if !matches!(source, "any" | "unseen" | "incorrect" | "marked") {
                return Err(ApiError::unprocessable(
                    "invalid_source",
                    "source must be any|unseen|incorrect|marked",
                ));
            }

            let total = slices.iter().map(|slice| slice.count).sum::<i32>();
            let mut qs = Vec::with_capacity(total as usize);
            for slice in slices {
                let pool_qs = sqlx::query!(
                    r#"SELECT id, vignette, lead_in, difficulty, options
                       FROM question_versions qv
                       WHERE status = 'published' AND chapter_id = $1
                         AND NOT EXISTS (
                             SELECT 1 FROM question_reports r
                             WHERE r.question_version_id = qv.id
                               AND r.status = 'quarantined'
                         )
                         AND (
                             $3 = 'any'
                             OR ($3 = 'unseen' AND NOT EXISTS (
                                 SELECT 1 FROM attempts a
                                 WHERE a.question_version_id = qv.id AND a.user_id = $2))
                             OR ($3 = 'incorrect' AND EXISTS (
                                 SELECT 1 FROM attempts a
                                 WHERE a.question_version_id = qv.id AND a.user_id = $2
                                   AND a.correct = FALSE))
                             OR ($3 = 'marked' AND EXISTS (
                                 SELECT 1 FROM question_marks m
                                 WHERE m.question_version_id = qv.id AND m.user_id = $2))
                         )
                       ORDER BY random() LIMIT $4"#,
                    slice.chapter_id,
                    user.user_id,
                    source,
                    i64::from(slice.count)
                )
                .fetch_all(&state.pool)
                .await?;
                if pool_qs.len() != slice.count as usize {
                    return Err(ApiError::unprocessable(
                        "blueprint_pool_shortfall",
                        "not enough eligible questions to satisfy the requested blueprint",
                    ));
                }
                qs.extend(pool_qs.into_iter().map(|r| PoolQuestion {
                    id: r.id,
                    vignette: r.vignette,
                    lead_in: r.lead_in,
                    difficulty: r.difficulty,
                    options: r.options,
                }));
            }

            insert_session(
                &state.pool,
                user.user_id,
                "blueprint",
                None,
                None,
                None,
                None,
                None,
                &qs,
            )
            .await
        }
        "revision" => {
            let src = req.source_session_id.ok_or_else(|| {
                ApiError::unprocessable(
                    "source_session_required",
                    "revision sessions need a source_session_id",
                )
            })?;
            let linked_question_count = validate_plan_task_key(
                &state.pool,
                user.user_id,
                req.plan_task_key,
                "revision",
                None,
                Some(src),
                req.question_count,
            )
            .await?;
            sqlx::query!(
                "SELECT 1 AS one FROM practice_sessions
                 WHERE id = $1 AND user_id = $2 AND status = 'submitted'",
                src,
                user.user_id
            )
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| {
                ApiError::unprocessable(
                    "source_not_found",
                    "source session does not exist or is not submitted",
                )
            })?;
            let mut pool_qs = sqlx::query!(
                r#"SELECT id, vignette, lead_in, difficulty, options FROM (
                       SELECT DISTINCT qv.id, qv.vignette, qv.lead_in, qv.difficulty, qv.options
                       FROM question_versions qv
                       WHERE qv.status = 'published' AND (qv.id IN (
                           SELECT question_version_id FROM attempts
                           WHERE session_id = $1
                             AND (correct = FALSE OR chosen_index IS NULL)
                       )
                       OR qv.id IN (
                           SELECT si.question_version_id FROM session_items si
                           WHERE si.session_id = $1 AND NOT EXISTS (
                               SELECT 1 FROM attempts a
                               WHERE a.session_id = si.session_id
                                 AND a.item_index = si.item_index
                           )
                       ))
                       AND NOT EXISTS (
                           SELECT 1 FROM question_reports r
                           WHERE r.question_version_id = qv.id
                             AND r.status = 'quarantined'
                       )
                   ) t
                   ORDER BY random()"#,
                src
            )
            .fetch_all(&state.pool)
            .await?;
            if let Some(expected_count) = linked_question_count {
                if pool_qs.len() < expected_count as usize {
                    return Err(ApiError::unprocessable(
                        "plan_task_pool_shortfall",
                        "There are not enough eligible missed questions to complete this planned revision task.",
                    ));
                }
                pool_qs.truncate(expected_count as usize);
            }
            if pool_qs.is_empty() {
                return Err(ApiError::unprocessable(
                    "nothing_to_revise",
                    "that session has no missed questions to re-practice",
                ));
            }
            let qs: Vec<PoolQuestion> = pool_qs
                .into_iter()
                .map(|r| PoolQuestion {
                    id: r.id,
                    vignette: r.vignette,
                    lead_in: r.lead_in,
                    difficulty: r.difficulty,
                    options: r.options,
                })
                .collect();
            insert_session(
                &state.pool,
                user.user_id,
                "revision",
                None,
                Some(src),
                None,
                None,
                req.plan_task_key,
                &qs,
            )
            .await
        }
        other => Err(ApiError::unprocessable(
            "unknown_preset",
            format!("unknown preset {other}"),
        )),
    };
    if let Some(tx) = plan_task_lock {
        if result.is_ok() {
            tx.commit().await?;
        }
    }
    result
}

/// Session detail for the client: full item list for the navigator, with
/// tutor feedback released per answer and exam-mode feedback held until submit
/// (§11.2–11.3 — nothing unreleased reaches the client).
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(sid): Path<Uuid>,
) -> ApiResult<Json<PracticeSessionResponse>> {
    let session = sqlx::query!(
        "SELECT preset, chapter_id, source_session_id, status, time_limit_seconds, deadline, mock_id, per_question_seconds AS \"per_question_seconds?\"
         FROM practice_sessions
         WHERE id = $1 AND user_id = $2",
        sid,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("session_not_found"))?;

    let items = sqlx::query!(
        r#"SELECT si.item_index, qv.id AS question_version_id, qv.status AS question_status,
                  qv.vignette, qv.lead_in, qv.difficulty,
                  COALESCE(source_corrected.options, qv.options) AS options,
                  COALESCE(source_corrected.correct_index, qv.correct_index) AS correct_index,
                  COALESCE(source_corrected.key_learning_point, qv.key_learning_point) AS key_learning_point,
                  (qv.hint IS NOT NULL AND btrim(qv.hint) <> '') AS "hint_available!",
                  si.hint_used,
                  COALESCE(source_corrected.exam_tip, qv.exam_tip) AS "exam_tip?",
                  a.id AS "attempt_id?", a.chosen_index, a.correct,
                  EXISTS (
                      SELECT 1 FROM question_reports r
                      WHERE r.question_version_id = qv.id
                        AND r.status IN ('open', 'quarantined')
                  ) AS "flagged!",
                  EXISTS (
                      SELECT 1 FROM question_reports r
                      WHERE r.question_version_id = qv.id
                        AND r.status = 'quarantined'
                  ) AS "quarantined!",
                  ($3 = 'submitted' AND EXISTS (
                      SELECT 1 FROM question_reports r
                      WHERE r.question_version_id = qv.id
                        AND r.status = 'resolved_fixed'
                  )) OR latest_correction.corrected_version_id IS NOT NULL AS "corrected!",
                  EXISTS (
                      SELECT 1 FROM question_reports r
                      WHERE r.corrected_version_id = qv.id
                        AND r.status = 'resolved_fixed'
                  ) AS "current_corrected!",
                  (SELECT r.correction_note FROM question_reports r
                   WHERE r.corrected_version_id = qv.id AND r.status = 'resolved_fixed'
                   ORDER BY r.resolved_at DESC LIMIT 1) AS "correction_note?",
                  EXISTS (
                      SELECT 1 FROM question_reports r
                      WHERE r.question_version_id = qv.id
                        AND r.status = 'resolved_rejected'
                  ) AS "reviewed_rejected!",
                  latest_correction.corrected_version_id AS "corrected_version_id?",
                  own_report.status AS "my_report_status?",
                  own_report.resolution_note AS "my_resolution_note?",
                  own_report.correction_note AS "my_correction_note?",
                  own_report.resolved_at AS "my_report_resolved_at?",
                  own_report.corrected_version_id AS "my_corrected_version_id?",
                  (SELECT corrected.version FROM question_versions corrected
                   WHERE corrected.id = own_report.corrected_version_id)
                      AS "my_corrected_version_number?",
                  own_report.created_at AS "my_report_created_at?",
                  own_report.acknowledged_at AS "my_acknowledged_at?"
           FROM session_items si
           JOIN question_versions qv ON qv.id = si.question_version_id
           LEFT JOIN LATERAL (
               SELECT correction.corrected_version_id
               FROM (
                   SELECT t.corrected_question_version_id AS corrected_version_id,
                          t.resolved_at, t.id
                   FROM source_change_tasks t
                   WHERE t.question_version_id = qv.id
                     AND t.status = 'resolved' AND t.resolution = 'corrected'
                   UNION ALL
                   SELECT r.corrected_version_id, r.resolved_at, r.id
                   FROM question_reports r
                   WHERE r.question_version_id = qv.id
                     AND r.status = 'resolved_fixed' AND r.corrected_version_id IS NOT NULL
               ) correction
               ORDER BY correction.resolved_at DESC NULLS LAST, correction.id DESC LIMIT 1
           ) latest_correction ON $3 = 'submitted'
           LEFT JOIN question_versions source_corrected
             ON source_corrected.id = latest_correction.corrected_version_id
           LEFT JOIN attempts a
             ON a.session_id = si.session_id AND a.item_index = si.item_index
           LEFT JOIN question_reports own_report
             ON own_report.question_version_id = qv.id AND own_report.reporter_id = $2
           WHERE si.session_id = $1
           ORDER BY si.item_index"#,
        sid,
        user.user_id,
        session.status
    )
    .fetch_all(&state.pool)
    .await?;

    let mut out = Vec::with_capacity(items.len());
    for it in items {
        let opts: Vec<QuestionOption> =
            serde_json::from_value(it.options.unwrap_or_else(|| serde_json::json!([])))
                .map_err(|_| ApiError::internal())?;
        let answered = it.attempt_id.is_some();
        let feedback_released = answered
            && (!matches!(session.preset.as_str(), "mock" | "timed")
                || session.status == "submitted");
        let public_opts = opts
            .into_iter()
            .map(|o| SessionDetailOption {
                text: o.text,
                rationale: if feedback_released {
                    Some(o.rationale)
                } else {
                    None
                },
            })
            .collect();
        // QB-08: honest flag state so the UI can label affected items.
        let report_status = if it.corrected {
            Some(SessionReportStatus::ResolvedFixed)
        } else if it.quarantined || it.question_status == "quarantined" {
            Some(SessionReportStatus::Quarantined)
        } else if it.flagged {
            Some(SessionReportStatus::Open)
        } else if it.reviewed_rejected {
            Some(SessionReportStatus::ResolvedRejected)
        } else {
            None
        };
        let my_report = match (
            it.my_report_status,
            it.my_report_created_at,
            it.my_acknowledged_at,
        ) {
            (Some(status), Some(created_at), Some(acknowledged_at)) => {
                let resolution_due_at = created_at + chrono::Duration::hours(72);
                let acknowledgement_due_at = created_at + chrono::Duration::hours(24);
                let resolution_overdue = it.my_report_resolved_at.is_none()
                    && chrono::Utc::now() > resolution_due_at;
                Some(SessionReportReceipt {
                    status,
                    resolution_note: it.my_resolution_note,
                    correction_note: it.my_correction_note,
                    resolved_at: it.my_report_resolved_at,
                    corrected_version_id: it.my_corrected_version_id,
                    corrected_version_number: it.my_corrected_version_number,
                    acknowledged_at,
                    acknowledgement_due_at,
                    resolution_due_at,
                    resolution_overdue,
                })
            }
            _ => None,
        };
        let tutoring_cards = if feedback_released
            && session.preset == "tutor"
            && it.question_status == "published"
        {
            Some(
                crate::routes::program::ensure_pregen(&state.pool, it.question_version_id).await?,
            )
        } else {
            None
        };
        out.push(SessionDetailItem {
            item_index: it.item_index,
            question_version_id: it.question_version_id,
            vignette: it.vignette,
            lead_in: it.lead_in,
            difficulty: it.difficulty,
            hint_available: session.preset == "tutor" && it.hint_available,
            hint_used: it.hint_used,
            options: public_opts,
            answered,
            chosen_index: it.chosen_index,
            correct: if feedback_released { it.correct } else { None },
            correct_index: if feedback_released {
                Some(it.correct_index)
            } else {
                None
            },
            key_learning_point: if feedback_released {
                Some(it.key_learning_point)
            } else {
                None
            },
            exam_tip: if feedback_released {
                it.exam_tip
            } else {
                None
            },
            report_status,
            corrected_version_id: it.corrected_version_id,
            // corrected covers both directions: a source correction swapped this
            // item's content, or the learner was served the replacement version.
            corrected: it.corrected || it.current_corrected,
            correction_note: it.correction_note,
            my_report,
            tutoring_cards,
        });
    }

    Ok(Json(PracticeSessionResponse {
        session_id: sid,
        user_id: user.user_id,
        preset: session.preset,
        chapter_id: session.chapter_id,
        source_session_id: session.source_session_id,
        status: session.status,
        mock_id: session.mock_id,
        time_limit_seconds: session.time_limit_seconds,
        per_question_seconds: session.per_question_seconds,
        // EX-08: the client derives its countdown from these two values, so
        // changing the device clock never extends the timer.
        deadline: session.deadline,
        server_now: chrono::Utc::now(),
        items: out,
    }))
}

/// Return an authored hint only after explicit learner action in a tutor run.
/// Viewing it is persisted on the session item before the answer can be saved.
#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/SessionHintResponse.ts",
        rename = "SessionHintResponse"
    )
)]
pub struct SessionHintResponse {
    hint: String,
    #[cfg_attr(feature = "type-export", ts(type = "true"))]
    assisted: bool,
}

pub async fn hint(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((sid, item_index)): Path<(Uuid, i16)>,
) -> ApiResult<Json<SessionHintResponse>> {
    let mut tx = state.pool.begin().await?;
    let session = sqlx::query!(
        "SELECT preset, status, deadline FROM practice_sessions WHERE id = $1 AND user_id = $2 FOR UPDATE",
        sid,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("session_not_found"))?;
    if session.preset != "tutor" {
        return Err(ApiError::forbidden(
            "hint_unavailable",
            "hints are available in tutor sessions only",
        ));
    }
    if session.status != "open" {
        return Err(ApiError::conflict(
            "session_closed",
            "session is already closed",
        ));
    }
    if session
        .deadline
        .is_some_and(|deadline| chrono::Utc::now() > deadline)
    {
        return Err(ApiError::conflict(
            "session_expired",
            "the session deadline has passed",
        ));
    }
    let item = sqlx::query!(
        "SELECT question_version_id FROM session_items
         WHERE session_id = $1 AND item_index = $2 FOR UPDATE",
        sid,
        item_index
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("unknown_item"))?;
    let answered = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM attempts WHERE session_id = $1 AND item_index = $2)",
    )
    .bind(sid)
    .bind(item_index)
    .fetch_one(&mut *tx)
    .await?;
    if answered {
        return Err(ApiError::conflict(
            "already_answered",
            "hints can only be opened before answering",
        ));
    }
    let hint: Option<String> =
        sqlx::query_scalar("SELECT hint FROM question_versions WHERE id = $1")
            .bind(item.question_version_id)
            .fetch_one(&mut *tx)
            .await?;
    let hint = hint
        .filter(|hint| !hint.trim().is_empty())
        .ok_or_else(|| ApiError::not_found("hint_unavailable"))?;
    sqlx::query!(
        "UPDATE session_items SET hint_used = TRUE
         WHERE session_id = $1 AND item_index = $2",
        sid,
        item_index
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(SessionHintResponse {
        hint,
        assisted: true,
    }))
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/AnswerRequest.ts",
        rename = "AnswerRequest"
    )
)]
pub struct AnswerReq {
    pub item_index: i16,
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub chosen_index: Option<i16>,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"sure\" | \"unsure\"", optional = nullable)
    )]
    pub confidence: Option<String>,
    /// QB-04: client-declared assistance (in-session tools). The server also
    /// marks answers assisted when a coach turn preceded them on the question.
    #[cfg_attr(feature = "type-export", ts(optional = nullable))]
    pub assisted: Option<bool>,
    pub idempotency_key: String,
    /// QB-17: client-measured time on item. Server clamps; absent stays
    /// null (never synthesized).
    #[cfg_attr(feature = "type-export", ts(type = "number", optional = nullable))]
    pub elapsed_ms: Option<i64>,
    /// Local timestamp for a queued practice answer. A late upload is accepted
    /// only within the practice grace window and is always assisted evidence.
    #[cfg_attr(feature = "type-export", ts(type = "string", optional = nullable))]
    pub client_recorded_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/AnswerRecordedResponse.ts",
        rename = "AnswerRecordedResponse"
    )
)]
pub struct AnswerRecordedResponse {
    already_recorded: bool,
    #[cfg_attr(feature = "type-export", ts(type = "true"))]
    recorded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(type = "true", optional))]
    answer_changed: Option<bool>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/AnswerFeedbackResponse.ts",
        rename = "AnswerFeedbackResponse"
    )
)]
pub struct AnswerFeedbackResponse {
    already_recorded: bool,
    correct: Option<bool>,
    correct_index: i16,
    options: Vec<QuestionOption>,
    key_learning_point: String,
    exam_tip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    tutoring_cards: Option<Vec<crate::routes::program::TutoringCard>>,
}

#[derive(Serialize)]
#[serde(untagged)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "practice/AnswerResponse.ts",
        rename = "AnswerResponse"
    )
)]
pub enum AnswerResponse {
    Feedback(AnswerFeedbackResponse),
    Recorded(AnswerRecordedResponse),
}

async fn replay_answer(
    state: &AppState,
    sid: Uuid,
    preset: &str,
    status: &str,
    idempotency_key: &str,
) -> ApiResult<Option<Json<AnswerResponse>>> {
    let replay = sqlx::query!(
        r#"SELECT a.question_version_id, a.chosen_index, a.correct, qv.correct_index, qv.options,
                  qv.key_learning_point, qv.exam_tip, qv.status AS question_status
           FROM attempts a JOIN question_versions qv ON qv.id = a.question_version_id
           WHERE a.session_id = $1 AND a.idempotency_key = $2"#,
        sid,
        idempotency_key
    )
    .fetch_optional(&state.pool)
    .await?;
    match replay {
        Some(r) => Ok(Some(
            response_for_question(
                state,
                preset,
                status,
                r.question_version_id,
                &r.question_status,
                true,
                r.correct,
                r.correct_index,
                &r.options,
                r.key_learning_point,
                r.exam_tip,
            )
            .await?,
        )),
        None => Ok(None),
    }
}

pub async fn answer(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(sid): Path<Uuid>,
    Json(req): Json<AnswerReq>,
) -> ApiResult<Json<AnswerResponse>> {
    apply_answer(&state, user.user_id, sid, req).await
}

/// OFF-02: the entire answer path — open-session check, server deadline,
/// idempotent insert, mock answer-change rule, SR-09 key-point card — lives
/// here so the offline sync endpoint applies exactly the same semantics
/// (no second implementation to drift).
pub async fn apply_answer(
    state: &AppState,
    user_id: Uuid,
    sid: Uuid,
    req: AnswerReq,
) -> ApiResult<Json<AnswerResponse>> {
    let session = sqlx::query!(
        "SELECT status, deadline, preset, created_at,
                per_question_seconds AS \"per_question_seconds?\",
                late_sync_grace_seconds
         FROM practice_sessions WHERE id = $1 AND user_id = $2",
        sid,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("session_not_found"))?;
    // Idempotent replay: same key, same stored answer, no duplicate attempt.
    if let Some(replay) = replay_answer(
        state,
        sid,
        &session.preset,
        &session.status,
        &req.idempotency_key,
    )
    .await?
    {
        return Ok(replay);
    }

    if session.status != "open" {
        return Err(ApiError::conflict(
            "session_closed",
            "session already submitted",
        ));
    }
    // Late practice sync is accepted only for a locally recorded event inside
    // the session window and the session's snapshotted grace period.
    let late_offline_sync = if let Some(deadline) = session.deadline {
        if chrono::Utc::now() > deadline {
            let now = chrono::Utc::now();
            let recorded_at = req.client_recorded_at.as_ref();
            let accepted = matches!(session.preset.as_str(), "tutor" | "timed" | "mock")
                && session.late_sync_grace_seconds > 0
                && now
                    <= deadline
                        + chrono::Duration::seconds(i64::from(session.late_sync_grace_seconds))
                && recorded_at.is_some_and(|at| at >= &session.created_at && at <= &deadline);
            if !accepted {
                return Err(ApiError::conflict(
                    "session_expired",
                    "Time is up. The session submits with what you answered.",
                ));
            }
            true
        } else {
            false
        }
    } else {
        false
    };

    let item = sqlx::query!(
        r#"SELECT qv.id, qv.status AS question_status, qv.correct_index, qv.options, qv.key_learning_point,
                  qv.exam_tip, si.hint_used
           FROM session_items si JOIN question_versions qv ON qv.id = si.question_version_id
           WHERE si.session_id = $1 AND si.item_index = $2"#,
        sid,
        req.item_index
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("unknown_item"))?;

    let already = sqlx::query!(
        r#"SELECT a.id, a.chosen_index, qv.correct_index
           FROM attempts a JOIN question_versions qv ON qv.id = a.question_version_id
           WHERE a.session_id = $1 AND a.item_index = $2"#,
        sid,
        req.item_index
    )
    .fetch_optional(&state.pool)
    .await?;
    if let Some(prev) = already {
        // Mocks behave like an exam: answers may change until submission, and
        // each real change is counted for the QB-17 answer-change analysis.
        // Everything else keeps first-answer-wins (idempotency, EX-04).
        if session.preset == "mock" {
            let changed = req.chosen_index != Some(prev.chosen_index.unwrap_or(-1));
            if changed {
                let correct = req.chosen_index.map(|c| c == prev.correct_index);
                sqlx::query!(
                    r#"UPDATE attempts
                       SET chosen_index = $3, correct = $4,
                           answer_changes = answer_changes + 1,
                           assisted = assisted OR $5,
                           offline_recorded_at = COALESCE(offline_recorded_at, $6)
                       WHERE id = $2 AND session_id = $1"#,
                    sid,
                    prev.id,
                    req.chosen_index,
                    correct,
                    req.assisted.unwrap_or(false) || item.hint_used || late_offline_sync,
                    if late_offline_sync {
                        req.client_recorded_at.clone()
                    } else {
                        None
                    }
                )
                .execute(&state.pool)
                .await?;
                return Ok(Json(AnswerResponse::Recorded(AnswerRecordedResponse {
                    already_recorded: true,
                    recorded: true,
                    answer_changed: Some(true),
                })));
            }
            return Ok(Json(AnswerResponse::Recorded(AnswerRecordedResponse {
                already_recorded: true,
                recorded: true,
                answer_changed: None,
            })));
        }
        return Err(ApiError::conflict(
            "already_answered",
            "this item already has an answer (first answer wins)",
        ));
    }

    if let Some(c) = req.chosen_index {
        let n_opts = item
            .options
            .as_array()
            .map(|a| a.len())
            .ok_or_else(ApiError::internal)?;
        if c < 0 || c as usize >= n_opts {
            return Err(ApiError::unprocessable(
                "invalid_option",
                "chosen_index is out of range",
            ));
        }
    }
    if let Some(conf) = &req.confidence {
        if conf != "sure" && conf != "unsure" {
            return Err(ApiError::unprocessable(
                "invalid_confidence",
                "confidence must be 'sure' or 'unsure'",
            ));
        }
    }
    let correct = req.chosen_index.map(|c| c == item.correct_index);
    // QB-17: clamp client-reported time into a sane range; out-of-range or
    // negative reports are dropped to NULL rather than trusted.
    let elapsed_ms = req.elapsed_ms.filter(|ms| (0..=3_600_000).contains(ms));
    // QB-03: when the session carries a per-question budget, a reported pace
    // beyond it is refused — the timer is optional but the cutoff is real.
    if let (Some(budget), Some(elapsed)) = (session.per_question_seconds, elapsed_ms) {
        if elapsed > budget as i64 * 1000 {
            return Err(ApiError::unprocessable(
                "question_time_exceeded",
                "the per-question budget elapsed before this answer",
            ));
        }
    }
    // QB-13: a viewed tutor hint is server-recorded on the session item, so a
    // client cannot omit that assistance when it records the answer.
    let assisted = req.assisted.unwrap_or(false) || item.hint_used || late_offline_sync;
    let offline_recorded_at = if late_offline_sync {
        req.client_recorded_at
    } else {
        None
    };

    let insert = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO attempts
             (id, session_id, item_index, user_id, question_version_id,
              chosen_index, correct, confidence, assisted, idempotency_key, elapsed_ms,
              offline_recorded_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
           ON CONFLICT DO NOTHING
           RETURNING id"#,
    )
    .bind(Uuid::new_v4())
    .bind(sid)
    .bind(req.item_index)
    .bind(user_id)
    .bind(item.id)
    .bind(req.chosen_index)
    .bind(correct)
    .bind(&req.confidence)
    .bind(assisted)
    .bind(&req.idempotency_key)
    .bind(elapsed_ms)
    .bind(offline_recorded_at);
    let free_tier = if session.preset == "revision" {
        false
    } else {
        sqlx::query_scalar::<_, String>("SELECT tier FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await?
            == "free"
    };
    let inserted = if free_tier {
        let free_daily_questions = crate::routes::settings::current_bounded_i64(
            &state.pool,
            "free_daily_questions",
            state.free_daily_questions,
            0,
            5000,
        )
        .await?;
        let mut tx = state.pool.begin().await?;
        let tier = sqlx::query_scalar::<_, String>(
            "SELECT tier FROM users WHERE id = $1 FOR NO KEY UPDATE",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        if tier == "free" {
            let replay_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (
                     SELECT 1 FROM attempts WHERE session_id = $1 AND idempotency_key = $2
                 )",
            )
            .bind(sid)
            .bind(&req.idempotency_key)
            .fetch_one(&mut *tx)
            .await?;
            if replay_exists {
                tx.commit().await?;
                return replay_answer(
                    state,
                    sid,
                    &session.preset,
                    &session.status,
                    &req.idempotency_key,
                )
                .await?
                .ok_or_else(ApiError::internal);
            }
            let used = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM attempts a
                 JOIN practice_sessions s ON s.id = a.session_id
                 WHERE a.user_id = $1 AND a.created_at::date = CURRENT_DATE
                   AND s.preset <> 'revision'",
            )
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
            if used >= free_daily_questions {
                return Err(free_allowance_reached(free_daily_questions, used));
            }
        }
        let inserted = insert.fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        inserted
    } else {
        insert.fetch_optional(&state.pool).await?
    };
    if inserted.is_none() {
        // Lost a race with the same key: serve the stored answer.
        let r = sqlx::query!(
            r#"SELECT a.question_version_id, a.chosen_index, a.correct, qv.correct_index, qv.options,
                      qv.key_learning_point, qv.exam_tip, qv.status AS question_status
               FROM attempts a JOIN question_versions qv ON qv.id = a.question_version_id
               WHERE a.session_id = $1 AND a.idempotency_key = $2"#,
            sid,
            req.idempotency_key
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::conflict("already_answered", "item already answered"))?;
        return response_for_question(
            state,
            &session.preset,
            &session.status,
            r.question_version_id,
            &r.question_status,
            true,
            r.correct,
            r.correct_index,
            &r.options,
            r.key_learning_point,
            r.exam_tip,
        )
        .await;
    }

    // SR-09: a miss files the key learning point as a review card so the
    // fact resurfaces even after the question itself is mastered.
    // Deduplicated per learner + question (source_question_version_id).
    if correct == Some(false) {
        ensure_key_point_card(
            state,
            user_id,
            item.id,
            &item.key_learning_point,
            item.exam_tip.as_deref(),
        )
        .await?;
    }

    response_for_question(
        state,
        &session.preset,
        &session.status,
        item.id,
        &item.question_status,
        false,
        correct,
        item.correct_index,
        &item.options,
        item.key_learning_point.clone(),
        item.exam_tip.clone(),
    )
    .await
}

/// SR-09: idempotently file the question's key learning point as a review
/// card in the learner's "Key points" deck. provenance via
/// cards.source_question_version_id prevents duplicates across sessions.
async fn ensure_key_point_card(
    state: &AppState,
    user_id: Uuid,
    question_version_id: Uuid,
    key_learning_point: &str,
    exam_tip: Option<&str>,
) -> ApiResult<()> {
    let mut tx = state.pool.begin().await?;
    let question_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM question_versions WHERE id = $1 FOR SHARE",
    )
    .bind(question_version_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("question_not_found"))?;
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
             SELECT 1 FROM cards
             WHERE user_id = $1 AND source_question_version_id = $2
         )",
    )
    .bind(user_id)
    .bind(question_version_id)
    .fetch_one(&mut *tx)
    .await?;
    if exists {
        tx.commit().await?;
        return Ok(());
    }
    let deck_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM decks WHERE user_id = $1 AND name = 'Key points'",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let deck_id = match deck_id {
        Some(id) => id,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO decks (id, user_id, name) VALUES ($1, $2, 'Key points')")
                .bind(id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            id
        }
    };
    let scheduler = scheduler::Scheduler::new();
    let card_state = serde_json::to_value(scheduler::to_state(&scheduler.new_card()))
        .map_err(|_| ApiError::internal())?;
    let back = exam_tip
        .map(str::to_string)
        .unwrap_or_else(|| key_learning_point.to_string());
    sqlx::query(
        r#"INSERT INTO cards
             (id, deck_id, user_id, front, back, state, suspended,
              source_question_version_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
    )
    .bind(Uuid::new_v4())
    .bind(deck_id)
    .bind(user_id)
    .bind(key_learning_point)
    .bind(back)
    .bind(card_state)
    .bind(question_status != "published")
    .bind(question_version_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// §11.2 one-session vocabulary: tutor reveals immediately; exam-style
/// presets (mock and timed) defer everything until submission — not even the
/// key is released early.
fn response_for_preset(
    preset: &str,
    already: bool,
    correct: Option<bool>,
    correct_index: i16,
    options: &serde_json::Value,
    key_learning_point: String,
    exam_tip: Option<String>,
) -> ApiResult<AnswerResponse> {
    if matches!(preset, "mock" | "timed") {
        return Ok(AnswerResponse::Recorded(AnswerRecordedResponse {
            already_recorded: already,
            recorded: true,
            answer_changed: None,
        }));
    }
    let opts: Vec<QuestionOption> =
        serde_json::from_value(options.clone()).map_err(|_| ApiError::internal())?;
    Ok(AnswerResponse::Feedback(AnswerFeedbackResponse {
        already_recorded: already,
        correct,
        correct_index,
        options: opts,
        key_learning_point,
        exam_tip,
        tutoring_cards: None,
    }))
}

/// SR-08: enroll missed, skipped, unsure, and assisted questions from the submitted
/// practice session into the learner's re-test queue at the configured first valid interval.
/// Atomic with the open-to-submitted transition.
async fn enroll_missed_questions_in_retest_queue(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    sid: Uuid,
) -> ApiResult<()> {
    let setting_val = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT value FROM app_settings WHERE key = 'retest_intervals_days'",
    )
    .fetch_optional(&mut *conn)
    .await?;
    let intervals = crate::routes::settings::effective_retest_intervals(
        crate::routes::settings::i64_list_or_default(
            setting_val.as_ref(),
            &crate::routes::settings::DEFAULT_RETEST_INTERVAL_DAYS,
        ),
    );
    let first_interval_days = intervals[0];
    let now = chrono::Utc::now();
    let due = now + chrono::Duration::days(first_interval_days);

    sqlx::query(
        r#"INSERT INTO retest_cards (user_id, question_version_id, passes, due, updated_at)
           SELECT DISTINCT
               $2 AS user_id,
               qv.id AS question_version_id,
               0 AS passes,
               $3 AS due,
               $4 AS updated_at
           FROM session_items si
           JOIN question_versions qv ON qv.id = si.question_version_id
           LEFT JOIN attempts a
             ON a.session_id = si.session_id AND a.item_index = si.item_index
           WHERE si.session_id = $1
             AND qv.status = 'published'
             AND (
                 a.id IS NULL
                 OR a.chosen_index IS NULL
                 OR a.correct = FALSE
                 OR a.confidence = 'unsure'
                 OR a.assisted = TRUE
                 OR si.hint_used = TRUE
             )
           ON CONFLICT (user_id, question_version_id) DO UPDATE SET
               passes = 0,
               due = EXCLUDED.due,
               updated_at = EXCLUDED.updated_at"#,
    )
    .bind(sid)
    .bind(user_id)
    .bind(due)
    .bind(now)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "mock/SubmitTimeItem.ts",
        rename = "SubmitTimeItem"
    )
)]
pub struct SubmitTimeItem {
    item_index: i16,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    elapsed_ms: Option<i64>,
    answer_changes: Option<i32>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "mock/SubmitTime.ts", rename = "SubmitTime")
)]
pub struct SubmitTime {
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    duration_seconds: Option<i64>,
    items: Vec<SubmitTimeItem>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "mock/MockResultBreakdown.ts",
        rename = "MockResultBreakdown"
    )
)]
pub struct MockResultBreakdown {
    chapter: String,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    total: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    correct: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(type = "number", optional))]
    time_seconds: Option<i64>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "mock/MockResult.ts", rename = "MockResult")
)]
pub struct MockResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    mock_type: Option<MockType>,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    score_percent: i64,
    passed: bool,
    pass_mark_percent: i32,
    percentile: Option<i32>,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    takers: i64,
    ranked: bool,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    late_sync_answers: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(type = "number", optional))]
    total_time_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(type = "number", optional))]
    avg_time_per_question_seconds: Option<i64>,
    breakdown: Vec<MockResultBreakdown>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "mock/SubmitResult.ts", rename = "SubmitResult")
)]
pub struct SubmitResult {
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    total: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    correct: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    incorrect: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    skipped: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    score: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    expected_score: Option<i64>,
    mock: Option<MockResult>,
    time: SubmitTime,
}

pub async fn submit(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(sid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let initial = sqlx::query!(
        r#"SELECT chapter_id, source_session_id, status, preset, mock_id,
                  result_payload::text AS "result_payload?"
           FROM practice_sessions
          WHERE id = $1 AND user_id = $2"#,
        sid,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("session_not_found"))?;
    if initial.status != "open" && initial.status != "submitted" {
        return Err(ApiError::conflict(
            "session_closed",
            "session already submitted",
        ));
    }
    if let Some(stored) = initial.result_payload.as_deref() {
        let receipt = serde_json::from_str(stored).map_err(|_| ApiError::internal())?;
        return Ok(Json(receipt));
    }

    agent::get_or_create_today(&state.pool, user.user_id).await?;
    let mut tx = state.pool.begin().await?;
    // Keep the session lock compatible with the fork's FK key-share check;
    // a stronger lock can deadlock with a submit retry waiting on the user row.
    let session = sqlx::query!(
        r#"SELECT chapter_id, source_session_id, status, preset, mock_id,
                  result_payload::text AS "result_payload?"
           FROM practice_sessions
          WHERE id = $1 AND user_id = $2
          FOR NO KEY UPDATE"#,
        sid,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("session_not_found"))?;
    let plan_task_key = sqlx::query_scalar!(
        "SELECT plan_task_key FROM practice_sessions WHERE id = $1",
        sid
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 FOR UPDATE",
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if session.status != "open" && session.status != "submitted" {
        return Err(ApiError::conflict(
            "session_closed",
            "session already submitted",
        ));
    }
    if let Some(stored) = session.result_payload.as_deref() {
        tx.commit().await?;
        let receipt = serde_json::from_str(stored).map_err(|_| ApiError::internal())?;
        return Ok(Json(receipt));
    }

    let totals = sqlx::query!(
        r#"SELECT
             COUNT(*) AS "total!",
             COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS "correct!",
             COALESCE(COUNT(*) FILTER (WHERE a.chosen_index IS NOT NULL AND a.correct = FALSE), 0) AS "incorrect!",
             COALESCE(COUNT(*) FILTER (WHERE a.id IS NULL OR a.chosen_index IS NULL), 0) AS "skipped!"
           FROM session_items si
           LEFT JOIN attempts a
             ON a.session_id = si.session_id AND a.item_index = si.item_index
           WHERE si.session_id = $1"#,
        sid
    )
    .fetch_one(&mut *tx)
    .await?;

    let mut applied_completion = false;
    if session.status == "open" {
        let updated = sqlx::query!(
            "UPDATE practice_sessions SET status = 'submitted', submitted_at = now()
             WHERE id = $1 AND user_id = $2 AND status = 'open'",
            sid,
            user.user_id
        )
        .execute(&mut *tx)
        .await?;
        applied_completion = updated.rows_affected() > 0;
    }
    if applied_completion {
        enroll_missed_questions_in_retest_queue(&mut *tx, user.user_id, sid).await?;
        agent::mark_linked_task_done_on(&mut tx, user.user_id, plan_task_key).await?;
    }
    tx.commit().await?;

    if applied_completion {
        crate::routes::engagement::award_session_xp(&state, user.user_id, totals.correct).await?;
        crate::routes::engagement::record_daily_progress(&state, user.user_id).await?;
        crate::routes::community::on_session_submitted(&state, sid).await?;
        agent::update_learner_state(&state.pool, user.user_id, sid).await?;
        // Revision plans come only from self-directed practice; mocks and
        // revision sessions never spawn them (anti-loop, §8.6).
        if session.preset == "tutor" {
            agent::maybe_create_revision(
                &state.pool,
                user.user_id,
                sid,
                totals.incorrect,
                totals.skipped,
            )
            .await?;
        }
    }

    let score = if totals.total == 0 {
        0
    } else {
        totals.correct * 100 / totals.total
    };

    // QB-15: expected-score comparison for self-built sessions, computed
    // from community correct-rates of the exact questions served. Percentile
    // is reserved for fixed forms (mocks). Hidden below the min sample.
    let community_min_sample = crate::routes::settings::current_bounded_i64(
        &state.pool,
        "community_min_sample",
        state.community_min_sample,
        1,
        1_000_000,
    )
    .await?;
    let expected_score = expected_score_for_session(&state.pool, sid, community_min_sample).await?;

    // QB-17: time + answer-change analysis — per-item elapsed (only where
    // the client reported it) and mock answer changes; wall-clock duration
    // from server timestamps. All numbers trace to records; missing time
    // stays null.
    let timing = sqlx::query!(
        r#"SELECT created_at, submitted_at FROM practice_sessions WHERE id = $1"#,
        sid
    )
    .fetch_one(&state.pool)
    .await?;
    let duration_seconds = timing
        .submitted_at
        .map(|end| (end - timing.created_at).num_seconds().max(0));
    let item_timings = sqlx::query!(
        r#"SELECT si.item_index, a.elapsed_ms, a.answer_changes AS "answer_changes?"
           FROM session_items si
           LEFT JOIN attempts a
             ON a.session_id = si.session_id AND a.item_index = si.item_index
           WHERE si.session_id = $1
           ORDER BY si.item_index"#,
        sid
    )
    .fetch_all(&state.pool)
    .await?;
    let time_items: Vec<SubmitTimeItem> = item_timings
        .iter()
        .map(|t| SubmitTimeItem {
            item_index: t.item_index,
            elapsed_ms: t.elapsed_ms,
            answer_changes: t.answer_changes,
        })
        .collect();

    let mut body = SubmitResult {
        total: totals.total,
        correct: totals.correct,
        incorrect: totals.incorrect,
        skipped: totals.skipped,
        score,
        expected_score,
        mock: None,
        time: SubmitTime {
            duration_seconds,
            items: time_items,
        },
    };

    if let Some(mock_id) = session.mock_id {
        let mock = sqlx::query!(
            "SELECT pass_mark_percent, mock_type FROM mocks WHERE id = $1",
            mock_id
        )
        .fetch_one(&state.pool)
        .await?;
        let passed = score >= mock.pass_mark_percent as i64;
        let late_sync_answers = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM attempts WHERE session_id = $1 AND offline_recorded_at IS NOT NULL",
        )
        .bind(sid)
        .fetch_one(&state.pool)
        .await?;
        let ranked = late_sync_answers == 0;
        sqlx::query!(
            "INSERT INTO mock_attempts
               (id, mock_id, user_id, session_id, score_percent, passed, ranked)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (mock_id, user_id, session_id) DO NOTHING",
            Uuid::new_v4(),
            mock_id,
            user.user_id,
            sid,
            score as i32,
            passed,
            ranked
        )
        .execute(&state.pool)
        .await?;
        // §11.8: percentile among takers of the SAME form, only once the
        // sample is meaningful — otherwise an honest null.
        let takers = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM mock_attempts WHERE mock_id = $1 AND ranked = TRUE",
        )
        .bind(mock_id)
        .fetch_one(&state.pool)
        .await?;
        let below = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM mock_attempts
             WHERE mock_id = $1 AND ranked = TRUE AND score_percent < $2",
        )
        .bind(mock_id)
        .bind(score as i32)
        .fetch_one(&state.pool)
        .await?;
        let percentile = if ranked && takers >= community_min_sample && takers > 1 {
            Some((below * 100 / (takers - 1)) as i32)
        } else {
            None
        };
        let total_time_seconds = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(elapsed_ms) / 1000, 0)::BIGINT FROM attempts WHERE session_id = $1",
        )
        .bind(sid)
        .fetch_one(&state.pool)
        .await?
        .max(0);

        let timing_stats = sqlx::query!(
            r#"SELECT
                COALESCE(SUM(elapsed_ms) / 1000, 0)::BIGINT AS "answered_time_seconds!",
                COUNT(*)::BIGINT AS "timed_answered_count!"
               FROM attempts
               WHERE session_id = $1
                 AND chosen_index IS NOT NULL
                 AND elapsed_ms IS NOT NULL"#,
            sid
        )
        .fetch_one(&state.pool)
        .await?;

        let avg_time_per_question_seconds = if timing_stats.timed_answered_count > 0 {
            (timing_stats.answered_time_seconds / timing_stats.timed_answered_count).max(0)
        } else {
            0
        };

        let breakdown = sqlx::query!(
            r#"SELECT c.name AS chapter_name,
                      COALESCE(COUNT(*), 0) AS "total!",
                      COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS "correct!",
                      COALESCE(SUM(a.elapsed_ms) / 1000, 0)::BIGINT AS "time_seconds!"
               FROM session_items si
               JOIN question_versions qv ON qv.id = si.question_version_id
               JOIN curriculum_nodes c ON c.id = qv.chapter_id
               LEFT JOIN attempts a
                 ON a.session_id = si.session_id AND a.item_index = si.item_index
               WHERE si.session_id = $1
               GROUP BY c.name ORDER BY c.name"#,
            sid
        )
        .fetch_all(&state.pool)
        .await?;
        body.mock = Some(MockResult {
            mock_type: Some(MockType::from_wire(&mock.mock_type).ok_or_else(ApiError::internal)?),
            score_percent: score,
            passed,
            pass_mark_percent: mock.pass_mark_percent,
            percentile,
            takers,
            ranked,
            late_sync_answers,
            total_time_seconds: Some(total_time_seconds),
            avg_time_per_question_seconds: Some(avg_time_per_question_seconds),
            breakdown: breakdown
                .into_iter()
                .map(|b| MockResultBreakdown {
                    chapter: b.chapter_name,
                    total: b.total,
                    correct: b.correct,
                    time_seconds: Some(b.time_seconds.max(0)),
                })
                .collect(),
        });
    }

    let stored = sqlx::query_scalar::<_, String>(
        "UPDATE practice_sessions
         SET result_payload = COALESCE(result_payload, $2)
         WHERE id = $1 AND user_id = $3 AND status = 'submitted'
         RETURNING result_payload::text",
    )
    .bind(sid)
    .bind(serde_json::to_value(body).map_err(|_| ApiError::internal())?)
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::conflict("session_closed", "session result is unavailable"))?;
    let receipt = serde_json::from_str(&stored).map_err(|_| ApiError::internal())?;
    Ok(Json(receipt))
}

#[allow(clippy::too_many_arguments)] // planner response assembly; each arg maps to one wire field
async fn response_for_question(
    state: &AppState,
    preset: &str,
    session_status: &str,
    question_version_id: Uuid,
    question_status: &str,
    already: bool,
    correct: Option<bool>,
    correct_index: i16,
    options: &serde_json::Value,
    key_learning_point: String,
    exam_tip: Option<String>,
) -> ApiResult<Json<AnswerResponse>> {
    let mut effective_question_version_id = question_version_id;
    let mut effective_question_status = question_status.to_owned();
    let mut effective_correct_index = correct_index;
    let mut effective_options = options.clone();
    let mut effective_key_learning_point = key_learning_point;
    let mut effective_exam_tip = exam_tip;

    // Submitted results follow the reviewed correction. Open sessions remain
    // pinned to the version and answer key they started with.
    if session_status == "submitted" {
        if let Some((version_id, status, corrected_index, corrected_options, key_point, tip)) =
            sqlx::query_as::<_, (Uuid, String, i16, serde_json::Value, String, Option<String>)>(
                r#"SELECT corrected.id, corrected.status, corrected.correct_index,
                          corrected.options, corrected.key_learning_point, corrected.exam_tip
                   FROM (
                       SELECT t.corrected_question_version_id AS corrected_version_id,
                              t.resolved_at, t.id
                       FROM source_change_tasks t
                       WHERE t.question_version_id = $1
                         AND t.status = 'resolved' AND t.resolution = 'corrected'
                       UNION ALL
                       SELECT r.corrected_version_id, r.resolved_at, r.id
                       FROM question_reports r
                       WHERE r.question_version_id = $1
                         AND r.status = 'resolved_fixed'
                         AND r.corrected_version_id IS NOT NULL
                   ) correction
                   JOIN question_versions corrected
                     ON corrected.id = correction.corrected_version_id
                   ORDER BY correction.resolved_at DESC NULLS LAST, correction.id DESC
                   LIMIT 1"#,
            )
            .bind(question_version_id)
            .fetch_optional(&state.pool)
            .await?
        {
            effective_question_version_id = version_id;
            effective_question_status = status;
            effective_correct_index = corrected_index;
            effective_options = corrected_options;
            effective_key_learning_point = key_point;
            effective_exam_tip = tip;
        }
    }

    let mut response = response_for_preset(
        preset,
        already,
        correct,
        effective_correct_index,
        &effective_options,
        effective_key_learning_point,
        effective_exam_tip,
    )?;
    if preset == "tutor" && effective_question_status == "published" {
        if let AnswerResponse::Feedback(response) = &mut response {
            response.tutoring_cards = Some(
                crate::routes::program::ensure_pregen(&state.pool, effective_question_version_id)
                    .await?,
            );
        }
    }
    Ok(Json(response))
}

/// Mean community correct-rate over the session's questions that already have
/// enough attempts (QB-15). None until at least one question qualifies.
async fn expected_score_for_session(
    pool: &sqlx::PgPool,
    sid: Uuid,
    min_sample: i64,
) -> ApiResult<Option<i64>> {
    let rows = sqlx::query!(
        r#"SELECT qv.id,
                  COALESCE(COUNT(*), 0) AS "attempts!",
                  COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS "correct!"
           FROM session_items si
           JOIN question_versions qv ON qv.id = si.question_version_id
           LEFT JOIN attempts a
             ON a.question_version_id = si.question_version_id
            AND a.correct IS NOT NULL
           WHERE si.session_id = $1
           GROUP BY qv.id"#,
        sid
    )
    .fetch_all(pool)
    .await?;
    let mut rates: Vec<f64> = Vec::new();
    for r in rows {
        if r.attempts >= min_sample {
            rates.push(r.correct as f64 * 100.0 / r.attempts as f64);
        }
    }
    if rates.is_empty() {
        return Ok(None);
    }
    let mean = rates.iter().sum::<f64>() / rates.len() as f64;
    Ok(Some(mean.round() as i64))
}

/// QB-15: per-question community statistics. Aggregates never identify a
/// learner; the numbers stay hidden until the minimum sample is met, and the
/// response says so instead of showing nothing.
pub async fn community_stats(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(vid): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let community_min_sample = crate::routes::settings::current_bounded_i64(
        &state.pool,
        "community_min_sample",
        state.community_min_sample,
        1,
        1_000_000,
    )
    .await?;
    let totals = sqlx::query!(
        r#"SELECT
             COALESCE(COUNT(*), 0) AS "attempts!",
             COALESCE(COUNT(*) FILTER (WHERE correct = TRUE), 0) AS "correct!"
           FROM attempts
           WHERE question_version_id = $1 AND chosen_index IS NOT NULL
             AND assisted = FALSE"#,
        vid
    )
    .fetch_one(&state.pool)
    .await?;
    let distribution = sqlx::query!(
        r#"SELECT chosen_index, COALESCE(COUNT(*), 0) AS "picks!"
           FROM attempts
           WHERE question_version_id = $1 AND chosen_index IS NOT NULL
             AND assisted = FALSE
           GROUP BY chosen_index ORDER BY chosen_index"#,
        vid
    )
    .fetch_all(&state.pool)
    .await?;
    let revealed = totals.attempts >= community_min_sample;
    let correct_rate_percent = if revealed && totals.attempts > 0 {
        Some((totals.correct * 100 / totals.attempts) as i32)
    } else {
        None
    };
    Ok(Json(serde_json::json!({
        "min_sample": community_min_sample,
        "attempts": totals.attempts,
        "revealed": revealed,
        "correct_rate_percent": correct_rate_percent,
        "option_distribution": if revealed {
            serde_json::Value::from(distribution.iter().map(|d| serde_json::json!({
                "index": d.chosen_index,
                "picks": d.picks,
            })).collect::<Vec<_>>())
        } else {
            serde_json::Value::Null
        },
    })))
}
