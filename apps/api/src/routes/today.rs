//! PLAN-01/AI-02: today's plan with cold start, honest learner evidence, and
//! revision undo (AI-07).

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::agent;
use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/NextActionRequest.ts",
        rename = "NextActionRequest"
    )
)]
pub struct NextActionQuery {
    available_minutes: i32,
    #[serde(default = "default_activity_preference")]
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"any\" | \"practice\" | \"revision\"", optional)
    )]
    activity_preference: String,
    #[serde(default = "default_time_multiplier")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    time_multiplier: f64,
}

fn default_activity_preference() -> String {
    "any".to_owned()
}

fn default_time_multiplier() -> f64 {
    1.0
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/RecommendedAction.ts",
        rename = "RecommendedAction"
    )
)]
pub struct RecommendedAction {
    task_id: Uuid,
    task_key: Uuid,
    kind: String,
    title: String,
    chapter_id: Option<Uuid>,
    source_session_id: Option<Uuid>,
    question_count: i32,
    estimated_minutes: i32,
    adjusted_estimated_minutes: i32,
    protected: bool,
    reason_code: String,
    independent_count: i64,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/NextActionAllowance.ts",
        rename = "NextActionAllowance"
    )
)]
pub struct NextActionAllowance {
    limit: i64,
    used: i64,
    remaining: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number | null"))]
    required: Option<i64>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/NextActionDeadlinePassed.ts",
        rename = "NextActionDeadlinePassed"
    )
)]
pub struct NextActionDeadlinePassed {
    available_minutes: i32,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"any\" | \"practice\" | \"revision\"")
    )]
    activity_preference: String,
    time_multiplier: f64,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    exam_date: Option<chrono::NaiveDate>,
    reason_code: String,
    recommended_action: Option<RecommendedAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    allowance: Option<NextActionAllowance>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/NextActionNoPlan.ts",
        rename = "NextActionNoPlan"
    )
)]
pub struct NextActionNoPlan {
    available_minutes: i32,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"any\" | \"practice\" | \"revision\"")
    )]
    activity_preference: String,
    time_multiplier: f64,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    exam_date: Option<chrono::NaiveDate>,
    plan_id: Option<Uuid>,
    plan_version: Option<i32>,
    reason_code: String,
    recommended_action: Option<RecommendedAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    allowance: Option<NextActionAllowance>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/NextActionNoAction.ts",
        rename = "NextActionNoAction"
    )
)]
pub struct NextActionNoAction {
    available_minutes: i32,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"any\" | \"practice\" | \"revision\"")
    )]
    activity_preference: String,
    time_multiplier: f64,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    exam_date: Option<chrono::NaiveDate>,
    plan_id: Uuid,
    plan_version: i32,
    reason_code: String,
    allowance: Option<NextActionAllowance>,
    recommended_action: Option<RecommendedAction>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/NextActionWithAction.ts",
        rename = "NextActionWithAction"
    )
)]
pub struct NextActionWithAction {
    available_minutes: i32,
    #[cfg_attr(
        feature = "type-export",
        ts(type = "\"any\" | \"practice\" | \"revision\"")
    )]
    activity_preference: String,
    time_multiplier: f64,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    exam_date: Option<chrono::NaiveDate>,
    plan_id: Uuid,
    plan_version: i32,
    reason_code: String,
    recommended_action: RecommendedAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "type-export", ts(optional))]
    allowance: Option<NextActionAllowance>,
}

#[derive(Serialize)]
#[serde(untagged)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/NextActionRecommendation.ts",
        rename = "NextActionRecommendation"
    )
)]
pub enum NextActionRecommendation {
    DeadlinePassed(NextActionDeadlinePassed),
    NoPlan(NextActionNoPlan),
    NoAction(NextActionNoAction),
    WithAction(NextActionWithAction),
}

/// GET /v1/me/plan/next-action returns one bounded suggestion from the
/// learner's current plan. It does not revise or otherwise mutate the plan.
pub async fn next_action(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query): Query<NextActionQuery>,
) -> ApiResult<Json<NextActionRecommendation>> {
    if !(5..=480).contains(&query.available_minutes) {
        return Err(ApiError::unprocessable(
            "invalid_time_budget",
            "available_minutes must be 5-480",
        ));
    }
    if !matches!(
        query.activity_preference.as_str(),
        "any" | "practice" | "revision"
    ) {
        return Err(ApiError::unprocessable(
            "invalid_activity_preference",
            "activity_preference must be any, practice, or revision",
        ));
    }
    if !query.time_multiplier.is_finite() || !(1.0..=4.0).contains(&query.time_multiplier) {
        return Err(ApiError::unprocessable(
            "invalid_time_multiplier",
            "time_multiplier must be between 1 and 4",
        ));
    }
    let free_daily_questions = crate::routes::settings::current_bounded_i64(
        &state.pool,
        "free_daily_questions",
        state.free_daily_questions,
        0,
        5000,
    )
    .await?;

    let goal = sqlx::query!(
        r#"SELECT exam_date, (exam_date < CURRENT_DATE) AS "deadline_passed!"
           FROM goals
           WHERE user_id = $1 AND NOT retired AND exam_date IS NOT NULL
           ORDER BY created_at DESC LIMIT 1"#,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let exam_date = goal.as_ref().and_then(|goal| goal.exam_date);
    if goal.as_ref().is_some_and(|goal| goal.deadline_passed) {
        return Ok(Json(NextActionRecommendation::DeadlinePassed(
            NextActionDeadlinePassed {
                available_minutes: query.available_minutes,
                activity_preference: query.activity_preference,
                time_multiplier: query.time_multiplier,
                exam_date,
                reason_code: "exam_deadline_passed".to_owned(),
                recommended_action: None,
                allowance: None,
            },
        )));
    }

    // A recommendation is strictly read-only: a missing cold-start plan is
    // left for GET /v1/me/today to create.
    let plan = sqlx::query!(
        "SELECT id, version FROM plans
         WHERE user_id = $1 AND plan_date = CURRENT_DATE
         ORDER BY version DESC LIMIT 1",
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let Some(plan) = plan else {
        return Ok(Json(NextActionRecommendation::NoPlan(NextActionNoPlan {
            available_minutes: query.available_minutes,
            activity_preference: query.activity_preference,
            time_multiplier: query.time_multiplier,
            exam_date,
            plan_id: None,
            plan_version: None,
            reason_code: "no_current_plan".to_owned(),
            recommended_action: None,
            allowance: None,
        })));
    };

    let is_free_tier: bool = sqlx::query_scalar("SELECT tier = 'free' FROM users WHERE id = $1")
        .bind(user.user_id)
        .fetch_one(&state.pool)
        .await?;
    let attempted_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM attempts a
         JOIN practice_sessions s ON s.id = a.session_id
         WHERE a.user_id = $1 AND a.created_at::date = CURRENT_DATE
           AND s.preset <> 'revision'",
    )
    .bind(user.user_id)
    .fetch_one(&state.pool)
    .await?;
    let candidates = sqlx::query!(
        r#"WITH eligible_questions AS (
               SELECT qv.id, qv.chapter_id
               FROM question_versions qv
               WHERE qv.status = 'published'
                 AND NOT EXISTS (
                     SELECT 1 FROM question_reports r
                     WHERE r.question_version_id = qv.id AND r.status = 'quarantined'
                 )
                 AND NOT EXISTS (
                     SELECT 1 FROM reserved_questions rq
                     WHERE rq.question_version_id = qv.id
                 )
                 AND EXISTS (
                     SELECT 1 FROM content_rights rights
                     WHERE rights.ref_code = UPPER(BTRIM(qv.rights_ref))
                       AND rights.revoked_at IS NULL
                       AND rights.valid_from <= CURRENT_DATE
                       AND (rights.valid_to IS NULL OR rights.valid_to >= CURRENT_DATE)
                       AND rights.permitted_uses @> '["display"]'::jsonb
                       AND (rights.audiences = '[]'::jsonb OR EXISTS (
                           SELECT 1
                           FROM jsonb_array_elements_text(rights.audiences) AS audiences(audience)
                           WHERE LOWER(BTRIM(audiences.audience)) IN ('learners', 'all')
                       ))
                       -- No seat-allocation ledger exists, so capped grants fail closed.
                       AND rights.seat_limit IS NULL
                       AND rights.asset_refs @> jsonb_build_array(qv.source_ref)
                       AND NOT EXISTS (
                           SELECT 1
                           FROM unnest(qv.source_refs || qv.media_refs) AS refs(asset_ref)
                           WHERE NOT (rights.asset_refs @> jsonb_build_array(refs.asset_ref))
                       )
                 )
           )
           SELECT t.id, t.task_key, t.kind, t.title, t.chapter_id, t.source_session_id,
                  t.question_count, t.estimated_minutes, t.protected,
                  CASE t.kind
                      WHEN 'practice' THEN (
                          SELECT COUNT(*)
                          FROM eligible_questions qv
                          WHERE qv.chapter_id = t.chapter_id
                      )
                      WHEN 'revision' THEN (
                          SELECT COUNT(DISTINCT qv.id)
                          FROM eligible_questions qv
                          WHERE t.source_session_id IS NOT NULL
                            AND EXISTS (
                                SELECT 1 FROM practice_sessions source
                                WHERE source.id = t.source_session_id
                                  AND source.user_id = $1 AND source.status = 'submitted'
                            )
                            AND (
                                EXISTS (
                                    SELECT 1 FROM attempts a
                                    WHERE a.session_id = t.source_session_id
                                      AND a.question_version_id = qv.id
                                      AND (a.correct = FALSE OR a.chosen_index IS NULL)
                                )
                                OR EXISTS (
                                    SELECT 1 FROM session_items si
                                    WHERE si.session_id = t.source_session_id
                                      AND si.question_version_id = qv.id
                                      AND NOT EXISTS (
                                          SELECT 1 FROM attempts a
                                          WHERE a.session_id = si.session_id
                                            AND a.item_index = si.item_index
                                      )
                                )
                            )
                      )
                      ELSE 0
                  END AS "eligible_question_count!",
                  COALESCE(evidence.independent_count, 0) AS "independent_count!",
                  COALESCE(evidence.correct_count, 0) AS "correct_count!"
           FROM plan_tasks t
           LEFT JOIN LATERAL (
               SELECT COUNT(*) AS independent_count,
                      COUNT(*) FILTER (WHERE a.correct = TRUE) AS correct_count
               FROM attempts a
               JOIN question_versions qv ON qv.id = a.question_version_id
               WHERE a.user_id = $1 AND qv.chapter_id = t.chapter_id
                 AND a.assisted = FALSE AND a.correct IS NOT NULL
           ) evidence ON TRUE
           WHERE t.plan_id = $2 AND t.status = 'pending'
           ORDER BY t.created_at, t.id"#,
        user.user_id,
        plan.id
    )
    .fetch_all(&state.pool)
    .await?;

    let time_fitting: Vec<_> = candidates
        .into_iter()
        .filter_map(|task| {
            let adjusted_minutes =
                (f64::from(task.estimated_minutes) * query.time_multiplier).ceil() as i32;
            (adjusted_minutes <= query.available_minutes).then_some((task, adjusted_minutes))
        })
        .collect();
    let has_preference_match = time_fitting.iter().any(|(task, _)| {
        task.protected
            || query.activity_preference == "any"
            || task.kind == query.activity_preference
    });
    let preference_blocked = !time_fitting.is_empty() && !has_preference_match;
    let preferred_fitting: Vec<_> = time_fitting
        .into_iter()
        .filter(|(task, _)| {
            task.protected
                || query.activity_preference == "any"
                || task.kind == query.activity_preference
        })
        .collect();
    let content_blocked = !preferred_fitting.is_empty()
        && preferred_fitting
            .iter()
            .all(|(task, _)| task.eligible_question_count < i64::from(task.question_count));
    let available: Vec<_> = preferred_fitting
        .into_iter()
        .filter(|(task, _)| task.eligible_question_count >= i64::from(task.question_count))
        .collect();
    let practice_question_counts: Vec<i64> = available
        .iter()
        .filter(|(task, _)| task.kind == "practice")
        .map(|(task, _)| i64::from(task.question_count))
        .collect();
    let remaining_free_questions = free_daily_questions.saturating_sub(attempted_today).max(0);
    let entitlement_blocked =
        is_free_tier && remaining_free_questions == 0 && !practice_question_counts.is_empty();
    let allowance_insufficient = is_free_tier
        && remaining_free_questions > 0
        && !practice_question_counts.is_empty()
        && practice_question_counts
            .iter()
            .all(|required| *required > remaining_free_questions);
    let candidates: Vec<_> = available
        .into_iter()
        .filter(|(task, _)| {
            task.kind == "revision"
                || task.kind != "practice"
                || !is_free_tier
                || i64::from(task.question_count) <= remaining_free_questions
        })
        .collect();

    let protected = candidates.iter().find(|(task, _)| task.protected);
    let revision = candidates.iter().find(|(task, _)| task.kind == "revision");
    let evidence_based = candidates
        .iter()
        .filter(|(task, _)| task.kind == "practice" && task.independent_count >= 10)
        .min_by(|(left, _), (right, _)| {
            (i128::from(left.correct_count) * i128::from(right.independent_count))
                .cmp(&(i128::from(right.correct_count) * i128::from(left.independent_count)))
        });
    let selected = protected
        .or(revision)
        .or(evidence_based)
        .or_else(|| candidates.first());

    let Some((task, adjusted_estimated_minutes)) = selected else {
        let reason_code = if entitlement_blocked {
                "free_allowance_reached"
            } else if allowance_insufficient {
                "free_allowance_insufficient"
            } else if preference_blocked {
                "activity_preference_unavailable"
            } else if content_blocked {
                "content_unavailable"
            } else {
                "no_task_fits"
            }
            .to_owned();
        let allowance = (entitlement_blocked || allowance_insufficient).then(|| {
            NextActionAllowance {
                limit: free_daily_questions,
                used: attempted_today,
                remaining: remaining_free_questions,
                required: practice_question_counts.iter().min().copied(),
            }
        });
        return Ok(Json(NextActionRecommendation::NoAction(NextActionNoAction {
            available_minutes: query.available_minutes,
            activity_preference: query.activity_preference,
            time_multiplier: query.time_multiplier,
            exam_date,
            plan_id: plan.id,
            plan_version: plan.version,
            reason_code,
            allowance,
            recommended_action: None,
        })));
    };

    let reason_code = if task.protected {
        "protected_task"
    } else if task.kind == "revision" {
        "missed_question_revision"
    } else if task.independent_count >= 10 {
        "lower_observed_accuracy"
    } else {
        "current_plan_order"
    };
    let recommendation = RecommendedAction {
        task_id: task.id,
        task_key: task.task_key,
        kind: task.kind.clone(),
        title: task.title.clone(),
        chapter_id: task.chapter_id,
        source_session_id: task.source_session_id,
        question_count: task.question_count,
        estimated_minutes: task.estimated_minutes,
        adjusted_estimated_minutes,
        protected: task.protected,
        reason_code: reason_code.to_owned(),
        independent_count: task.independent_count,
    };
    Ok(Json(NextActionRecommendation::WithAction(
        NextActionWithAction {
            available_minutes: query.available_minutes,
            activity_preference: query.activity_preference,
            time_multiplier: query.time_multiplier,
            exam_date,
            plan_id: plan.id,
            plan_version: plan.version,
            reason_code: reason_code.to_owned(),
            recommended_action: recommendation,
            allowance: None,
        },
    )))
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "today/TodayTask.ts", rename = "TodayTask")
)]
pub struct TaskView {
    id: Uuid,
    task_key: Uuid,
    kind: String,
    title: String,
    chapter_id: Option<Uuid>,
    source_session_id: Option<Uuid>,
    question_count: i32,
    estimated_minutes: i32,
    status: String,
    protected: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "today/TodayRevision.ts", rename = "TodayRevision")
)]
pub struct RevisionView {
    id: Uuid,
    to_version: i32,
    reason_code: String,
    explanation: String,
    automatic: bool,
    undone: bool,
    deferred_tasks: Vec<String>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/LearnerChapter.ts",
        rename = "LearnerChapter"
    )
)]
pub struct LearnerChapter {
    chapter_id: Uuid,
    chapter_name: String,
    /// Observed independent accuracy, 0-100. Null under the evidence floor —
    /// never a prediction (ADR 0004, AI-02).
    mastery_index: Option<i32>,
    evidence_level: &'static str,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    independent_count: i64,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "today/Today.ts", rename = "Today")
)]
pub struct TodayResponse {
    plan_id: Uuid,
    version: i32,
    tasks: Vec<TaskView>,
    revisions: Vec<RevisionView>,
    learner: Vec<LearnerChapter>,
    revision_budget: RevisionBudget,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "today/TodayRevisionBudget.ts",
        rename = "TodayRevisionBudget"
    )
)]
pub struct RevisionBudget {
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    automatic_used: i64,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    automatic_limit: i64,
    total_used: i32,
    total_limit: i32,
}

pub async fn today(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<TodayResponse>> {
    let (plan_id, version) = agent::get_or_create_today(&state.pool, user.user_id).await?;
    let task_rows = sqlx::query!(
        "SELECT id, task_key, kind, title, chapter_id, source_session_id, question_count,
                estimated_minutes, status, protected
         FROM plan_tasks
         WHERE plan_id = $1 ORDER BY created_at, id",
        plan_id
    )
    .fetch_all(&state.pool)
    .await?;
    let revision_rows = sqlx::query!(
        r#"SELECT r.id, r.to_version, r.reason_code, r.explanation, r.automatic, r.undone,
                  r.receipt::text AS "receipt_text!"
           FROM plan_revisions r JOIN plans p ON p.id = r.plan_id
           WHERE p.user_id = $1 AND p.plan_date = CURRENT_DATE
           ORDER BY r.created_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let learner_rows = sqlx::query!(
        r#"SELECT qv.chapter_id, c.name AS chapter_name,
                  COALESCE(COUNT(*), 0) AS "independent_count!",
                  COALESCE(COUNT(*) FILTER (WHERE a.correct = TRUE), 0) AS "correct!"
           FROM attempts a
           JOIN question_versions qv ON qv.id = a.question_version_id
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE a.user_id = $1 AND a.assisted = FALSE AND a.correct IS NOT NULL
           GROUP BY qv.chapter_id, c.name
           ORDER BY c.name"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;

    let automatic_revisions_used = revision_rows.iter().filter(|r| r.automatic).count() as i64;
    let total_revisions_used = revision_rows.len() as i32;
    let tasks = task_rows
        .into_iter()
        .map(|t| TaskView {
            id: t.id,
            task_key: t.task_key,
            kind: t.kind,
            title: t.title,
            chapter_id: t.chapter_id,
            source_session_id: t.source_session_id,
            question_count: t.question_count,
            estimated_minutes: t.estimated_minutes,
            status: t.status,
            protected: t.protected,
        })
        .collect();
    let revisions = revision_rows
        .into_iter()
        .map(|r| RevisionView {
            id: r.id,
            to_version: r.to_version,
            reason_code: r.reason_code,
            explanation: r.explanation,
            automatic: r.automatic,
            undone: r.undone,
            deferred_tasks: serde_json::from_str::<serde_json::Value>(&r.receipt_text)
                .ok()
                .and_then(|receipt| {
                    receipt
                        .get("deferred")
                        .and_then(serde_json::Value::as_array)
                        .map(|tasks| {
                            tasks
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .map(str::to_owned)
                                .collect()
                        })
                })
                .unwrap_or_default(),
        })
        .collect();
    let learner = learner_rows
        .into_iter()
        .map(|r| {
            // §8.8: show the evidence level beside the index; under roughly 10
            // independent attempts there is no meaningful index at all.
            let (mastery_index, evidence_level) = if r.independent_count < 10 {
                (None, "low_evidence")
            } else if r.independent_count < 30 {
                (
                    Some((r.correct * 100 / r.independent_count) as i32),
                    "developing",
                )
            } else {
                (
                    Some((r.correct * 100 / r.independent_count) as i32),
                    "established",
                )
            };
            LearnerChapter {
                chapter_id: r.chapter_id,
                chapter_name: r.chapter_name,
                mastery_index,
                evidence_level,
                independent_count: r.independent_count,
            }
        })
        .collect();

    Ok(Json(TodayResponse {
        plan_id,
        version,
        tasks,
        revisions,
        learner,
        revision_budget: RevisionBudget {
            automatic_used: automatic_revisions_used,
            automatic_limit: agent::MAX_AUTOMATIC_PLAN_REVISIONS_PER_DAY,
            total_used: total_revisions_used,
            total_limit: agent::MAX_PLAN_REVISIONS_PER_DAY,
        },
    }))
}

#[derive(serde::Deserialize)]
pub struct TaskProtectionReq {
    protected: bool,
}

pub async fn set_task_protection(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((plan_id, task_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<TaskProtectionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 FOR UPDATE",
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let current = sqlx::query!(
        "SELECT id, version FROM plans
         WHERE user_id = $1 AND plan_date = CURRENT_DATE
         ORDER BY version DESC LIMIT 1 FOR UPDATE",
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if current.id != plan_id {
        return Err(ApiError::conflict_with_details(
            "stale_plan_version",
            "Your plan changed. Refresh it before changing task protection.",
            json!({ "current_plan_id": current.id, "current_version": current.version }),
        ));
    }
    let task = sqlx::query!(
        "SELECT status, protected FROM plan_tasks
         WHERE id = $1 AND plan_id = $2 FOR UPDATE",
        task_id,
        plan_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("plan_task_not_found"))?;
    if task.status != "pending" {
        return Err(ApiError::conflict(
            "task_not_pending",
            "Only pending tasks can be protected.",
        ));
    }
    if task.protected != req.protected {
        sqlx::query!(
            "UPDATE plan_tasks SET protected = $3 WHERE id = $1 AND plan_id = $2",
            task_id,
            plan_id,
            req.protected
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(
        json!({ "task_id": task_id, "protected": req.protected }),
    ))
}

#[derive(Serialize)]
pub struct UndoResponse {
    plan_version: i32,
}

async fn deferred_task_ids_on(
    conn: &mut PgConnection,
    user_id: Uuid,
    from_version: i32,
    receipt: &serde_json::Value,
) -> ApiResult<Vec<Uuid>> {
    if let Some(ids) = receipt.get("deferred_task_ids") {
        return ids
            .as_array()
            .ok_or_else(ApiError::internal)?
            .iter()
            .map(|id| {
                id.as_str()
                    .and_then(|id| Uuid::parse_str(id).ok())
                    .ok_or_else(ApiError::internal)
            })
            .collect();
    }

    // Receipts written before AI-08 stored deferred titles only. Resolve that
    // title multiset against the immutable source plan so old Undo buttons
    // continue to restore the exact number of rows they removed.
    let titles = receipt
        .get("deferred")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            ApiError::conflict(
                "legacy_revision_receipt_invalid",
                "this older revision has no deferred-task details to restore",
            )
        })?
        .iter()
        .map(|title| {
            title.as_str().map(str::to_owned).ok_or_else(|| {
                ApiError::conflict(
                    "legacy_revision_receipt_invalid",
                    "this older revision has invalid deferred-task details",
                )
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    if titles.is_empty() {
        return Ok(Vec::new());
    }
    let source_plan_id = sqlx::query_scalar!(
        "SELECT id FROM plans
         WHERE user_id = $1 AND plan_date = CURRENT_DATE AND version = $2",
        user_id,
        from_version
    )
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(ApiError::internal)?;
    let rows = sqlx::query!(
        "SELECT id, title FROM plan_tasks WHERE plan_id = $1 ORDER BY created_at, id",
        source_plan_id
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut remaining = HashMap::<String, usize>::new();
    for title in titles {
        *remaining.entry(title).or_default() += 1;
    }
    let mut ids = Vec::new();
    for row in rows {
        if let Some(count) = remaining.get_mut(&row.title) {
            if *count > 0 {
                ids.push(row.id);
                *count -= 1;
            }
        }
    }
    if remaining.values().any(|count| *count > 0) {
        return Err(ApiError::conflict(
            "legacy_revision_receipt_invalid",
            "some deferred tasks from this older revision are no longer available to restore",
        ));
    }
    Ok(ids)
}

pub async fn undo_revision(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((pid, rid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<UndoResponse>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 FOR UPDATE",
        user.user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let plan = sqlx::query!(
        "SELECT id, version FROM plans
         WHERE user_id = $1 AND plan_date = CURRENT_DATE
         ORDER BY version DESC LIMIT 1 FOR UPDATE",
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("plan_not_found"))?;
    if plan.id != pid {
        return Err(ApiError::conflict_with_details(
            "stale_plan_version",
            "Your plan changed. Refresh it before undoing a revision.",
            json!({ "current_plan_id": plan.id, "current_version": plan.version }),
        ));
    }
    let revision = sqlx::query!(
        "SELECT id, undone, reason_code, from_version,
                receipt::text AS \"receipt_text!\"
         FROM plan_revisions WHERE id = $1 AND plan_id = $2",
        rid,
        pid
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("revision_not_found"))?;
    if revision.undone {
        return Err(ApiError::conflict(
            "already_undone",
            "revision is already undone",
        ));
    }
    // Only the newest revision can be undone (older ones would silently
    // resurrect tasks removed by later revisions).
    let latest = sqlx::query!(
        r#"SELECT r.id FROM plan_revisions r JOIN plans p ON p.id = r.plan_id
           WHERE p.user_id = $1 AND p.plan_date = CURRENT_DATE
           ORDER BY r.to_version DESC, p.version DESC LIMIT 1"#,
        user.user_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if latest.map(|l| l.id) != Some(revision.id) {
        return Err(ApiError::conflict(
            "not_latest_revision",
            "only the most recent revision can be undone",
        ));
    }

    let (new_plan_id, new_version) = agent::fork_plan_on(&mut tx, plan.id, plan.version).await?;
    sqlx::query!(
        "DELETE FROM plan_tasks WHERE plan_id = $1 AND added_by_revision = $2",
        new_plan_id,
        rid
    )
    .execute(&mut *tx)
    .await?;
    if revision.reason_code == "capacity_change" {
        let receipt: serde_json::Value =
            serde_json::from_str(&revision.receipt_text).map_err(|_| ApiError::internal())?;
        let deferred_task_ids =
            deferred_task_ids_on(&mut tx, user.user_id, revision.from_version, &receipt).await?;
        if !deferred_task_ids.is_empty() {
            let source_plan = sqlx::query!(
                "SELECT id FROM plans
                 WHERE user_id = $1 AND plan_date = CURRENT_DATE AND version = $2",
                user.user_id,
                revision.from_version
            )
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(ApiError::internal)?;
            let restored = sqlx::query!(
                "INSERT INTO plan_tasks
                   (id, plan_id, kind, title, chapter_id, question_count,
                    estimated_minutes, source_session_id, status, added_by_revision,
                    protected, task_key, created_at)
                 SELECT gen_random_uuid(), $1, kind, title, chapter_id, question_count,
                        estimated_minutes, source_session_id, status, added_by_revision,
                        protected, task_key, created_at
                 FROM plan_tasks WHERE plan_id = $2 AND id = ANY($3)
                 ORDER BY created_at, id",
                new_plan_id,
                source_plan.id,
                &deferred_task_ids
            )
            .execute(&mut *tx)
            .await?;
            if restored.rows_affected() != deferred_task_ids.len() as u64 {
                return Err(ApiError::internal());
            }
        }
    }
    let stamp = json!({ "undone": true, "undone_at": chrono::Utc::now().to_rfc3339() });
    sqlx::query!(
        "UPDATE plan_revisions SET undone = true, receipt = receipt || $2::jsonb WHERE id = $1",
        rid,
        stamp
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(UndoResponse {
        plan_version: new_version,
    }))
}

// ---- QB-12: learner-facing curriculum for the session builder ---------------

/// Chapters with their system/subject context — the multi-select source for
/// the practice builder. Read-only, learner-scoped, no admin gate.
pub async fn my_curriculum(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT ch.id AS chapter_id, ch.name AS chapter_name,
                  sys.name AS system_name, sub.name AS subject_name,
                  e.id AS exam_id, e.name AS exam_name,
                  (SELECT COUNT(*) FROM question_versions qv
                   WHERE qv.chapter_id = ch.id AND qv.status = 'published') AS "published!"
           FROM curriculum_nodes ch
           JOIN curriculum_nodes sys ON sys.id = ch.parent_id
           JOIN curriculum_nodes sub ON sub.id = sys.parent_id
           JOIN exams e ON e.id = ch.exam_id
           WHERE ch.kind = 'chapter'
           ORDER BY e.name, sub.display_order, sys.display_order, ch.display_order"#
    )
    .fetch_all(&state.pool)
    .await?;
    let chapters: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "chapter_id": r.chapter_id,
                "chapter_name": r.chapter_name,
                "system": r.system_name,
                "subject": r.subject_name,
                "exam_id": r.exam_id,
                "exam": r.exam_name,
                "published_questions": r.published,
            })
        })
        .collect();
    Ok(Json(json!({ "chapters": chapters })))
}
