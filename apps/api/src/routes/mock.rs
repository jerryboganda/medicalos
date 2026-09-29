//! EX-07: administrator-configured mock tests. Configuration is admin-token
//! gated until the editorial console (ADMIN-06) lands; learners get list +
//! start. The form is frozen at start (EX-03): questions are chosen against
//! the blueprint and snapshotted into session_items — later content edits
//! cannot alter a started mock.

use axum::extract::{Path, State};
use axum::Json;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::authz::Permission;
use crate::error::{ApiError, ApiResult};
use crate::routes::practice::PoolQuestion;
use crate::state::AppState;

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "mock/CreateMockRequest.ts",
        rename = "CreateMockRequest"
    )
)]
pub struct CreateMockReq {
    pub title: String,
    pub exam_id: Uuid,
    #[cfg_attr(
        feature = "type-export",
        ts(as = "Option<MockType>", optional = nullable)
    )]
    pub mock_type: Option<String>,
    pub blueprint: Vec<BlueprintEntry>,
    #[cfg_attr(feature = "type-export", ts(type = "number", optional = nullable))]
    pub time_limit_seconds: Option<i64>,
    #[cfg_attr(feature = "type-export", ts(type = "number", optional = nullable))]
    pub pass_mark_percent: Option<i32>,
    #[cfg_attr(feature = "type-export", ts(type = "number", optional = nullable))]
    pub attempts_allowed: Option<i32>,
    #[cfg_attr(feature = "type-export", ts(type = "number", optional = nullable))]
    pub late_sync_grace_seconds: Option<i32>,
    #[cfg_attr(
        feature = "type-export",
        ts(as = "Option<MockIntegrityPolicy>", optional = nullable)
    )]
    pub integrity_policy: Option<String>,
    #[cfg_attr(feature = "type-export", ts(type = "number", optional = nullable))]
    pub away_timeout_seconds: Option<i32>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "mock/MockBlueprintEntry.ts",
        rename = "MockBlueprintEntry"
    )
)]
pub struct BlueprintEntry {
    pub chapter_id: Uuid,
    pub count: i32,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "mock/MockType.ts", rename = "MockType")
)]
pub enum MockType {
    Full,
    Mini,
    Subject,
    System,
    Chapter,
    GrandTest,
    FinalAssessment,
}

impl MockType {
    pub(crate) fn from_wire(value: &str) -> Option<Self> {
        parse_wire_enum(value)
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "mock/MockIntegrityPolicy.ts",
        rename = "MockIntegrityPolicy"
    )
)]
pub enum MockIntegrityPolicy {
    LogOnly,
    Warn,
    AutoSubmit,
}

fn parse_wire_enum<T: DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).ok()
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "mock/CreateMockResponse.ts",
        rename = "CreateMockResponse"
    )
)]
pub struct CreateMockResponse {
    mock_id: Uuid,
    mock_type: MockType,
    late_sync_grace_seconds: i32,
    integrity_policy: MockIntegrityPolicy,
    away_timeout_seconds: Option<i32>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "mock/MockTest.ts", rename = "MockTest")
)]
pub struct MockTest {
    mock_id: Uuid,
    title: String,
    mock_type: MockType,
    pass_mark_percent: i32,
    attempts_allowed: i32,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    attempts_used: i64,
    time_limit_seconds: Option<i32>,
    late_sync_grace_seconds: i32,
    integrity_policy: MockIntegrityPolicy,
    away_timeout_seconds: Option<i32>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "mock/MockListResponse.ts",
        rename = "MockListResponse"
    )
)]
pub struct MockListResponse {
    mocks: Vec<MockTest>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "mock/StartMockResponse.ts",
        rename = "StartMockResponse"
    )
)]
pub struct StartMockResponse {
    session_id: Uuid,
    mock_type: MockType,
    #[cfg_attr(feature = "type-export", ts(type = "number"))]
    question_count: usize,
    late_sync_grace_seconds: i32,
}

pub async fn create_mock(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateMockReq>,
) -> ApiResult<Json<CreateMockResponse>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_permission(&user, provided, Permission::ExamConfigure)?;
    let title = req.title.trim();
    if title.is_empty() || title.len() > 200 {
        return Err(ApiError::unprocessable(
            "invalid_title",
            "title must be 1-200 characters",
        ));
    }
    let mock_type = req.mock_type.as_deref().unwrap_or("full");
    let mock_type_contract = MockType::from_wire(mock_type).ok_or_else(|| {
        ApiError::unprocessable(
            "invalid_mock_type",
            "mock_type must be full, mini, subject, system, chapter, grand_test, or final_assessment",
        )
    })?;

    let exam_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM exams WHERE id = $1)")
            .bind(req.exam_id)
            .fetch_one(&state.pool)
            .await?;
    if !exam_exists {
        return Err(ApiError::unprocessable(
            "invalid_exam_id",
            "exam does not exist",
        ));
    }

    if req.blueprint.is_empty() || !req.blueprint.iter().all(|e| e.count >= 1 && e.count <= 200) {
        return Err(ApiError::unprocessable(
            "invalid_blueprint",
            "blueprint needs 1-200 questions per chapter entry",
        ));
    }

    let chapter_ids: Vec<Uuid> = req.blueprint.iter().map(|e| e.chapter_id).collect();
    let unique_chapters: std::collections::HashSet<Uuid> = chapter_ids.iter().copied().collect();
    let valid_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM curriculum_nodes WHERE exam_id = $1 AND kind = 'chapter' AND id = ANY($2)"
    )
    .bind(req.exam_id)
    .bind(&chapter_ids)
    .fetch_one(&state.pool)
    .await?;

    if valid_count != unique_chapters.len() as i64 {
        return Err(ApiError::unprocessable(
            "invalid_blueprint_chapter",
            "all blueprint chapters must exist, have kind 'chapter', and belong to the chosen exam",
        ));
    }

    let pass_mark = match req.pass_mark_percent {
        Some(p) if (1..=100).contains(&p) => p,
        Some(_) => {
            return Err(ApiError::unprocessable(
                "pass_mark_out_of_range",
                "pass_mark_percent must be 1..=100",
            ));
        }
        None => 50,
    };
    let attempts = match req.attempts_allowed {
        Some(a) if (1..=10).contains(&a) => a,
        Some(_) => {
            return Err(ApiError::unprocessable(
                "attempts_out_of_range",
                "attempts_allowed must be 1..=10",
            ));
        }
        None => 1,
    };
    if let Some(limit) = req.time_limit_seconds {
        if !(60..=28_800).contains(&limit) {
            return Err(ApiError::unprocessable(
                "time_limit_out_of_range",
                "mock time_limit_seconds must be 60..=28800 (1 to 480 minutes)",
            ));
        }
    }
    let late_sync_grace_seconds = match req.late_sync_grace_seconds {
        Some(grace) if (0..=600).contains(&grace) => grace,
        Some(_) => {
            return Err(ApiError::unprocessable(
                "late_sync_grace_out_of_range",
                "late_sync_grace_seconds must be 0..=600 (0 to 10 minutes)",
            ));
        }
        None => 600,
    };
    let integrity_policy = req.integrity_policy.as_deref().unwrap_or("log_only");
    let integrity_policy_contract = parse_wire_enum(integrity_policy).ok_or_else(|| {
        ApiError::unprocessable(
            "invalid_integrity_policy",
            "integrity_policy must be log_only, warn, or auto_submit",
        )
    })?;
    let away_timeout_seconds = match (integrity_policy_contract, req.away_timeout_seconds) {
        (MockIntegrityPolicy::LogOnly, None) => None,
        (MockIntegrityPolicy::LogOnly, Some(_)) => {
            return Err(ApiError::unprocessable(
                "unexpected_away_timeout",
                "log_only policy does not accept away_timeout_seconds",
            ));
        }
        (_, Some(seconds)) if (15..=3600).contains(&seconds) => Some(seconds),
        _ => {
            return Err(ApiError::unprocessable(
                "away_timeout_out_of_range",
                "warn and auto_submit policies require away_timeout_seconds from 15 through 3600",
            ));
        }
    };
    let blueprint = serde_json::to_value(&req.blueprint).map_err(|_| ApiError::internal())?;
    let mock_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO mocks
           (id, title, exam_id, blueprint, time_limit_seconds, pass_mark_percent,
            attempts_allowed, created_by, late_sync_grace_seconds, integrity_policy,
            away_timeout_seconds, mock_type)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        mock_id,
        title,
        req.exam_id,
        blueprint,
        // INT column binds as i32; the request field is i64 for validation.
        req.time_limit_seconds.map(|l| l as i32),
        pass_mark,
        attempts,
        user.user_id,
        late_sync_grace_seconds,
        integrity_policy,
        away_timeout_seconds,
        mock_type
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(CreateMockResponse {
        mock_id,
        mock_type: mock_type_contract,
        late_sync_grace_seconds,
        integrity_policy: integrity_policy_contract,
        away_timeout_seconds,
    }))
}

pub async fn list_mocks(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<MockListResponse>> {
    let rows = sqlx::query!(
        r#"SELECT m.id, m.title, m.pass_mark_percent, m.attempts_allowed,
                  m.time_limit_seconds, m.late_sync_grace_seconds,
                  m.integrity_policy, m.away_timeout_seconds, m.mock_type,
                  (SELECT COUNT(*) FROM mock_attempts ma
                   WHERE ma.mock_id = m.id AND ma.user_id = $1) AS "used!"
           FROM mocks m ORDER BY m.created_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let mocks: Vec<MockTest> = rows
        .into_iter()
        .map(|m| {
            Ok(MockTest {
                mock_id: m.id,
                title: m.title,
                mock_type: MockType::from_wire(&m.mock_type).ok_or_else(ApiError::internal)?,
                pass_mark_percent: m.pass_mark_percent,
                attempts_allowed: m.attempts_allowed,
                attempts_used: m.used,
                time_limit_seconds: m.time_limit_seconds,
                late_sync_grace_seconds: m.late_sync_grace_seconds,
                integrity_policy: parse_wire_enum(&m.integrity_policy)
                    .ok_or_else(ApiError::internal)?,
                away_timeout_seconds: m.away_timeout_seconds,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(MockListResponse { mocks }))
}

/// Freeze the form (EX-03): pick against the blueprint once, snapshot the
/// items — then the standard answer/submit pipeline takes over with feedback
/// deferred (§11.2 exam-style) and the §11.3 server-issued timer.
pub async fn start_mock(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(mid): Path<Uuid>,
) -> ApiResult<Json<StartMockResponse>> {
    let mock = sqlx::query!(
        "SELECT id, blueprint, time_limit_seconds, late_sync_grace_seconds,
                integrity_policy, away_timeout_seconds, mock_type FROM mocks WHERE id = $1",
        mid
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("mock_not_found"))?;
    let mock_type = MockType::from_wire(&mock.mock_type).ok_or_else(ApiError::internal)?;
    let blueprint_json = mock.blueprint;
    let time_limit_seconds = mock.time_limit_seconds;
    let late_sync_grace_seconds = mock.late_sync_grace_seconds;
    let integrity_policy = mock.integrity_policy;
    let away_timeout_seconds = mock.away_timeout_seconds;

    let used = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM mock_attempts
           WHERE mock_id = $1 AND user_id = $2"#,
        mid,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    // COM-01: the full-mock upgrade trigger originates from the entitlement
    // check — free-tier accounts get free_mock_attempts full mocks total.
    let total = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM practice_sessions
           WHERE user_id = $1 AND mock_id IS NOT NULL"#,
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    if total >= state.free_mock_attempts {
        return Err(ApiError::forbidden_with_details(
            "upgrade_required",
            "full mocks beyond the free allowance need an upgrade",
            serde_json::json!({
                "trigger": "full_mock",
                "entitlement": {
                    "free_mock_attempts": state.free_mock_attempts,
                    "used": total
                }
            }),
        ));
    }
    let allowed = sqlx::query!("SELECT attempts_allowed FROM mocks WHERE id = $1", mid)
        .fetch_one(&state.pool)
        .await?
        .attempts_allowed;
    if used >= allowed as i64 {
        return Err(ApiError::conflict(
            "attempts_exhausted",
            format!("all {} attempts for this mock are used", allowed),
        ));
    }

    let blueprint: Vec<BlueprintEntry> =
        serde_json::from_value(blueprint_json).map_err(|_| ApiError::internal())?;
    let mut pool_questions: Vec<PoolQuestion> = Vec::new();
    for entry in &blueprint {
        let count = entry.count.clamp(0, 200) as i64;
        let rows = sqlx::query!(
            r#"SELECT id, vignette, lead_in, difficulty, options
               FROM question_versions
               WHERE status = 'published' AND chapter_id = $1
               ORDER BY random() LIMIT $2"#,
            entry.chapter_id,
            count
        )
        .fetch_all(&state.pool)
        .await?;
        if rows.len() < count as usize {
            return Err(ApiError::unprocessable(
                "insufficient_questions",
                format!(
                    "blueprint needs {} questions in one chapter, only {} are available",
                    count,
                    rows.len()
                ),
            ));
        }
        for r in rows {
            pool_questions.push(PoolQuestion {
                id: r.id,
                vignette: r.vignette,
                lead_in: r.lead_in,
                difficulty: r.difficulty,
                options: r.options,
            });
        }
    }

    let deadline = time_limit_seconds
        .map(|limit| chrono::Utc::now() + chrono::Duration::seconds(limit as i64));
    let sid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO practice_sessions
           (id, user_id, preset, mock_id, time_limit_seconds, deadline,
            late_sync_grace_seconds, integrity_policy, away_timeout_seconds)
         VALUES ($1, $2, 'mock', $3, $4, $5, $6, $7, $8)",
        sid,
        user.user_id,
        mid,
        time_limit_seconds,
        deadline,
        late_sync_grace_seconds,
        integrity_policy,
        away_timeout_seconds
    )
    .execute(&state.pool)
    .await?;
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
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(StartMockResponse {
        session_id: sid,
        mock_type,
        question_count: pool_questions.len(),
        late_sync_grace_seconds,
    }))
}
