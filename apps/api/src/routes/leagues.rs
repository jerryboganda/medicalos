//! COMP-03: opt-in weekly competition leagues.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Datelike;
use serde_json::json;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const COHORT_SIZE: i64 = 30;
const PROMOTION_SIZE: usize = 3;

fn week_start(date: chrono::NaiveDate) -> chrono::NaiveDate {
    date - chrono::Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

async fn lock_exam(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, exam_id: Uuid) -> ApiResult<()> {
    sqlx::query("SELECT id FROM exams WHERE id = $1 FOR UPDATE")
        .bind(exam_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| ApiError::not_found("exam_not_found"))?;
    Ok(())
}

/// Return the promoted/relegated entrants for a previous cohort. Non-entrants
/// remain in their division, and a cohort needs ten active members plus one
/// completed score to move anyone.
async fn cohort_movements(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cohort_id: Uuid,
    previous_week: chrono::NaiveDate,
) -> ApiResult<HashMap<Uuid, i32>> {
    let cohort = sqlx::query(
        "SELECT division,
                (SELECT COUNT(*) FROM competition_league_memberships
                 WHERE cohort_id = $1 AND left_at IS NULL) AS member_count
         FROM competition_league_cohorts WHERE id = $1",
    )
    .bind(cohort_id)
    .fetch_one(&mut **tx)
    .await?;
    let division: i32 = cohort.try_get("division")?;
    let member_count: i64 = cohort.try_get("member_count")?;
    if member_count < 10 {
        return Ok(HashMap::new());
    }

    let ranking = sqlx::query(
        "SELECT member.user_id,
                SUM(entry.score)::DOUBLE PRECISION AS points,
                SUM(entry.correct_count)::DOUBLE PRECISION /
                    NULLIF(SUM(entry.attempted_count), 0) AS accuracy,
                SUM(entry.total_time_ms)::BIGINT AS total_time_ms
         FROM competition_league_memberships member
         JOIN competitions event
           ON event.exam_id = member.exam_id
          AND event.ends_at >= ($2::date::timestamp AT TIME ZONE 'UTC')
          AND event.ends_at < (($2::date + 7)::timestamp AT TIME ZONE 'UTC')
         JOIN competition_entries entry
           ON entry.competition_id = event.id AND entry.user_id = member.user_id
          AND entry.submitted_at >= ($2::date::timestamp AT TIME ZONE 'UTC')
          AND entry.submitted_at < (($2::date + 7)::timestamp AT TIME ZONE 'UTC')
         WHERE member.cohort_id = $1 AND member.left_at IS NULL
         GROUP BY member.user_id
         HAVING SUM(entry.attempted_count) > 0
         ORDER BY points DESC, accuracy DESC, total_time_ms ASC, member.user_id ASC",
    )
    .bind(cohort_id)
    .bind(previous_week)
    .fetch_all(&mut **tx)
    .await?;
    let mut movements = HashMap::new();
    for (index, row) in ranking.iter().enumerate() {
        let new_division = if index < PROMOTION_SIZE {
            Some(division.checked_add(1).ok_or_else(|| {
                ApiError::unprocessable("league_division_limit", "league division limit reached")
            })?)
        } else if division > 1 && index >= ranking.len().saturating_sub(PROMOTION_SIZE) {
            Some(division - 1)
        } else {
            None
        };
        if let Some(new_division) = new_division {
            movements.insert(row.try_get("user_id")?, new_division);
        }
    }
    Ok(movements)
}

async fn find_or_create_cohort(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    exam_id: Uuid,
    week: chrono::NaiveDate,
    division: i32,
) -> ApiResult<Uuid> {
    let available = sqlx::query(
        "SELECT cohort.id
         FROM competition_league_cohorts cohort
         WHERE cohort.exam_id = $1 AND cohort.week_start = $2 AND cohort.division = $3
           AND (SELECT COUNT(*) FROM competition_league_memberships member
                WHERE member.cohort_id = cohort.id AND member.left_at IS NULL) < $4
         ORDER BY cohort.cohort_number
         LIMIT 1 FOR UPDATE",
    )
    .bind(exam_id)
    .bind(week)
    .bind(division)
    .bind(COHORT_SIZE)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = available {
        return Ok(row.try_get("id")?);
    }

    let number: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(cohort_number), 0) + 1
         FROM competition_league_cohorts
         WHERE exam_id = $1 AND week_start = $2 AND division = $3",
    )
    .bind(exam_id)
    .bind(week)
    .bind(division)
    .fetch_one(&mut **tx)
    .await?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO competition_league_cohorts
           (id, exam_id, week_start, division, cohort_number)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(exam_id)
    .bind(week)
    .bind(division)
    .bind(number)
    .execute(&mut **tx)
    .await?;
    Ok(id)
}

/// Carry a bounded batch of the active roster into this week. The requesting
/// learner is first, previous cohorts stay adjacent, and each write is
/// serialized by the exam lock.
async fn ensure_current_memberships(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    exam_id: Uuid,
    week: chrono::NaiveDate,
    priority_user: Uuid,
) -> ApiResult<()> {
    let players = sqlx::query(
        "SELECT roster.user_id, roster.handle, roster.division,
                prior.cohort_id AS prior_cohort_id,
                prior_cohort.division AS prior_division,
                EXISTS (
                    SELECT 1 FROM competition_league_memberships existing
                    WHERE existing.exam_id = roster.exam_id
                      AND existing.week_start = $2 AND existing.user_id = roster.user_id
                      AND existing.left_at IS NOT NULL
                ) AS was_left
         FROM competition_league_players roster
         LEFT JOIN competition_league_memberships prior
           ON prior.exam_id = roster.exam_id AND prior.user_id = roster.user_id
          AND prior.week_start = $2 - 7 AND prior.left_at IS NULL
         LEFT JOIN competition_league_cohorts prior_cohort
           ON prior_cohort.id = prior.cohort_id
         WHERE roster.exam_id = $1 AND roster.active = true
           AND NOT EXISTS (
               SELECT 1 FROM competition_league_memberships existing
               WHERE existing.exam_id = roster.exam_id AND existing.week_start = $2
                 AND existing.user_id = roster.user_id AND existing.left_at IS NULL
           )
         ORDER BY (roster.user_id = $3) DESC, prior.cohort_id NULLS LAST,
                  roster.division, roster.joined_at, roster.user_id
         LIMIT 31 FOR UPDATE OF roster",
    )
    .bind(exam_id)
    .bind(week)
    .bind(priority_user)
    .fetch_all(&mut **tx)
    .await?;

    let previous_week = week - chrono::Duration::days(7);
    let mut movements_by_cohort: HashMap<Uuid, HashMap<Uuid, i32>> = HashMap::new();

    for player in players {
        let user_id: Uuid = player.try_get("user_id")?;
        let handle: String = player.try_get("handle")?;
        let saved_division: i32 = player.try_get("division")?;
        let was_left: bool = player.try_get("was_left")?;
        let prior_cohort_id: Option<Uuid> = player.try_get("prior_cohort_id")?;
        let prior_division: Option<i32> = player.try_get("prior_division")?;

        let mut division = saved_division;
        if !was_left {
            if let (Some(cohort_id), Some(previous_division)) = (prior_cohort_id, prior_division) {
                division = previous_division;
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    movements_by_cohort.entry(cohort_id)
                {
                    let movements = cohort_movements(tx, cohort_id, previous_week).await?;
                    entry.insert(movements);
                }
                if let Some(moved_division) = movements_by_cohort
                    .get(&cohort_id)
                    .and_then(|movement| movement.get(&user_id))
                {
                    division = *moved_division;
                }
            }
        }

        sqlx::query(
            "UPDATE competition_league_players SET division = $3
             WHERE exam_id = $1 AND user_id = $2",
        )
        .bind(exam_id)
        .bind(user_id)
        .bind(division)
        .execute(&mut **tx)
        .await?;
        let cohort_id = find_or_create_cohort(tx, exam_id, week, division).await?;
        if was_left {
            sqlx::query(
                "UPDATE competition_league_memberships
                 SET cohort_id = $4, handle = $5, joined_at = now(), left_at = NULL
                 WHERE exam_id = $1 AND week_start = $2 AND user_id = $3",
            )
            .bind(exam_id)
            .bind(week)
            .bind(user_id)
            .bind(cohort_id)
            .bind(handle)
            .execute(&mut **tx)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO competition_league_memberships
                   (cohort_id, exam_id, week_start, user_id, handle)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(cohort_id)
            .bind(exam_id)
            .bind(week)
            .bind(user_id)
            .bind(handle)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

async fn current_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    exam_id: Uuid,
    user_id: Uuid,
    week: chrono::NaiveDate,
) -> ApiResult<serde_json::Value> {
    let membership = sqlx::query(
        "SELECT cohort.id AS cohort_id, cohort.division, cohort.cohort_number
         FROM competition_league_memberships member
         JOIN competition_league_cohorts cohort ON cohort.id = member.cohort_id
         WHERE member.exam_id = $1 AND member.week_start = $2
           AND member.user_id = $3 AND member.left_at IS NULL",
    )
    .bind(exam_id)
    .bind(week)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(membership) = membership else {
        return Ok(json!({ "joined": false }));
    };
    let cohort_id: Uuid = membership.try_get("cohort_id")?;
    let division: i32 = membership.try_get("division")?;
    let cohort_number: i32 = membership.try_get("cohort_number")?;
    let scores = sqlx::query(
        "WITH totals AS (
            SELECT member.user_id, member.handle,
                   SUM(entry.score)::DOUBLE PRECISION AS points,
                   SUM(entry.correct_count)::DOUBLE PRECISION /
                       NULLIF(SUM(entry.attempted_count), 0) AS accuracy,
                   SUM(entry.total_time_ms)::BIGINT AS total_time_ms,
                   SUM(entry.attempted_count)::BIGINT AS attempted_count
            FROM competition_league_memberships member
            JOIN competitions event
              ON event.exam_id = member.exam_id
             AND event.ends_at >= ($2::date::timestamp AT TIME ZONE 'UTC')
             AND event.ends_at < (($2::date + 7)::timestamp AT TIME ZONE 'UTC')
            JOIN competition_entries entry
              ON entry.competition_id = event.id AND entry.user_id = member.user_id
             AND entry.submitted_at >= ($2::date::timestamp AT TIME ZONE 'UTC')
             AND entry.submitted_at < (($2::date + 7)::timestamp AT TIME ZONE 'UTC')
            WHERE member.cohort_id = $1 AND member.left_at IS NULL
            GROUP BY member.user_id, member.handle
         )
         SELECT user_id, handle, points, accuracy, total_time_ms,
                ROW_NUMBER() OVER (
                    ORDER BY points DESC, accuracy DESC, total_time_ms ASC, user_id ASC
                ) AS rank
         FROM totals WHERE attempted_count > 0 ORDER BY rank",
    )
    .bind(cohort_id)
    .bind(week)
    .fetch_all(&mut **tx)
    .await?;
    let standings = scores
        .iter()
        .map(|row| {
            Ok(json!({
                "rank": row.try_get::<i64, _>("rank")?,
                "handle": row.try_get::<String, _>("handle")?,
                "points": row.try_get::<f64, _>("points")?,
                "accuracy": row.try_get::<f64, _>("accuracy")?,
                "total_time_ms": row.try_get::<i64, _>("total_time_ms")?,
                "is_me": row.try_get::<Uuid, _>("user_id")? == user_id,
            }))
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(json!({
        "joined": true,
        "exam_id": exam_id,
        "cohort_id": cohort_id,
        "week_start": week,
        "week_end": week + chrono::Duration::days(7),
        "division": division,
        "cohort_number": cohort_number,
        "standings": standings,
    }))
}

pub async fn state(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(exam_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let week = week_start(chrono::Utc::now().date_naive());
    let mut tx = state.pool.begin().await?;
    lock_exam(&mut tx, exam_id).await?;
    let opted_in: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM competition_league_players
            WHERE exam_id = $1 AND user_id = $2 AND active = true
        )",
    )
    .bind(exam_id)
    .bind(user.user_id)
    .fetch_one(&mut *tx)
    .await?;
    if !opted_in {
        tx.commit().await?;
        return Ok(Json(json!({ "joined": false })));
    }
    ensure_current_memberships(&mut tx, exam_id, week, user.user_id).await?;
    let result = current_state(&mut tx, exam_id, user.user_id, week).await?;
    tx.commit().await?;
    Ok(Json(result))
}

pub async fn join(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(exam_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let week = week_start(chrono::Utc::now().date_naive());
    let mut tx = state.pool.begin().await?;
    lock_exam(&mut tx, exam_id).await?;
    let handle: String =
        sqlx::query_scalar("SELECT handle FROM community_profiles WHERE user_id = $1")
            .bind(user.user_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                ApiError::forbidden(
                    "not_opted_in",
                    "create a community profile to join a league",
                )
            })?;
    sqlx::query(
        "INSERT INTO competition_league_players (exam_id, user_id, handle)
         VALUES ($1, $2, $3)
         ON CONFLICT (exam_id, user_id) DO UPDATE
         SET handle = EXCLUDED.handle, active = true",
    )
    .bind(exam_id)
    .bind(user.user_id)
    .bind(handle)
    .execute(&mut *tx)
    .await?;
    ensure_current_memberships(&mut tx, exam_id, week, user.user_id).await?;
    let result = current_state(&mut tx, exam_id, user.user_id, week).await?;
    tx.commit().await?;
    Ok(Json(result))
}

pub async fn leave(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(exam_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let week = week_start(chrono::Utc::now().date_naive());
    let mut tx = state.pool.begin().await?;
    lock_exam(&mut tx, exam_id).await?;
    let deactivated = sqlx::query(
        "UPDATE competition_league_players SET active = false
         WHERE exam_id = $1 AND user_id = $2 AND active = true",
    )
    .bind(exam_id)
    .bind(user.user_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if deactivated > 0 {
        sqlx::query(
            "UPDATE competition_league_memberships SET left_at = now()
             WHERE exam_id = $1 AND week_start = $2 AND user_id = $3 AND left_at IS NULL",
        )
        .bind(exam_id)
        .bind(week)
        .bind(user.user_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({ "left": deactivated > 0 })))
}
