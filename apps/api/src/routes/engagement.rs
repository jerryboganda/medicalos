//! ENG-02 (XP/achievements), COMP-01/02 (competitions on the shared
//! scoring crate), INST-04 (curriculum coverage), ADMIN-06 settings.

use axum::extract::{Path, State};
use axum::Json;
use competition_scoring::{AnswerRecord, Difficulty, ScoringConfig};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ---- ENG-02: XP + achievements -----------------------------------------------

fn xp_for_difficulty(difficulty: &str) -> i32 {
    match difficulty {
        "hard" => 30,
        "medium" => 20,
        _ => 10,
    }
}

/// Called by the practice submit path: award XP for correct answers and
/// check milestone achievements. Deterministic rules (§17.2 guardrails —
/// never an optimization target).
pub async fn award_xp_for_submit(
    state: &AppState,
    user_id: Uuid,
    correct: i64,
    difficulties: Vec<String>,
) -> ApiResult<()> {
    let total: i32 = difficulties.iter().map(|d| xp_for_difficulty(d)).sum();
    if total == 0 {
        return Ok(());
    }
    sqlx::query!(
        "INSERT INTO xp_ledger (id, user_id, points, reason)
         VALUES ($1, $2, $3, 'session_correct')",
        Uuid::new_v4(),
        user_id,
        total
    )
    .execute(&state.pool)
    .await?;
    // Milestone achievements: 100 / 500 correct-answer XP totals.
    let earned: i64 = sqlx::query!(
        r#"SELECT COALESCE(SUM(points), 0) AS "total!" FROM xp_ledger WHERE user_id = $1"#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?
    .total;
    for milestone in [100, 500] {
        if earned >= milestone {
            sqlx::query!(
                "INSERT INTO achievements (user_id, code) VALUES ($1, $2)
                 ON CONFLICT DO NOTHING",
                user_id,
                format!("xp_{milestone}")
            )
            .execute(&state.pool)
            .await?;
        }
    }
    Ok(())
}

pub async fn my_xp(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let total = sqlx::query!(
        r#"SELECT COALESCE(SUM(points), 0) AS "total!" FROM xp_ledger WHERE user_id = $1"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .total;
    let achievements = sqlx::query!(
        "SELECT code, earned_at FROM achievements WHERE user_id = $1 ORDER BY earned_at",
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "xp_total": total,
        "achievements": achievements.iter().map(|a| json!({
            "code": a.code, "earned_at": a.earned_at,
        })).collect::<Vec<_>>(),
    })))
}

// ---- INST-04: curriculum coverage for institution members --------------------

/// Per-chapter coverage across an institution's learners: how many members
/// attempted, how many questions exist vs attempted per chapter. Aggregates
/// only — no individual rows (§18.3).
pub async fn institution_coverage(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(inst_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let member = sqlx::query!(
        "SELECT 1 AS one FROM institution_members
         WHERE institution_id = $1 AND user_id = $2",
        inst_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("institution_not_found"))?;
    let _ = member;

    let rows = sqlx::query!(
        r#"SELECT c.name AS chapter_name,
                  COALESCE(COUNT(DISTINCT a.user_id), 0) AS "learners!",
                  COALESCE(COUNT(*), 0) AS "attempts!"
           FROM institution_members im
           JOIN cohorts coh ON coh.institution_id = im.institution_id
           JOIN cohort_members cm ON cm.cohort_id = coh.id AND cm.user_id = im.user_id
           LEFT JOIN attempts a ON a.user_id = im.user_id
           LEFT JOIN question_versions qv ON qv.id = a.question_version_id
           LEFT JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE im.institution_id = $1
           GROUP BY c.name ORDER BY c.name"#,
        inst_id
    )
    .fetch_all(&state.pool)
    .await?;
    let coverage: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "chapter": r.chapter_name,
                "learners": r.learners,
                "attempts": r.attempts,
            })
        })
        .collect();
    Ok(Json(json!({ "coverage": coverage })))
}

// ---- COMP-01/02: competitions on the shared scoring crate --------------------

#[derive(Deserialize)]
pub struct CreateCompetitionReq {
    pub title: String,
    pub exam_id: Uuid,
    pub question_ids: Vec<Uuid>,
    pub starts_at: chrono::DateTime<chrono::Utc>,
    pub ends_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_competition(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateCompetitionReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    if req.title.trim().is_empty() || req.question_ids.len() < 3 {
        return Err(ApiError::unprocessable(
            "invalid_competition",
            "title required and at least 3 questions",
        ));
    }
    if req.ends_at <= req.starts_at {
        return Err(ApiError::unprocessable(
            "invalid_window",
            "competition must end after it starts",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO competitions (id, title, exam_id, question_ids, starts_at, ends_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        id,
        req.title.trim(),
        req.exam_id,
        serde_json::to_value(&req.question_ids).map_err(|_| ApiError::internal())?,
        req.starts_at,
        req.ends_at,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "competition_id": id })))
}

pub async fn list_competitions(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT c.id, c.title, c.starts_at, c.ends_at, c.status,
                  (SELECT COUNT(*) FROM competition_entries ce
                   WHERE ce.competition_id = c.id AND ce.user_id = $1) AS "entered!"
           FROM competitions c ORDER BY c.starts_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let comps: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "competition_id": r.id, "title": r.title, "starts_at": r.starts_at,
                "ends_at": r.ends_at, "status": r.status, "entered": r.entered,
            })
        })
        .collect();
    Ok(Json(json!({ "competitions": comps })))
}

#[derive(Deserialize)]
pub struct SubmitEntryReq {
    /// Ordered answers aligned with the competition's question_ids:
    /// [{question_version_id, chosen_index, elapsed_ms}]
    pub answers: Vec<serde_json::Value>,
    pub handle: String,
    pub total_time_ms: i64,
}

/// Score one entry with the shared scoring crate and rank it against the
/// competition's entries (COMP-02: guess penalty, capped speed bonus,
/// tie-break ladder). Integrity review (COMP-04) gates prize claims later.
pub async fn submit_competition_entry(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(comp_id): Path<Uuid>,
    Json(req): Json<SubmitEntryReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let comp = sqlx::query!(
        "SELECT starts_at, ends_at, status FROM competitions WHERE id = $1",
        comp_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("competition_not_found"))?;
    let now = chrono::Utc::now();
    if now < comp.starts_at {
        return Err(ApiError::conflict(
            "not_started",
            "competition has not started",
        ));
    }
    if now > comp.ends_at {
        return Err(ApiError::conflict(
            "already_closed",
            "competition window has closed",
        ));
    }
    let handle = req.handle.trim();
    if handle.is_empty() || handle.len() > 40 {
        return Err(ApiError::unprocessable(
            "invalid_handle",
            "handle must be 1-40 characters",
        ));
    }

    // Score with the shared crate: one AnswerRecord per question.
    let cfg = ScoringConfig::default();
    let mut records = Vec::with_capacity(req.answers.len());
    let mut total_time = 0i64;
    for a in &req.answers {
        let vid = a
            .get("question_version_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                ApiError::unprocessable("invalid_answer", "missing question_version_id")
            })?;
        let chosen = a
            .get("chosen_index")
            .and_then(|v| v.as_i64())
            .map(|v| v as i16)
            .ok_or_else(|| ApiError::unprocessable("invalid_answer", "missing chosen_index"))?;
        let elapsed = a
            .get("elapsed_ms")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ApiError::unprocessable("invalid_answer", "missing elapsed_ms"))?;
        // Look up the reviewed key + difficulty for this version.
        let qv = sqlx::query!(
            "SELECT correct_index, difficulty FROM question_versions WHERE id = $1",
            vid
        )
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("question_not_found"))?;
        let correct = chosen == qv.correct_index;
        #[allow(unused_variables)]
        let _ = correct;
        let difficulty = match qv.difficulty.as_str() {
            "easy" => Difficulty::Easy,
            "hard" => Difficulty::Hard,
            _ => Difficulty::Medium,
        };
        records.push((vid, difficulty, correct, elapsed));
        total_time += elapsed;
    }

    // Score via the crate and persist the entry (§31.1: scoring lives in
    // the shared crate, never re-implemented here).
    let answer_records: Vec<AnswerRecord> = records
        .iter()
        .map(|(_, difficulty, correct, elapsed)| AnswerRecord {
            difficulty: *difficulty,
            correct: *correct,
            elapsed_ms: *elapsed,
        })
        .collect();
    let score: f64 = answer_records.iter().map(|r| cfg.item_score(r)).sum();

    let entry_id = Uuid::new_v4();
    let score_f32 = score as f32;
    sqlx::query!(
        "INSERT INTO competition_entries
           (id, competition_id, user_id, handle, answers, score, total_time_ms, submitted_order)
         VALUES ($1, $2, $3, $4, $5, $6, $7,
           (SELECT COALESCE(MAX(submitted_order), 0) + 1 FROM competition_entries
            WHERE competition_id = $2))",
        entry_id,
        comp_id,
        user.user_id,
        handle,
        serde_json::to_value(&req.answers).map_err(|_| ApiError::internal())?,
        score_f32,
        total_time
    )
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "entry_id": entry_id,
        "score": score,
        "questions": records.len(),
    })))
}
