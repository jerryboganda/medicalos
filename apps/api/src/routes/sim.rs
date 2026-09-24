//! SIM-06/CAREER-02 adjacent: scenario debriefs and appeals. A debrief is
//! the run's own transcript plus outcome — evidence, not commentary.
//! Appeals record the learner's challenge; a human reviewer resolves it
//! (SIM-07) — the route records, never auto-decides.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(sqlx::FromRow)]
struct ScenarioDebriefRow {
    current_state: String,
    transcript: serde_json::Value,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
    title: String,
    version: i32,
    scenario_version_id: Uuid,
    state_machine: serde_json::Value,
}

#[derive(sqlx::FromRow)]
struct RubricResultRow {
    criterion_key: String,
    label: String,
    max_score: f32,
    assessment_status: Option<String>,
    evidence: Option<String>,
    score: Option<f32>,
    transcript_event_indexes: Option<serde_json::Value>,
    transcript_uncertain: Option<bool>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

async fn rubric_results(
    state: &AppState,
    run_id: Uuid,
    scenario_version_id: Uuid,
) -> ApiResult<Vec<serde_json::Value>> {
    let rows = sqlx::query_as::<_, RubricResultRow>(
        r#"SELECT r.criterion_key, r.label, r.max_score,
                  e.assessment_status, e.evidence, e.score,
                  e.transcript_event_indexes, e.transcript_uncertain, e.created_at
           FROM scenario_rubrics r
           LEFT JOIN scenario_rubric_evidence e
             ON e.run_id = $1 AND e.criterion_key = r.criterion_key
           WHERE r.scenario_version_id = $2
           ORDER BY r.criterion_key"#,
    )
    .bind(run_id)
    .bind(scenario_version_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "criterion_key": row.criterion_key,
                "label": row.label,
                "max_score": row.max_score,
                "assessment_status": row.assessment_status.unwrap_or_else(|| "not_assessed".into()),
                "evidence": row.evidence,
                "score": row.score,
                "transcript_event_indexes": row.transcript_event_indexes.unwrap_or_else(|| json!([])),
                "transcript_uncertain": row.transcript_uncertain.unwrap_or(false),
                "reviewed_at": row.created_at,
            })
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct ScenarioAppealStatusRow {
    appeal_id: Uuid,
    reason: String,
    created_at: chrono::DateTime<chrono::Utc>,
    decision: Option<String>,
    rationale: Option<String>,
    reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn debrief(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let run = sqlx::query_as::<_, ScenarioDebriefRow>(
        r#"SELECT r.current_state, r.transcript, r.started_at, r.finished_at,
                  s.title, sv.version, r.scenario_version_id, sv.state_machine
           FROM scenario_runs r
           JOIN scenarios s ON s.id = r.scenario_id
           JOIN scenario_versions sv ON sv.id = r.scenario_version_id
           WHERE r.id = $1
             AND (r.user_id = $2 OR EXISTS (
               SELECT 1 FROM scenario_team_members member
               WHERE member.run_id = r.id AND member.user_id = $2
             ))"#,
    )
    .bind(run_id)
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    let rubric = rubric_results(&state, run_id, run.scenario_version_id).await?;
    let appeal = sqlx::query_as::<_, ScenarioAppealStatusRow>(
        r#"SELECT appeal.id AS appeal_id, appeal.reason, appeal.created_at,
		          review.decision, review.rationale, review.created_at AS reviewed_at
		   FROM scenario_assessment_appeals appeal
		   LEFT JOIN scenario_assessment_appeal_reviews review ON review.appeal_id = appeal.id
		   WHERE appeal.run_id = $1 AND appeal.appellant_id = $2"#,
    )
    .bind(run_id)
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?
    .map(|appeal| {
        json!({
            "appeal_id": appeal.appeal_id,
            "reason": appeal.reason,
            "status": if appeal.decision.is_some() { "reviewed" } else { "open" },
            "decision": appeal.decision,
            "rationale": appeal.rationale,
            "created_at": appeal.created_at,
            "reviewed_at": appeal.reviewed_at,
        })
    });
    let consequential_use_status = match appeal
        .as_ref()
        .and_then(|appeal| appeal.get("decision"))
        .and_then(serde_json::Value::as_str)
    {
        Some("reassessment_required") => "reassessment_required",
        _ => "not_authorized_by_assessment",
    };
    Ok(Json(json!({
        "scenario": run.title,
        "scenario_version": run.version,
        "final_state": run.current_state,
        "transcript": run.transcript,
        "timeline": crate::routes::program::transcript_timeline(&run.transcript),
        "transcript_corrections": sqlx::query!(
            r#"SELECT event_index, original_event, corrected_text, corrected_by, created_at
               FROM scenario_transcript_corrections
               WHERE run_id = $1 ORDER BY event_index"#,
            run_id
        )
        .fetch_all(&state.pool)
        .await?
        .iter()
        .map(|c| {
            json!({
                "event_index": c.event_index,
                "original_event": c.original_event,
                "corrected_text": c.corrected_text,
                "corrected_by": c.corrected_by,
                "created_at": c.created_at,
            })
        })
        .collect::<Vec<_>>(),
        "available_actions": crate::routes::program::scenario_events(&run.state_machine, None),
        "started_at": run.started_at,
        "finished_at": run.finished_at,
        "rubric": rubric,
        "appeal": appeal,
        "consequential_use_status": consequential_use_status,
    })))
}

#[derive(Deserialize)]
pub struct CounterfactualReplayReq {
    pub events: Vec<String>,
}

#[derive(sqlx::FromRow)]
struct CounterfactualRun {
    version: i32,
    state_machine: serde_json::Value,
    transcript: serde_json::Value,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn counterfactual_replay(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
    Json(req): Json<CounterfactualReplayReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.events.is_empty()
        || req.events.len() > 100
        || req.events.iter().any(|event| {
            let event = event.trim();
            event.is_empty() || event.chars().count() > 120 || event.chars().any(char::is_control)
        })
    {
        return Err(ApiError::unprocessable(
            "invalid_counterfactual_events",
            "replay requires 1-100 bounded event names",
        ));
    }
    let run = sqlx::query_as::<_, CounterfactualRun>(
        r#"SELECT sv.version, sv.state_machine,
		          r.transcript, r.finished_at
		   FROM scenario_runs r
		   JOIN scenario_versions sv ON sv.id = r.scenario_version_id
		   WHERE r.id = $1
		     AND (r.user_id = $2 OR EXISTS (
		       SELECT 1 FROM scenario_team_members member
		       WHERE member.run_id = r.id AND member.user_id = $2
		     ))"#,
    )
    .bind(run_id)
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    if run.finished_at.is_none() {
        return Err(ApiError::conflict(
            "run_not_finished",
            "counterfactual replay is available after the station is finished",
        ));
    }
    let terminal_states: HashSet<String> = run
        .state_machine
        .get("terminal_states")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect();
    let mut current = run
        .state_machine
        .get("initial")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(ApiError::internal)?
        .to_string();
    let mut timeline = Vec::with_capacity(req.events.len());
    for (index, event) in req.events.iter().enumerate() {
        if terminal_states.contains(current.as_str()) {
            return Err(ApiError::unprocessable(
                "counterfactual_after_terminal",
                format!("event {} follows a terminal state", index + 1),
            ));
        }
        let action = event.trim();
        let next = crate::routes::program::next_state(&run.state_machine, &current, action)
            .ok_or_else(|| {
                ApiError::unprocessable(
                    "counterfactual_transition_unavailable",
                    format!("event {} is not valid from state {current}", index + 1),
                )
            })?;
        let terminal = terminal_states.contains(next.as_str());
        timeline.push(json!({
            "index": index,
            "sequence": index + 1,
            "from": current.clone(),
            "on": action,
            "to": next.clone(),
            "terminal": terminal,
        }));
        current = next;
    }
    Ok(Json(json!({
        "run_id": run_id,
        "scenario_version": run.version,
        "original_timeline": crate::routes::program::transcript_timeline(&run.transcript),
        "counterfactual_timeline": timeline,
        "final_state": current,
        "terminal": terminal_states.contains(current.as_str()),
    })))
}

#[derive(sqlx::FromRow)]
struct AdminAssessmentRun {
    scenario_version_id: Uuid,
    transcript: serde_json::Value,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
    title: String,
    version: i32,
}

async fn admin_assessment_run(state: &AppState, run_id: Uuid) -> ApiResult<AdminAssessmentRun> {
    sqlx::query_as::<_, AdminAssessmentRun>(
        r#"SELECT r.scenario_version_id, r.transcript,
                  r.started_at, r.finished_at, s.title, sv.version
           FROM scenario_runs r
           JOIN scenarios s ON s.id = r.scenario_id
           JOIN scenario_versions sv ON sv.id = r.scenario_version_id
           WHERE r.id = $1"#,
    )
    .bind(run_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))
}

fn require_admin(state: &AppState, headers: &axum::http::HeaderMap) -> ApiResult<()> {
    state.require_admin(
        headers
            .get("x-admin-token")
            .and_then(|value| value.to_str().ok()),
    )
}

#[derive(sqlx::FromRow)]
struct PendingScenarioAssessment {
    run_id: Uuid,
    scenario: String,
    scenario_version: i32,
    finished_at: chrono::DateTime<chrono::Utc>,
    criterion_count: i64,
}

pub async fn pending_assessments(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&state, &headers)?;
    let rows = sqlx::query_as::<_, PendingScenarioAssessment>(
        r#"SELECT run.id AS run_id, scenario.title AS scenario,
                  version.version AS scenario_version, run.finished_at,
                  (SELECT COUNT(*) FROM scenario_rubrics rubric
                   WHERE rubric.scenario_version_id = run.scenario_version_id) AS criterion_count
           FROM scenario_runs run
           JOIN scenarios scenario ON scenario.id = run.scenario_id
           JOIN scenario_versions version ON version.id = run.scenario_version_id
           WHERE run.finished_at IS NOT NULL
             AND EXISTS (
                 SELECT 1 FROM scenario_rubrics rubric
                 WHERE rubric.scenario_version_id = run.scenario_version_id
             )
             AND NOT EXISTS (
                 SELECT 1 FROM scenario_rubric_evidence evidence
                 WHERE evidence.run_id = run.id
             )
           ORDER BY run.finished_at DESC, run.id
           LIMIT 100"#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "runs": rows.into_iter().map(|run| json!({
            "run_id": run.run_id,
            "scenario": run.scenario,
            "scenario_version": run.scenario_version,
            "finished_at": run.finished_at,
            "criterion_count": run.criterion_count,
        })).collect::<Vec<_>>()
    })))
}

pub async fn admin_assessment(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(run_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&state, &headers)?;
    let run = admin_assessment_run(&state, run_id).await?;
    if run.finished_at.is_none() {
        return Err(ApiError::conflict(
            "run_not_finished",
            "finish the station before assessment",
        ));
    }
    let rubric = rubric_results(&state, run_id, run.scenario_version_id).await?;
    Ok(Json(json!({
        "run_id": run_id,
        "scenario": run.title,
        "scenario_version": run.version,
        "transcript": run.transcript,
        "started_at": run.started_at,
        "finished_at": run.finished_at,
        "rubric": rubric,
    })))
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    Assessed,
    NotAssessed,
}

impl AssessmentStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Assessed => "assessed",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Deserialize)]
pub struct CriterionAssessmentReq {
    pub criterion_key: String,
    pub assessment_status: AssessmentStatus,
    pub score: Option<f32>,
    pub evidence: String,
    #[serde(default)]
    pub transcript_event_indexes: Vec<usize>,
    #[serde(default)]
    pub transcript_uncertain: bool,
}

#[derive(Deserialize)]
pub struct RecordAssessmentReq {
    pub criteria: Vec<CriterionAssessmentReq>,
}

#[derive(sqlx::FromRow)]
struct CriterionMaximum {
    criterion_key: String,
    max_score: f32,
}

#[derive(sqlx::FromRow)]
struct AssessmentTarget {
    user_id: Uuid,
    scenario_version_id: Uuid,
    transcript: serde_json::Value,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn record_assessment(
    State(state): State<Arc<AppState>>,
    reviewer: AuthUser,
    headers: axum::http::HeaderMap,
    Path(run_id): Path<Uuid>,
    Json(req): Json<RecordAssessmentReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers)?;
    let mut tx = state.pool.begin().await?;
    let run = sqlx::query_as::<_, AssessmentTarget>(
        r#"SELECT user_id, scenario_version_id, transcript, finished_at
           FROM scenario_runs
           WHERE id = $1
           FOR UPDATE"#,
    )
    .bind(run_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    if run.user_id == reviewer.user_id {
        return Err(ApiError::forbidden(
            "self_assessment_forbidden",
            "learners cannot act as their own examiner",
        ));
    }
    if run.finished_at.is_none() {
        return Err(ApiError::conflict(
            "run_not_finished",
            "finish the station before assessment",
        ));
    }
    let already_assessed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM scenario_rubric_evidence WHERE run_id = $1)",
    )
    .bind(run_id)
    .fetch_one(&mut *tx)
    .await?;
    if already_assessed {
        return Err(ApiError::conflict(
            "assessment_already_recorded",
            "this run already has an examiner assessment",
        ));
    }

    let rubric = sqlx::query_as::<_, CriterionMaximum>(
        "SELECT criterion_key, max_score FROM scenario_rubrics WHERE scenario_version_id = $1",
    )
    .bind(run.scenario_version_id)
    .fetch_all(&mut *tx)
    .await?;
    if rubric.is_empty() || req.criteria.len() != rubric.len() {
        return Err(ApiError::unprocessable(
            "criteria_mismatch",
            "submit exactly one result for every rubric criterion",
        ));
    }
    let maxima: HashMap<&str, f32> = rubric
        .iter()
        .map(|criterion| (criterion.criterion_key.as_str(), criterion.max_score))
        .collect();
    // SIM-03: transcript events marked uncertain stay uncertain for evidence
    // purposes until a correction covers them.
    let corrected_indexes: HashSet<usize> = sqlx::query_scalar(
        "SELECT event_index FROM scenario_transcript_corrections WHERE run_id = $1",
    )
    .bind(run_id)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|index: i32| index as usize)
    .collect();
    let uncertain_events: HashSet<i32> = run
        .transcript
        .as_array()
        .map(|events| {
            events
                .iter()
                .enumerate()
                .filter(|(_, event)| {
                    event.get("uncertain").and_then(serde_json::Value::as_bool) == Some(true)
                })
                .map(|(index, _)| index)
                .collect()
        })
        .unwrap_or_default();
    let mut seen = HashSet::new();
    let transcript_len = run.transcript.as_array().map_or(0, Vec::len);
    for criterion in &req.criteria {
        if !seen.insert(criterion.criterion_key.as_str()) {
            return Err(ApiError::unprocessable(
                "duplicate_criterion",
                "each criterion may be assessed once",
            ));
        }
        let Some(max_score) = maxima.get(criterion.criterion_key.as_str()) else {
            return Err(ApiError::unprocessable(
                "unknown_criterion",
                "criterion is not in this scenario version",
            ));
        };
        if criterion.evidence.trim().is_empty() || criterion.evidence.chars().count() > 2000 {
            return Err(ApiError::unprocessable(
                "invalid_assessment_evidence",
                "evidence or not-assessed reason must be 1-2000 characters",
            ));
        }
        match criterion.assessment_status {
            AssessmentStatus::Assessed => {
                if criterion
                    .score
                    .is_none_or(|score| !score.is_finite() || score < 0.0 || score > *max_score)
                {
                    return Err(ApiError::unprocessable(
                        "score_out_of_range",
                        "assessed scores must be within the criterion's allowed range",
                    ));
                }
                let cites_uncorrected_uncertain =
                    criterion.transcript_event_indexes.iter().any(|index| {
                        uncertain_events.contains(index) && !corrected_indexes.contains(index)
                    });
                if cites_uncorrected_uncertain && !criterion.transcript_uncertain {
                    return Err(ApiError::unprocessable(
                        "uncertain_transcript_evidence",
                        "this judgment cites an uncorrected uncertain transcript segment — acknowledge the uncertainty",
                    ));
                }
                if criterion.transcript_event_indexes.is_empty()
                    || criterion
                        .transcript_event_indexes
                        .iter()
                        .any(|index| *index >= transcript_len)
                    || !criterion
                        .transcript_event_indexes
                        .windows(2)
                        .all(|pair| pair[0] < pair[1])
                {
                    return Err(ApiError::unprocessable(
                        "invalid_transcript_evidence",
                        "an assessed result must cite ordered transcript events",
                    ));
                }
            }
            AssessmentStatus::NotAssessed => {
                if criterion.score.is_some()
                    || !criterion.transcript_event_indexes.is_empty()
                    || criterion.transcript_uncertain
                {
                    return Err(ApiError::unprocessable(
                        "invalid_not_assessed_result",
                        "not-assessed results have no score or transcript event reference",
                    ));
                }
            }
        }
    }

    let not_assessed = req
        .criteria
        .iter()
        .filter(|criterion| matches!(criterion.assessment_status, AssessmentStatus::NotAssessed))
        .count();
    for criterion in &req.criteria {
        sqlx::query(
            r#"INSERT INTO scenario_rubric_evidence
                   (id, run_id, criterion_key, evidence, score, transcript_uncertain,
                    assessment_status, reviewer_id, transcript_event_indexes)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(Uuid::new_v4())
        .bind(run_id)
        .bind(&criterion.criterion_key)
        .bind(criterion.evidence.trim())
        .bind(criterion.score)
        .bind(criterion.transcript_uncertain)
        .bind(criterion.assessment_status.as_str())
        .bind(reviewer.user_id)
        .bind(json!(criterion.transcript_event_indexes))
        .execute(&mut *tx)
        .await?;
    }
    crate::routes::admin::audit(
        &mut *tx,
        reviewer.user_id,
        "scenario_assessment_recorded",
        "scenario_run",
        run_id,
        json!({ "criteria": req.criteria.len(), "not_assessed": not_assessed }),
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "recorded_criteria": req.criteria.len(),
            "not_assessed": not_assessed
        })),
    ))
}

// ---- appeals (learner challenge; human review resolves, SIM-07) --------------

#[derive(Deserialize)]
pub struct ScenarioAssessmentAppealReq {
    pub reason: String,
}

pub async fn appeal_scenario_assessment(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
    Json(req): Json<ScenarioAssessmentAppealReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let reason = req.reason.trim();
    if reason.chars().count() < 10
        || reason.chars().count() > 2000
        || reason
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(ApiError::unprocessable(
            "invalid_appeal_reason",
            "appeal reason must be 10-2000 printable characters or line breaks",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let finished_at = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT finished_at FROM scenario_runs WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(run_id)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    if finished_at.is_none() {
        return Err(ApiError::conflict(
            "run_not_finished",
            "an assessment can be appealed after the station is finished",
        ));
    }
    let assessed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM scenario_rubric_evidence WHERE run_id = $1)",
    )
    .bind(run_id)
    .fetch_one(&mut *tx)
    .await?;
    if !assessed {
        return Err(ApiError::conflict(
            "assessment_not_recorded",
            "there is no examiner assessment to appeal",
        ));
    }
    let already_appealed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM scenario_assessment_appeals WHERE run_id = $1)",
    )
    .bind(run_id)
    .fetch_one(&mut *tx)
    .await?;
    if already_appealed {
        return Err(ApiError::conflict(
            "appeal_already_submitted",
            "this run already has an appeal",
        ));
    }
    let appeal_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO scenario_assessment_appeals (id, run_id, appellant_id, reason)
		 VALUES ($1, $2, $3, $4)",
    )
    .bind(appeal_id)
    .bind(run_id)
    .bind(user.user_id)
    .bind(reason)
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "scenario_assessment_appeal_submitted",
        "scenario_assessment_appeal",
        appeal_id,
        json!({"status":"open"}),
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"appeal_id": appeal_id, "status":"open"})),
    ))
}

#[derive(sqlx::FromRow)]
struct ScenarioAssessmentAppealQueueRow {
    appeal_id: Uuid,
    run_id: Uuid,
    scenario: String,
    scenario_version: i32,
    reason: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_scenario_assessment_appeals(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&state, &headers)?;
    let rows = sqlx::query_as::<_, ScenarioAssessmentAppealQueueRow>(
        r#"SELECT appeal.id AS appeal_id, appeal.run_id, scenario.title AS scenario,
		          version.version AS scenario_version, appeal.reason, appeal.created_at
		   FROM scenario_assessment_appeals appeal
		   JOIN scenario_runs run ON run.id = appeal.run_id
		   JOIN scenarios scenario ON scenario.id = run.scenario_id
		   JOIN scenario_versions version ON version.id = run.scenario_version_id
		   WHERE NOT EXISTS (
		       SELECT 1 FROM scenario_assessment_appeal_reviews review
		       WHERE review.appeal_id = appeal.id
		   )
		   ORDER BY appeal.created_at ASC
		   LIMIT 100"#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        json!({"appeals": rows.into_iter().map(|appeal| json!({
		"appeal_id": appeal.appeal_id,
		"run_id": appeal.run_id,
		"scenario": appeal.scenario,
		"scenario_version": appeal.scenario_version,
		"reason": appeal.reason,
		"created_at": appeal.created_at,
	})).collect::<Vec<_>>()}),
    ))
}

#[derive(sqlx::FromRow)]
struct ScenarioAssessmentAppealDetailRow {
    appeal_id: Uuid,
    run_id: Uuid,
    reason: String,
    created_at: chrono::DateTime<chrono::Utc>,
    scenario: String,
    scenario_version: i32,
    scenario_version_id: Uuid,
    transcript: serde_json::Value,
}

pub async fn get_scenario_assessment_appeal(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(appeal_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&state, &headers)?;
    let appeal = sqlx::query_as::<_, ScenarioAssessmentAppealDetailRow>(
        r#"SELECT appeal.id AS appeal_id, appeal.run_id, appeal.reason, appeal.created_at,
		          scenario.title AS scenario, version.version AS scenario_version,
		          version.id AS scenario_version_id, run.transcript
		   FROM scenario_assessment_appeals appeal
		   JOIN scenario_runs run ON run.id = appeal.run_id
		   JOIN scenarios scenario ON scenario.id = run.scenario_id
		   JOIN scenario_versions version ON version.id = run.scenario_version_id
		   WHERE appeal.id = $1"#,
    )
    .bind(appeal_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("scenario_assessment_appeal_not_found"))?;
    let rubric = rubric_results(&state, appeal.run_id, appeal.scenario_version_id).await?;
    Ok(Json(json!({
        "appeal_id": appeal.appeal_id,
        "run_id": appeal.run_id,
        "reason": appeal.reason,
        "created_at": appeal.created_at,
        "scenario": appeal.scenario,
        "scenario_version": appeal.scenario_version,
        "timeline": crate::routes::program::transcript_timeline(&appeal.transcript),
        "rubric": rubric,
    })))
}

#[derive(Deserialize)]
pub struct ScenarioAssessmentAppealReviewReq {
    pub decision: String,
    pub rationale: String,
}

pub async fn review_scenario_assessment_appeal(
    State(state): State<Arc<AppState>>,
    reviewer: AuthUser,
    headers: axum::http::HeaderMap,
    Path(appeal_id): Path<Uuid>,
    Json(req): Json<ScenarioAssessmentAppealReviewReq>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers)?;
    if !matches!(req.decision.as_str(), "confirmed" | "reassessment_required") {
        return Err(ApiError::unprocessable(
            "invalid_appeal_decision",
            "decision must be confirmed or reassessment_required",
        ));
    }
    let rationale = req.rationale.trim();
    if rationale.chars().count() < 10
        || rationale.chars().count() > 2000
        || rationale
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(ApiError::unprocessable(
            "invalid_appeal_rationale",
            "review rationale must be 10-2000 printable characters or line breaks",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let target = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT run_id, appellant_id FROM scenario_assessment_appeals WHERE id = $1 FOR UPDATE",
    )
    .bind(appeal_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("scenario_assessment_appeal_not_found"))?;
    if reviewer.user_id == target.1 {
        return Err(ApiError::forbidden(
            "appeal_appellant_cannot_review",
            "the appealing learner cannot review this appeal",
        ));
    }
    let prior_assessor = sqlx::query_scalar::<_, bool>(
		"SELECT EXISTS(SELECT 1 FROM scenario_rubric_evidence WHERE run_id = $1 AND reviewer_id = $2)",
	)
	.bind(target.0)
	.bind(reviewer.user_id)
	.fetch_one(&mut *tx)
	.await?;
    if prior_assessor {
        return Err(ApiError::forbidden(
            "appeal_assessor_cannot_review",
            "the assessed learner cannot review this appeal",
        ));
    }
    let already_reviewed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM scenario_assessment_appeal_reviews WHERE appeal_id = $1)",
    )
    .bind(appeal_id)
    .fetch_one(&mut *tx)
    .await?;
    if already_reviewed {
        return Err(ApiError::conflict(
            "appeal_already_reviewed",
            "this appeal already has a decision",
        ));
    }
    sqlx::query(
		"INSERT INTO scenario_assessment_appeal_reviews (id, appeal_id, reviewer_id, decision, rationale)
		 VALUES ($1, $2, $3, $4, $5)",
	)
	.bind(Uuid::new_v4())
	.bind(appeal_id)
	.bind(reviewer.user_id)
	.bind(&req.decision)
	.bind(rationale)
	.execute(&mut *tx)
	.await?;
    crate::routes::admin::audit(
        &mut *tx,
        reviewer.user_id,
        "scenario_assessment_appeal_reviewed",
        "scenario_assessment_appeal",
        appeal_id,
        json!({"decision": req.decision}),
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"appeal_id": appeal_id, "status":"reviewed", "decision":req.decision})),
    ))
}

#[derive(Deserialize)]
pub struct AppealReq {
    pub question_version_id: Uuid,
    pub reason: String,
}

pub async fn submit_appeal(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<AppealReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let reason = req.reason.trim();
    if reason.len() < 10 || reason.len() > 2000 {
        return Err(ApiError::unprocessable(
            "invalid_reason",
            "reason must be 10-2000 characters",
        ));
    }
    // Only appealable if the learner actually answered this version.
    let answered = sqlx::query!(
        "SELECT 1 AS one FROM attempts
         WHERE user_id = $1 AND question_version_id = $2",
        user.user_id,
        req.question_version_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ApiError::unprocessable(
            "not_answered",
            "you can only appeal a question you have answered",
        )
    })?;
    let _ = answered;
    let appeal_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO appeals (id, user_id, question_version_id, reason, status)
         VALUES ($1, $2, $3, $4, 'open')",
        appeal_id,
        user.user_id,
        req.question_version_id,
        reason
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({
        "appeal_id": appeal_id,
        "status": "open",
        "note": "A human reviewer decides appeals; the Coach cannot."
    })))
}
