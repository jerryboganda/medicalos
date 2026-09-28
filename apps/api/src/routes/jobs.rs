//! AI-05: bounded event-driven orchestration. A durable job queue claimed
//! with FOR UPDATE SKIP LOCKED, exponential backoff on failure, and a
//! dead-letter terminal state — executed by a background worker on the same
//! pattern as the EX-08 integrity auto-submit worker. Bounded by design:
//! a per-tick budget, a small closed set of job kinds, and per-job attempt
//! ceilings; nothing here fans out unbounded.

use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// How many due jobs a single worker tick may process.
pub const TICK_BUDGET: i64 = 50;
/// Seconds between worker ticks.
pub const TICK_SECONDS: u64 = 10;

/// Enqueue a job for the background worker. `payload` must carry exactly the
/// fields the kind's handler reads — unknown kinds fail at processing time
/// and dead-letter, they never block the request path.
pub async fn enqueue(
    pool: &PgPool,
    kind: &str,
    payload: serde_json::Value,
    created_by: Option<Uuid>,
) -> ApiResult<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO event_jobs (id, kind, payload, created_by) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(kind)
        .bind(payload)
        .bind(created_by)
        .execute(pool)
        .await?;
    Ok(id)
}

/// Claim and process due jobs until the tick budget is exhausted. Returns
/// the number of jobs processed (done, retried, or dead-lettered).
pub async fn process_due_jobs(state: &Arc<AppState>) -> ApiResult<usize> {
    let claimed = sqlx::query!(
        r#"UPDATE event_jobs SET status = 'processing', updated_at = now()
           WHERE id IN (
               SELECT id FROM event_jobs
                WHERE status = 'pending' AND run_after <= now()
                ORDER BY run_after
                LIMIT $1
                FOR UPDATE SKIP LOCKED
           )
           RETURNING id, kind, payload, attempts, max_attempts"#,
        TICK_BUDGET
    )
    .fetch_all(&state.pool)
    .await?;

    let mut processed = 0;
    for job in claimed {
        let outcome = dispatch(state, &job.kind, &job.payload).await;
        match outcome {
            Ok(()) => {
                sqlx::query!(
                    "UPDATE event_jobs SET status = 'done', updated_at = now() WHERE id = $1",
                    job.id
                )
                .execute(&state.pool)
                .await?;
            }
            Err(error) => {
                let attempts = job.attempts + 1;
                let (status, run_after, last_error) = if attempts >= job.max_attempts {
                    (
                        "failed",
                        None,
                        Some(format!("dead-letter after {attempts} attempts: {error}")),
                    )
                } else {
                    // Exponential backoff: 1, 2, 4, 8 ... minutes, capped at an hour.
                    let delay = std::cmp::min(60, 2_i64.pow(attempts.min(6) as u32 - 1));
                    (
                        "pending",
                        Some(chrono::Duration::minutes(delay)),
                        Some(truncate_error(&error)),
                    )
                };
                sqlx::query(
                    "UPDATE event_jobs
                        SET status = $2, attempts = $3,
                            run_after = COALESCE($4::timestamptz, run_after),
                            last_error = $5, updated_at = now()
                      WHERE id = $1",
                )
                .bind(job.id)
                .bind(status)
                .bind(attempts)
                .bind(run_after)
                .bind(last_error)
                .execute(&state.pool)
                .await?;
            }
        }
        processed += 1;
    }
    Ok(processed)
}

fn truncate_error(error: &ApiError) -> String {
    let text = error.to_string();
    if text.len() > 500 {
        format!("{}…", &text[..500])
    } else {
        text
    }
}

/// The closed set of job kinds. Adding orchestration work means adding a
/// match arm with its own guardrails — never an open dispatch.
async fn dispatch(state: &Arc<AppState>, kind: &str, payload: &serde_json::Value) -> ApiResult<()> {
    match kind {
        // AI-18: pre-generate one-tap tutoring cards for an answered question
        // version so the session detail read stays warm.
        "pregen_tutoring_cards" => {
            let question_version_id: Uuid = serde_json::from_value(
                payload
                    .get("question_version_id")
                    .cloned()
                    .ok_or_else(|| ApiError::unprocessable("missing_question_version_id", ""))?,
            )
            .map_err(|_| ApiError::unprocessable("invalid_question_version_id", ""))?;
            crate::routes::program::ensure_pregen(&state.pool, question_version_id)
                .await
                .map(|_| ())
        }
        _ => Err(ApiError::unprocessable(
            "unknown_job_kind",
            "no handler is registered for this job kind",
        )),
    }
}

/// AI-05: the bounded background worker. Same shape as the EX-08 integrity
/// worker: a fixed tick, a fixed budget, honest logging, no retries in-process.
pub fn spawn_jobs_worker(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(std::time::Duration::from_secs(TICK_SECONDS));
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            if let Err(error) = process_due_jobs(&state).await {
                tracing::warn!(?error, "event jobs worker tick failed");
            }
        }
    });
}
