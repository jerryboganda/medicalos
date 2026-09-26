//! Integrity event recording and server-enforced mock away-time policy (§11.3/EX-08).

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct IntegrityEventReq {
    pub session_id: Option<Uuid>,
    pub signal_type: String,
    #[serde(default)]
    pub detail: serde_json::Value,
    pub client_time: Option<chrono::DateTime<chrono::Utc>>,
}

const VALID_SIGNALS: &[&str] = &[
    "background",
    "screenshot",
    "screen_record",
    "split_screen",
    "clock_change",
    "fullscreen_exit",
    "window_blur",
    "foreground",
    "attestation_failure",
    "second_session",
];

pub async fn record_integrity_event(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<IntegrityEventReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !VALID_SIGNALS.contains(&req.signal_type.as_str()) {
        return Err(ApiError::unprocessable(
            "invalid_signal",
            format!("signal_type must be one of: {}", VALID_SIGNALS.join(", ")),
        ));
    }

    let mut action = "none";
    let mut away_seconds = None;
    let mut auto_submit = None;
    let id = Uuid::new_v4();
    let mut tx = state.pool.begin().await?;

    if let Some(session_id) = req.session_id {
        let session = sqlx::query(
            "SELECT preset, status, integrity_policy, away_timeout_seconds, away_since,
                    auto_submitted_by_policy
             FROM practice_sessions
             WHERE id = $1 AND user_id = $2
             FOR UPDATE",
        )
        .bind(session_id)
        .bind(user.user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("session_not_found"))?;
        let is_mock: bool = session.try_get::<String, _>("preset")? == "mock";
        let status: String = session.try_get("status")?;
        let is_open = status == "open";

        if is_mock {
            match req.signal_type.as_str() {
                "background" | "window_blur" if is_open => {
                    sqlx::query(
                        "UPDATE practice_sessions
                         SET away_since = COALESCE(away_since, clock_timestamp())
                         WHERE id = $1 AND status = 'open'",
                    )
                    .bind(session_id)
                    .execute(&mut *tx)
                    .await?;
                }
                "foreground" if is_open || status == "submitted" => {
                    let since: Option<chrono::DateTime<chrono::Utc>> =
                        session.try_get("away_since")?;
                    let policy: String = session.try_get("integrity_policy")?;
                    let timeout: Option<i32> = session.try_get("away_timeout_seconds")?;
                    let already_auto_submitted: bool =
                        session.try_get("auto_submitted_by_policy")?;
                    let elapsed = if let Some(since) = since {
                        let server_now: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
                            "SELECT clock_timestamp()",
                        )
                        .fetch_one(&mut *tx)
                        .await?;
                        let elapsed = (server_now - since).num_seconds().max(0);
                        away_seconds = Some(elapsed);
                        Some(elapsed)
                    } else {
                        None
                    };
                    let timeout_elapsed = elapsed
                        .zip(timeout)
                        .is_some_and(|(elapsed, limit)| elapsed >= i64::from(limit));
                    let submit_by_policy = policy == "auto_submit"
                        && (already_auto_submitted || (is_open && timeout_elapsed));
                    if policy == "warn" && is_open && timeout_elapsed {
                        action = "warn";
                    }
                    if submit_by_policy {
                        action = "auto_submitted";
                        auto_submit = Some(session_id);
                    }
                    sqlx::query(
                        "UPDATE practice_sessions
                         SET away_since = CASE WHEN $2 THEN away_since ELSE NULL END,
                             auto_submitted_by_policy = auto_submitted_by_policy OR $2
                         WHERE id = $1",
                    )
                    .bind(session_id)
                    .bind(submit_by_policy)
                    .execute(&mut *tx)
                    .await?;
                }
                _ => {}
            }
        }

    }
    sqlx::query(
        "INSERT INTO integrity_events
           (id, user_id, session_id, signal_type, detail, client_time)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(user.user_id)
    .bind(req.session_id)
    .bind(&req.signal_type)
    .bind(&req.detail)
    .bind(req.client_time)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let mut response = json!({
        "recorded": true,
        "event_id": id,
        "action": action,
    });
    if let Some(seconds) = away_seconds {
        response["away_seconds"] = json!(seconds);
    }
    if let Some(session_id) = auto_submit {
        let Json(receipt) = crate::routes::practice::submit(
            State(state.clone()),
            AuthUser {
                user_id: user.user_id,
            },
            Path(session_id),
        )
        .await?;
        response["receipt"] = receipt;
        sqlx::query(
            "UPDATE practice_sessions SET away_since = NULL
             WHERE id = $1 AND status = 'submitted' AND auto_submitted_by_policy = TRUE",
        )
        .bind(session_id)
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(response))
}

/// Complete one bounded worker tick. Persisted away intervals and policy
/// claims survive API restarts; submit's session lock makes retries idempotent.
pub async fn process_due_auto_submits(state: &Arc<AppState>) -> ApiResult<usize> {
    let due = sqlx::query(
        "WITH overdue AS (
             SELECT id FROM practice_sessions
             WHERE preset = 'mock' AND status = 'open'
               AND integrity_policy = 'auto_submit'
               AND (auto_submitted_by_policy OR (
                    away_since IS NOT NULL AND away_timeout_seconds IS NOT NULL
                    AND away_since + away_timeout_seconds * interval '1 second'
                        <= clock_timestamp()
               ))
             ORDER BY COALESCE(away_since, created_at)
             LIMIT 100 FOR UPDATE SKIP LOCKED
         )
         UPDATE practice_sessions AS session
         SET auto_submitted_by_policy = TRUE
         FROM overdue
         WHERE session.id = overdue.id
         RETURNING session.id, session.user_id",
    )
    .fetch_all(&state.pool)
    .await?;
    let mut submitted = 0;
    for row in due {
        let session_id: Uuid = row.try_get("id")?;
        let user_id: Uuid = row.try_get("user_id")?;
        match crate::routes::practice::submit(
            State(state.clone()),
            AuthUser { user_id },
            Path(session_id),
        )
        .await
        {
            Ok(_) => {
                sqlx::query(
                    "UPDATE practice_sessions SET away_since = NULL
                     WHERE id = $1 AND status = 'submitted'
                       AND auto_submitted_by_policy = TRUE",
                )
                .bind(session_id)
                .execute(&state.pool)
                .await?;
                submitted += 1;
            }
            Err(error) => {
                tracing::warn!(%error, %session_id, "could not auto-submit overdue mock");
            }
        }
    }
    Ok(submitted)
}

pub fn spawn_auto_submit_worker(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(Duration::from_secs(5));
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            if let Err(error) = process_due_auto_submits(&state).await {
                tracing::warn!(%error, "integrity auto-submit worker tick failed");
            }
        }
    });
}
