//! LIB-05: source passage dependencies, editorial change cases, and audited
//! review/correction actions. Source text is never copied into this graph.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{PgConnection, Row};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

fn require_admin(state: &AppState, headers: &HeaderMap) -> ApiResult<()> {
    state.require_admin(headers.get("x-admin-token").and_then(|v| v.to_str().ok()))
}

fn clean_text(
    value: &str,
    max: usize,
    code: &'static str,
    message: &'static str,
) -> ApiResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max {
        return Err(ApiError::unprocessable(code, message));
    }
    Ok(value.to_owned())
}

#[derive(Deserialize)]
pub struct RegisterPassageReq {
    pub source_ref: String,
    pub locator: String,
}

pub async fn register_passage(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Json(req): Json<RegisterPassageReq>,
) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let source_ref = clean_text(
        &req.source_ref,
        500,
        "invalid_source_ref",
        "source reference must be 1-500 characters",
    )?;
    let locator = clean_text(
        &req.locator,
        500,
        "invalid_source_locator",
        "source locator must be 1-500 characters",
    )?;
    let mut tx = state.pool.begin().await?;
    let new_id = Uuid::new_v4();
    let inserted = sqlx::query(
        r#"INSERT INTO source_passages (id, source_ref, locator, created_by)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (source_ref, locator) DO NOTHING
           RETURNING id"#,
    )
    .bind(new_id)
    .bind(&source_ref)
    .bind(&locator)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (passage_id, created) = match inserted {
        Some(row) => (row.try_get::<Uuid, _>("id")?, true),
        None => (
            sqlx::query_scalar(
                "SELECT id FROM source_passages WHERE source_ref = $1 AND locator = $2",
            )
            .bind(&source_ref)
            .bind(&locator)
            .fetch_one(&mut *tx)
            .await?,
            false,
        ),
    };
    if created {
        crate::routes::admin::audit(
            &mut *tx,
            user.user_id,
            "source_passage_registered",
            "source_passage",
            passage_id,
            json!({"source_ref":source_ref,"locator":locator}),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({
        "source_passage_id": passage_id,
        "created": created,
        "source_ref": source_ref,
        "locator": locator,
    })))
}

#[derive(Deserialize)]
pub struct LinkDependencyReq {
    pub resource_kind: String,
    pub resource_version_id: Uuid,
}

async fn resource_status_on(
    conn: &mut PgConnection,
    kind: &str,
    version_id: Uuid,
) -> ApiResult<Option<String>> {
    let status = match kind {
        "question" => {
            sqlx::query_scalar("SELECT status FROM question_versions WHERE id = $1 FOR SHARE")
                .bind(version_id)
                .fetch_optional(&mut *conn)
                .await?
        }
        "article" => {
            sqlx::query_scalar("SELECT status FROM article_versions WHERE id = $1 FOR SHARE")
                .bind(version_id)
                .fetch_optional(&mut *conn)
                .await?
        }
        "scenario" => {
            sqlx::query_scalar("SELECT status FROM scenario_versions WHERE id = $1 FOR SHARE")
                .bind(version_id)
                .fetch_optional(&mut *conn)
                .await?
        }
        _ => {
            return Err(ApiError::unprocessable(
                "invalid_resource_kind",
                "resource kind must be question, article, or scenario",
            ))
        }
    };
    Ok(status)
}

pub async fn link_dependency(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(passage_id): Path<Uuid>,
    Json(req): Json<LinkDependencyReq>,
) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let mut tx = state.pool.begin().await?;
    let passage = sqlx::query("SELECT id FROM source_passages WHERE id = $1 FOR SHARE")
        .bind(passage_id)
        .fetch_optional(&mut *tx)
        .await?;
    if passage.is_none() {
        return Err(ApiError::not_found("source_passage_not_found"));
    }
    let status = resource_status_on(&mut tx, &req.resource_kind, req.resource_version_id)
        .await?
        .ok_or_else(|| ApiError::not_found("resource_version_not_found"))?;
    if status != "published" {
        return Err(ApiError::conflict(
            "resource_version_unpublished",
            "source dependencies must target a published resource version",
        ));
    }
    let dependency_id = Uuid::new_v4();
    let inserted = sqlx::query(
        r#"INSERT INTO source_dependencies
             (id, source_passage_id, resource_kind, question_version_id,
              article_version_id, scenario_version_id, created_by)
           VALUES ($1, $2, $3,
                   CASE WHEN $3 = 'question' THEN $4 END,
                   CASE WHEN $3 = 'article' THEN $4 END,
                   CASE WHEN $3 = 'scenario' THEN $4 END, $5)
           ON CONFLICT DO NOTHING
           RETURNING id"#,
    )
    .bind(dependency_id)
    .bind(passage_id)
    .bind(&req.resource_kind)
    .bind(req.resource_version_id)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (dependency_id, created) = match inserted {
        Some(row) => (row.try_get::<Uuid, _>("id")?, true),
        None => {
            let column = match req.resource_kind.as_str() {
                "question" => "question_version_id",
                "article" => "article_version_id",
                "scenario" => "scenario_version_id",
                _ => unreachable!("kind checked by resource_status_on"),
            };
            let sql = format!(
                "SELECT id FROM source_dependencies WHERE source_passage_id = $1 AND {column} = $2"
            );
            (
                sqlx::query_scalar(&sql)
                    .bind(passage_id)
                    .bind(req.resource_version_id)
                    .fetch_one(&mut *tx)
                    .await?,
                false,
            )
        }
    };
    if created {
        crate::routes::admin::audit(
            &mut *tx,
            user.user_id,
            "source_dependency_linked",
            "source_dependency",
            dependency_id,
            json!({
                "source_passage_id": passage_id,
                "resource_kind": req.resource_kind,
                "resource_version_id": req.resource_version_id,
            }),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(
        json!({"dependency_id":dependency_id,"created":created}),
    ))
}

#[derive(Deserialize)]
pub struct RecordChangeReq {
    pub source_revision: String,
    pub classification: String,
    pub note: String,
}

fn valid_classification(classification: &str) -> bool {
    matches!(
        classification,
        "minor_typo" | "material_change" | "invalid_answer_key" | "source_withdrawn"
    )
}

async fn impacted_users_on(
    conn: &mut PgConnection,
    kind: &str,
    question_version_id: Option<Uuid>,
    article_version_id: Option<Uuid>,
    scenario_version_id: Option<Uuid>,
) -> ApiResult<Vec<Uuid>> {
    let rows =
        match kind {
            "question" => {
                sqlx::query(
                    r#"SELECT user_id FROM attempts WHERE question_version_id = $1
               UNION SELECT user_id FROM cards WHERE source_question_version_id = $1
               ORDER BY user_id"#,
                )
                .bind(question_version_id)
                .fetch_all(&mut *conn)
                .await?
            }
            "article" => sqlx::query(
                "SELECT user_id FROM article_reads WHERE article_version_id = $1 ORDER BY user_id",
            )
            .bind(article_version_id)
            .fetch_all(&mut *conn)
            .await?,
            "scenario" => sqlx::query(
                "SELECT user_id FROM scenario_runs WHERE scenario_version_id = $1 ORDER BY user_id",
            )
            .bind(scenario_version_id)
            .fetch_all(&mut *conn)
            .await?,
            _ => return Err(ApiError::internal()),
        };
    rows.into_iter()
        .map(|row| row.try_get("user_id").map_err(ApiError::from))
        .collect()
}

async fn quarantine_resource_on(
    conn: &mut PgConnection,
    kind: &str,
    question_version_id: Option<Uuid>,
    article_version_id: Option<Uuid>,
    scenario_version_id: Option<Uuid>,
) -> ApiResult<()> {
    match kind {
        "question" => {
            if let Some(version_id) = question_version_id {
                sqlx::query(
                    "UPDATE question_versions SET status = 'quarantined'
                     WHERE id = $1 AND status = 'published'",
                )
                .bind(version_id)
                .execute(&mut *conn)
                .await?;
                sqlx::query(
                    "UPDATE cards SET suspended = true WHERE source_question_version_id = $1",
                )
                .bind(version_id)
                .execute(&mut *conn)
                .await?;
                sqlx::query("DELETE FROM pregen_tutoring WHERE question_version_id = $1")
                    .bind(version_id)
                    .execute(&mut *conn)
                    .await?;
            }
        }
        "article" => {
            if let Some(version_id) = article_version_id {
                sqlx::query(
                    "UPDATE article_versions SET status = 'quarantined'
                     WHERE id = $1 AND status = 'published'",
                )
                .bind(version_id)
                .execute(&mut *conn)
                .await?;
            }
        }
        "scenario" => {
            if let Some(version_id) = scenario_version_id {
                sqlx::query(
                    "UPDATE scenario_versions SET status = 'quarantined'
                     WHERE id = $1 AND status = 'published'",
                )
                .bind(version_id)
                .execute(&mut *conn)
                .await?;
            }
        }
        _ => return Err(ApiError::internal()),
    }
    Ok(())
}

async fn case_json_on(conn: &mut PgConnection, event_id: Uuid) -> ApiResult<Value> {
    let event = sqlx::query(
        r#"SELECT e.id, e.source_passage_id, e.source_revision, e.classification,
                  e.note, e.actor_id, e.created_at, p.source_ref, p.locator
           FROM source_change_events e
           JOIN source_passages p ON p.id = e.source_passage_id
           WHERE e.id = $1"#,
    )
    .bind(event_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::not_found("source_change_not_found"))?;
    let task_rows = sqlx::query(
        r#"SELECT t.id, t.resource_kind, t.question_version_id, t.article_version_id,
                  t.scenario_version_id, t.status, t.resolution, t.resolution_note,
                  t.corrected_question_version_id, t.resolved_by, t.resolved_at,
                  COUNT(l.user_id)::BIGINT AS affected_learners,
                  COALESCE(array_agg(l.user_id ORDER BY l.user_id)
                      FILTER (WHERE l.user_id IS NOT NULL), ARRAY[]::UUID[]) AS affected_user_ids,
                  (SELECT COUNT(*) FROM cards c
                   WHERE c.source_question_version_id = t.question_version_id)::BIGINT AS affected_cards
           FROM source_change_tasks t
           LEFT JOIN source_change_task_learners l ON l.task_id = t.id
           WHERE t.event_id = $1
           GROUP BY t.id ORDER BY t.created_at, t.id"#,
    )
    .bind(event_id)
    .fetch_all(&mut *conn)
    .await?;
    let tasks = task_rows
        .into_iter()
        .map(|row| {
            Ok(json!({
                "task_id": row.try_get::<Uuid, _>("id")?,
                "resource_kind": row.try_get::<String, _>("resource_kind")?,
                "question_version_id": row.try_get::<Option<Uuid>, _>("question_version_id")?,
                "article_version_id": row.try_get::<Option<Uuid>, _>("article_version_id")?,
                "scenario_version_id": row.try_get::<Option<Uuid>, _>("scenario_version_id")?,
                "status": row.try_get::<String, _>("status")?,
                "resolution": row.try_get::<Option<String>, _>("resolution")?,
                "resolution_note": row.try_get::<String, _>("resolution_note")?,
                "corrected_question_version_id": row.try_get::<Option<Uuid>, _>("corrected_question_version_id")?,
                "resolved_by": row.try_get::<Option<Uuid>, _>("resolved_by")?,
                "resolved_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("resolved_at")?,
                "affected_learners": row.try_get::<i64, _>("affected_learners")?,
                "affected_user_ids": row.try_get::<Vec<Uuid>, _>("affected_user_ids")?,
                "affected_cards": row.try_get::<i64, _>("affected_cards")?,
            }))
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    let affected_learners: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(DISTINCT l.user_id)
           FROM source_change_tasks t
           JOIN source_change_task_learners l ON l.task_id = t.id
           WHERE t.event_id = $1"#,
    )
    .bind(event_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(json!({
        "source_change_id": event.try_get::<Uuid, _>("id")?,
        "source_passage_id": event.try_get::<Uuid, _>("source_passage_id")?,
        "source_ref": event.try_get::<String, _>("source_ref")?,
        "locator": event.try_get::<String, _>("locator")?,
        "source_revision": event.try_get::<String, _>("source_revision")?,
        "classification": event.try_get::<String, _>("classification")?,
        "note": event.try_get::<String, _>("note")?,
        "actor_id": event.try_get::<Uuid, _>("actor_id")?,
        "created_at": event.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")?,
        "affected_learners": affected_learners,
        "tasks": tasks,
    }))
}

pub async fn record_change(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(passage_id): Path<Uuid>,
    Json(req): Json<RecordChangeReq>,
) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let revision = clean_text(
        &req.source_revision,
        160,
        "invalid_source_revision",
        "source revision must be 1-160 characters",
    )?;
    let note = clean_text(
        &req.note,
        2000,
        "invalid_source_change_note",
        "source change note must be 1-2000 characters",
    )?;
    if !valid_classification(&req.classification) {
        return Err(ApiError::unprocessable(
            "invalid_source_change_classification",
            "classification must be minor_typo, material_change, invalid_answer_key, or source_withdrawn",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let passage = sqlx::query("SELECT id FROM source_passages WHERE id = $1 FOR UPDATE")
        .bind(passage_id)
        .fetch_optional(&mut *tx)
        .await?;
    if passage.is_none() {
        return Err(ApiError::not_found("source_passage_not_found"));
    }
    let event_id = Uuid::new_v4();
    let inserted = sqlx::query(
        r#"INSERT INTO source_change_events
             (id, source_passage_id, source_revision, classification, note, actor_id)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT (source_passage_id, source_revision) DO NOTHING
           RETURNING id"#,
    )
    .bind(event_id)
    .bind(passage_id)
    .bind(&revision)
    .bind(&req.classification)
    .bind(&note)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(inserted) = inserted else {
        let previous = sqlx::query(
            "SELECT id, classification, note FROM source_change_events
             WHERE source_passage_id = $1 AND source_revision = $2",
        )
        .bind(passage_id)
        .bind(&revision)
        .fetch_one(&mut *tx)
        .await?;
        let existing_id: Uuid = previous.try_get("id")?;
        if previous.try_get::<String, _>("classification")? != req.classification
            || previous.try_get::<String, _>("note")? != note
        {
            return Err(ApiError::conflict(
                "source_revision_already_recorded",
                "this source revision already has a different change record",
            ));
        }
        let body = case_json_on(&mut tx, existing_id).await?;
        tx.commit().await?;
        return Ok(Json(body));
    };
    let event_id: Uuid = inserted.try_get("id")?;
    let dependencies = sqlx::query(
        r#"SELECT d.id, d.resource_kind, d.question_version_id,
                  d.article_version_id, d.scenario_version_id
           FROM source_dependencies d
           LEFT JOIN question_versions qv ON qv.id = d.question_version_id
           LEFT JOIN article_versions av ON av.id = d.article_version_id
           LEFT JOIN scenario_versions sv ON sv.id = d.scenario_version_id
           WHERE d.source_passage_id = $1
             AND ((d.resource_kind = 'question' AND qv.status = 'published')
               OR (d.resource_kind = 'article' AND av.status = 'published')
               OR (d.resource_kind = 'scenario' AND sv.status = 'published'))
           ORDER BY d.created_at, d.id"#,
    )
    .bind(passage_id)
    .fetch_all(&mut *tx)
    .await?;
    let unsafe_change = matches!(
        req.classification.as_str(),
        "invalid_answer_key" | "source_withdrawn"
    );
    for dependency in dependencies {
        let dependency_id: Uuid = dependency.try_get("id")?;
        let kind: String = dependency.try_get("resource_kind")?;
        let question_version_id: Option<Uuid> = dependency.try_get("question_version_id")?;
        let article_version_id: Option<Uuid> = dependency.try_get("article_version_id")?;
        let scenario_version_id: Option<Uuid> = dependency.try_get("scenario_version_id")?;
        let task_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO source_change_tasks
                 (id, event_id, dependency_id, resource_kind, question_version_id,
                  article_version_id, scenario_version_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(task_id)
        .bind(event_id)
        .bind(dependency_id)
        .bind(&kind)
        .bind(question_version_id)
        .bind(article_version_id)
        .bind(scenario_version_id)
        .execute(&mut *tx)
        .await?;
        let learners = impacted_users_on(
            &mut tx,
            &kind,
            question_version_id,
            article_version_id,
            scenario_version_id,
        )
        .await?;
        for learner_id in learners {
            sqlx::query(
                "INSERT INTO source_change_task_learners (task_id, user_id)
                 VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(task_id)
            .bind(learner_id)
            .execute(&mut *tx)
            .await?;
        }
        if unsafe_change {
            quarantine_resource_on(
                &mut tx,
                &kind,
                question_version_id,
                article_version_id,
                scenario_version_id,
            )
            .await?;
        }
    }
    let task_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM source_change_tasks WHERE event_id = $1")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "source_change_recorded",
        "source_change_event",
        event_id,
        json!({
            "source_passage_id": passage_id,
            "source_revision": revision,
            "classification": req.classification,
            "task_count": task_count,
        }),
    )
    .await?;
    if req.classification == "invalid_answer_key" {
        let body = "A source change may affect a question answer key. Review your saved result and the corrected content notice when it arrives.";
        sqlx::query(
            r#"INSERT INTO notifications (id, user_id, category, title, body, deep_link)
               SELECT gen_random_uuid(), affected.user_id, 'content_update',
                      'Question source under review', $2, '/notifications'
               FROM (
                   SELECT DISTINCT l.user_id
                   FROM source_change_tasks t
                   JOIN source_change_task_learners l ON l.task_id = t.id
                   WHERE t.event_id = $1 AND t.resource_kind = 'question'
               ) affected
               LEFT JOIN notification_preferences p ON p.user_id = affected.user_id
               WHERE COALESCE(p.content_updates, TRUE)"#,
        )
        .bind(event_id)
        .bind(body)
        .execute(&mut *tx)
        .await?;
    }
    let body = case_json_on(&mut tx, event_id).await?;
    tx.commit().await?;
    Ok(Json(body))
}

pub async fn get_change(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let mut conn = state.pool.acquire().await?;
    Ok(Json(case_json_on(&mut conn, event_id).await?))
}

#[derive(Deserialize)]
pub struct ResolveTaskReq {
    pub resolution: String,
    pub corrected_version_id: Option<Uuid>,
    pub resolution_note: Option<String>,
}

fn same_option_texts(old: &Value, new: &Value) -> bool {
    let (Some(old), Some(new)) = (old.as_array(), new.as_array()) else {
        return false;
    };
    old.len() == new.len()
        && old.iter().zip(new).all(|(old, new)| {
            old.get("text").and_then(Value::as_str) == new.get("text").and_then(Value::as_str)
                && old.get("text").and_then(Value::as_str).is_some()
        })
}

async fn refresh_session_receipt_on(
    conn: &mut PgConnection,
    session_id: Uuid,
) -> ApiResult<Option<Uuid>> {
    let session = sqlx::query(
        r#"SELECT mock_id, result_payload FROM practice_sessions
           WHERE id = $1 AND status = 'submitted' FOR UPDATE"#,
    )
    .bind(session_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(ApiError::internal)?;
    let receipt = session.try_get::<Option<Value>, _>("result_payload")?;
    let mock_id: Option<Uuid> = session.try_get("mock_id")?;
    if receipt.is_none() && mock_id.is_none() {
        return Ok(None);
    }
    let totals = sqlx::query(
        r#"SELECT COUNT(si.id)::BIGINT AS total,
                  COUNT(*) FILTER (WHERE a.correct IS TRUE)::BIGINT AS correct,
                  COUNT(*) FILTER (WHERE a.chosen_index IS NOT NULL AND a.correct IS FALSE)::BIGINT AS incorrect,
                  COUNT(*) FILTER (WHERE a.id IS NULL OR a.chosen_index IS NULL)::BIGINT AS skipped
           FROM session_items si
           LEFT JOIN attempts a ON a.session_id = si.session_id AND a.item_index = si.item_index
           WHERE si.session_id = $1"#,
    )
    .bind(session_id)
    .fetch_one(&mut *conn)
    .await?;
    let total: i64 = totals.try_get("total")?;
    let correct: i64 = totals.try_get("correct")?;
    let incorrect: i64 = totals.try_get("incorrect")?;
    let skipped: i64 = totals.try_get("skipped")?;
    let score = if total == 0 { 0 } else { correct * 100 / total };
    let pass_mark: Option<i32> = if let Some(mock_id) = mock_id {
        Some(
            sqlx::query_scalar::<_, i32>("SELECT pass_mark_percent FROM mocks WHERE id = $1")
                .bind(mock_id)
                .fetch_one(&mut *conn)
                .await?,
        )
    } else {
        None
    };
    if let Some(mut receipt) = receipt {
        receipt["total"] = json!(total);
        receipt["correct"] = json!(correct);
        receipt["incorrect"] = json!(incorrect);
        receipt["skipped"] = json!(skipped);
        receipt["score"] = json!(score);
        receipt["expected_score"] = Value::Null;
        receipt["source_correction_adjusted"] = json!(true);
        if let (Some(_mock_id), Some(pass_mark)) = (mock_id, pass_mark) {
            let passed = score >= i64::from(pass_mark);
            let chapters = sqlx::query(
                r#"SELECT c.name AS chapter,
                          COUNT(*)::BIGINT AS total,
                          COUNT(*) FILTER (WHERE a.correct IS TRUE)::BIGINT AS correct
                   FROM session_items si
                   JOIN question_versions qv ON qv.id = si.question_version_id
                   JOIN curriculum_nodes c ON c.id = qv.chapter_id
                   LEFT JOIN attempts a ON a.session_id = si.session_id AND a.item_index = si.item_index
                   WHERE si.session_id = $1 GROUP BY c.name ORDER BY c.name"#,
            )
            .bind(session_id)
            .fetch_all(&mut *conn)
            .await?;
            let breakdown = chapters
                .into_iter()
                .map(|row| {
                    Ok(json!({
                        "chapter": row.try_get::<String, _>("chapter")?,
                        "total": row.try_get::<i64, _>("total")?,
                        "correct": row.try_get::<i64, _>("correct")?,
                    }))
                })
                .collect::<Result<Vec<_>, sqlx::Error>>()?;
            receipt["mock"] = json!({
                "score_percent": score,
                "passed": passed,
                "pass_mark_percent": pass_mark,
                "percentile": null,
                "takers": null,
                "breakdown": breakdown,
            });
        }
        sqlx::query("UPDATE practice_sessions SET result_payload = $2 WHERE id = $1")
            .bind(session_id)
            .bind(receipt)
            .execute(&mut *conn)
            .await?;
    }
    if let (Some(_mock_id), Some(pass_mark)) = (mock_id, pass_mark) {
        let passed = score >= i64::from(pass_mark);
        sqlx::query(
            "UPDATE mock_attempts SET score_percent = $2, passed = $3 WHERE session_id = $1",
        )
        .bind(session_id)
        .bind(score as i32)
        .bind(passed)
        .execute(&mut *conn)
        .await?;
    }
    Ok(mock_id)
}

async fn recalculate_mock_percentiles_on(
    conn: &mut PgConnection,
    mock_id: Uuid,
    minimum_sample: i64,
) -> ApiResult<()> {
    let rows = sqlx::query(
        r#"SELECT ma.session_id, ma.score_percent
           FROM mock_attempts ma WHERE ma.mock_id = $1 ORDER BY ma.session_id"#,
    )
    .bind(mock_id)
    .fetch_all(&mut *conn)
    .await?;
    let takers = rows.len() as i64;
    let scores: Vec<(Uuid, i32)> = rows
        .into_iter()
        .map(|row| Ok((row.try_get("session_id")?, row.try_get("score_percent")?)))
        .collect::<Result<_, sqlx::Error>>()?;
    for (session_id, score) in &scores {
        let below = scores.iter().filter(|(_, other)| other < score).count() as i64;
        let percentile = if takers >= minimum_sample && takers > 1 {
            Some((below * 100 / (takers - 1)) as i32)
        } else {
            None
        };
        sqlx::query("UPDATE mock_attempts SET percentile = $2 WHERE session_id = $1")
            .bind(session_id)
            .bind(percentile)
            .execute(&mut *conn)
            .await?;
        let stored = sqlx::query_scalar::<_, Option<Value>>(
            "SELECT result_payload FROM practice_sessions WHERE id = $1",
        )
        .bind(session_id)
        .fetch_one(&mut *conn)
        .await?;
        if let Some(mut receipt) = stored {
            if receipt["mock"].is_object() {
                receipt["mock"]["percentile"] = json!(percentile);
                receipt["mock"]["takers"] = json!(takers);
                sqlx::query("UPDATE practice_sessions SET result_payload = $2 WHERE id = $1")
                    .bind(session_id)
                    .bind(receipt)
                    .execute(&mut *conn)
                    .await?;
            }
        }
    }
    Ok(())
}

async fn apply_question_correction_on(
    conn: &mut PgConnection,
    task_id: Uuid,
    old_version_id: Uuid,
    new_version_id: Uuid,
    minimum_sample: i64,
) -> ApiResult<Value> {
    let versions = sqlx::query(
        r#"SELECT old.status AS old_status, old.options AS old_options,
                  old.chapter_id AS chapter_id, oq.family_id AS old_family,
                  new.status AS new_status, new.options AS new_options,
                  new.correct_index AS new_correct_index, nq.family_id AS new_family
           FROM question_versions old
           JOIN questions oq ON oq.id = old.question_id
           JOIN question_versions new ON new.id = $2
           JOIN questions nq ON nq.id = new.question_id
           WHERE old.id = $1"#,
    )
    .bind(old_version_id)
    .bind(new_version_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| ApiError::not_found("correction_version_not_found"))?;
    let old_family: Uuid = versions.try_get("old_family")?;
    let new_family: Uuid = versions.try_get("new_family")?;
    if old_family != new_family {
        return Err(ApiError::unprocessable(
            "correction_family_mismatch",
            "replacement question must belong to the same question family",
        ));
    }
    let new_status: String = versions.try_get("new_status")?;
    if new_status != "published" {
        return Err(ApiError::conflict(
            "correction_version_unpublished",
            "replacement question version must be published",
        ));
    }
    let old_options: Value = versions.try_get("old_options")?;
    let new_options: Value = versions.try_get("new_options")?;
    if !same_option_texts(&old_options, &new_options) {
        return Err(ApiError::unprocessable(
            "correction_option_identity_mismatch",
            "replacement must preserve every option text and its order",
        ));
    }
    let old_status: String = versions.try_get("old_status")?;
    if old_status != "quarantined" && old_status != "archived" {
        return Err(ApiError::conflict(
            "question_version_not_quarantined",
            "the source-linked question version is no longer quarantined",
        ));
    }
    let correct_index: i16 = versions.try_get("new_correct_index")?;
    let option_count = new_options.as_array().map_or(0, Vec::len);
    if correct_index < 0 || correct_index as usize >= option_count {
        return Err(ApiError::unprocessable(
            "invalid_correct_index",
            "replacement answer key is outside the option list",
        ));
    }
    let chapter_id: Uuid = versions.try_get("chapter_id")?;
    let attempts = sqlx::query(
        r#"SELECT a.id, a.user_id, a.session_id, a.chosen_index, a.correct
           FROM attempts a
           JOIN practice_sessions s ON s.id = a.session_id AND s.status = 'submitted'
           WHERE a.question_version_id = $1
           ORDER BY a.created_at, a.id FOR UPDATE OF a"#,
    )
    .bind(old_version_id)
    .fetch_all(&mut *conn)
    .await?;
    let mut changed_users = std::collections::HashSet::new();
    let mut changed_sessions = std::collections::HashSet::new();
    let mut updated_attempts = 0i64;
    for attempt in attempts {
        let attempt_id: Uuid = attempt.try_get("id")?;
        let user_id: Uuid = attempt.try_get("user_id")?;
        let session_id: Uuid = attempt.try_get("session_id")?;
        let chosen: Option<i16> = attempt.try_get("chosen_index")?;
        let old_correct: Option<bool> = attempt.try_get("correct")?;
        let new_correct = chosen.map(|chosen| chosen == correct_index);
        if old_correct == new_correct {
            continue;
        }
        sqlx::query("UPDATE attempts SET correct = $2 WHERE id = $1")
            .bind(attempt_id)
            .bind(new_correct)
            .execute(&mut *conn)
            .await?;
        sqlx::query(
            r#"INSERT INTO attempt_corrections
                 (id, task_id, attempt_id, old_correct, new_correct, corrected_question_version_id)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(Uuid::new_v4())
        .bind(task_id)
        .bind(attempt_id)
        .bind(old_correct)
        .bind(new_correct)
        .bind(new_version_id)
        .execute(&mut *conn)
        .await?;
        changed_users.insert(user_id);
        changed_sessions.insert(session_id);
        updated_attempts += 1;
    }
    for user_id in &changed_users {
        crate::agent::recompute_learner_chapter_on(&mut *conn, *user_id, chapter_id).await?;
    }
    let mut affected_mocks = std::collections::HashSet::new();
    for session_id in &changed_sessions {
        if let Some(mock_id) = refresh_session_receipt_on(&mut *conn, *session_id).await? {
            affected_mocks.insert(mock_id);
        }
    }
    for mock_id in affected_mocks {
        recalculate_mock_percentiles_on(&mut *conn, mock_id, minimum_sample).await?;
    }
    Ok(json!({
        "updated_attempts": updated_attempts,
        "recalculated_learners": changed_users.len(),
        "refreshed_sessions": changed_sessions.len(),
    }))
}

pub async fn resolve_task(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
    Json(req): Json<ResolveTaskReq>,
) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    if !matches!(
        req.resolution.as_str(),
        "reviewed_current" | "corrected" | "retired"
    ) {
        return Err(ApiError::unprocessable(
            "invalid_source_task_resolution",
            "resolution must be reviewed_current, corrected, or retired",
        ));
    }
    let note = req
        .resolution_note
        .as_deref()
        .map(|note| {
            clean_text(
                note,
                2000,
                "invalid_resolution_note",
                "resolution note must be 1-2000 characters",
            )
        })
        .transpose()?
        .unwrap_or_default();
    if (req.resolution == "corrected") != req.corrected_version_id.is_some() {
        return Err(ApiError::unprocessable(
            "correction_version_required",
            "a replacement version is required only for a corrected resolution",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let task = sqlx::query(
        r#"SELECT t.event_id, t.dependency_id, t.resource_kind, t.question_version_id,
                  t.article_version_id, t.scenario_version_id, t.status,
                  t.resolution, t.corrected_question_version_id, e.classification
           FROM source_change_tasks t
           JOIN source_change_events e ON e.id = t.event_id
           WHERE t.id = $1 FOR UPDATE OF t"#,
    )
    .bind(task_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("source_change_task_not_found"))?;
    let event_id: Uuid = task.try_get("event_id")?;
    let dependency_id: Uuid = task.try_get("dependency_id")?;
    let kind: String = task.try_get("resource_kind")?;
    let question_version_id: Option<Uuid> = task.try_get("question_version_id")?;
    let article_version_id: Option<Uuid> = task.try_get("article_version_id")?;
    let scenario_version_id: Option<Uuid> = task.try_get("scenario_version_id")?;
    let classification: String = task.try_get("classification")?;
    let current_status: String = task.try_get("status")?;
    let current_resolution: Option<String> = task.try_get("resolution")?;
    let current_corrected: Option<Uuid> = task.try_get("corrected_question_version_id")?;
    if current_status == "resolved" {
        if current_resolution.as_deref() == Some(req.resolution.as_str())
            && current_corrected == req.corrected_version_id
        {
            let mut body = case_json_on(&mut tx, event_id).await?;
            body["already_resolved"] = json!(true);
            tx.commit().await?;
            return Ok(Json(body));
        }
        return Err(ApiError::conflict(
            "source_change_task_resolved",
            "a resolved source review task cannot be changed",
        ));
    }
    if req.resolution == "reviewed_current"
        && matches!(
            classification.as_str(),
            "invalid_answer_key" | "source_withdrawn"
        )
    {
        return Err(ApiError::conflict(
            "unsafe_content_requires_correction_or_retirement",
            "unsafe content must be replaced or retired before closing review",
        ));
    }
    let correction = if req.resolution == "corrected" {
        if classification != "invalid_answer_key" || kind != "question" {
            return Err(ApiError::unprocessable(
                "correction_not_supported_for_resource",
                "answer-key corrections require an invalid-answer-key question task",
            ));
        }
        apply_question_correction_on(
            &mut tx,
            task_id,
            question_version_id.ok_or_else(ApiError::internal)?,
            req.corrected_version_id.ok_or_else(ApiError::internal)?,
            state.community_min_sample,
        )
        .await?
    } else {
        Value::Null
    };
    if req.resolution == "corrected" {
        sqlx::query(
            r#"INSERT INTO source_dependencies
                 (id, source_passage_id, resource_kind, question_version_id, created_by)
               SELECT gen_random_uuid(), source_passage_id, 'question', $2, $3
               FROM source_dependencies WHERE id = $1
               ON CONFLICT DO NOTHING"#,
        )
        .bind(dependency_id)
        .bind(req.corrected_version_id.ok_or_else(ApiError::internal)?)
        .bind(user.user_id)
        .execute(&mut *tx)
        .await?;
    }
    if req.resolution == "retired" {
        match kind.as_str() {
            "question" => {
                sqlx::query("UPDATE question_versions SET status = 'archived' WHERE id = $1")
                    .bind(question_version_id.ok_or_else(ApiError::internal)?)
                    .execute(&mut *tx)
                    .await?;
            }
            "article" => {
                sqlx::query("UPDATE article_versions SET status = 'archived' WHERE id = $1")
                    .bind(article_version_id.ok_or_else(ApiError::internal)?)
                    .execute(&mut *tx)
                    .await?;
            }
            "scenario" => {
                sqlx::query("UPDATE scenario_versions SET status = 'archived' WHERE id = $1")
                    .bind(scenario_version_id.ok_or_else(ApiError::internal)?)
                    .execute(&mut *tx)
                    .await?;
            }
            _ => return Err(ApiError::internal()),
        }
    }
    sqlx::query(
        r#"UPDATE source_change_tasks
           SET status = 'resolved', resolution = $2, resolution_note = $3,
               corrected_question_version_id = $4, resolved_by = $5, resolved_at = now()
           WHERE id = $1"#,
    )
    .bind(task_id)
    .bind(&req.resolution)
    .bind(&note)
    .bind(req.corrected_version_id)
    .bind(user.user_id)
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "source_change_task_resolved",
        "source_change_task",
        task_id,
        json!({
            "resolution": req.resolution,
            "resolution_note": note,
            "corrected_question_version_id": req.corrected_version_id,
            "source_dependency_inherited": req.resolution == "corrected",
            "correction": correction,
        }),
    )
    .await?;
    if req.resolution == "corrected" {
        sqlx::query(
            r#"INSERT INTO notifications (id, user_id, category, title, body, deep_link)
               SELECT gen_random_uuid(), l.user_id, 'content_update',
                      'Question answer key corrected',
                      'A reviewed source correction changed an answer key. Your stored practice results were updated where needed.',
                      '/notifications'
               FROM source_change_task_learners l
               LEFT JOIN notification_preferences p ON p.user_id = l.user_id
               WHERE l.task_id = $1 AND COALESCE(p.content_updates, TRUE)"#,
        )
        .bind(task_id)
        .execute(&mut *tx)
        .await?;
    }
    let mut body = case_json_on(&mut tx, event_id).await?;
    body["correction"] = correction;
    tx.commit().await?;
    Ok(Json(body))
}
