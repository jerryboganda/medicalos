//! Deterministic learner-state updates and plan revisions. No LLM here by
//! design (§9.1): arithmetic, scheduling invariants and plan mutations are
//! ordinary tested code; language models arrive with the Coach slice.

use std::collections::HashMap;

use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiResult;

// --- today's plan -----------------------------------------------------------

pub const MAX_AUTOMATIC_PLAN_REVISIONS_PER_DAY: i64 = 3;
pub const MAX_PLAN_REVISIONS_PER_DAY: i32 = 8;

/// Latest version of today's plan, creating the cold-start plan when needed
/// (AI-02: a modest first plan, one chapter task, no diagnostics demanded).
pub async fn get_or_create_today(pool: &sqlx::PgPool, user_id: Uuid) -> ApiResult<(Uuid, i32)> {
    let mut tx = pool.begin().await?;
    sqlx::query_scalar!("SELECT id FROM users WHERE id = $1 FOR UPDATE", user_id)
        .fetch_one(&mut *tx)
        .await?;
    let latest = sqlx::query!(
        "SELECT id, version FROM plans
         WHERE user_id = $1 AND plan_date = CURRENT_DATE
         ORDER BY version DESC LIMIT 1",
        user_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(p) = latest {
        tx.commit().await?;
        return Ok((p.id, p.version));
    }
    let plan_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO plans (id, user_id, version) VALUES ($1, $2, 1)",
        plan_id,
        user_id
    )
    .execute(&mut *tx)
    .await?;
    // Single-exam fixture world: the first chapter is the exam's first. The
    // selection policy of 8.8 replaces this when several exams exist.
    let first = sqlx::query!(
        "SELECT id, name FROM curriculum_nodes WHERE kind = 'chapter'
         ORDER BY display_order, name LIMIT 1"
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(ch) = first {
        let title = format!("Tutor practice: {} (10 questions)", ch.name);
        sqlx::query!(
            "INSERT INTO plan_tasks
               (id, plan_id, kind, title, chapter_id, question_count, estimated_minutes)
             VALUES ($1, $2, 'practice', $3, $4, 10, 15)",
            Uuid::new_v4(),
            plan_id,
            title,
            ch.id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok((plan_id, 1))
}

/// New plan version copying every task of the current version (audit trail
/// lives in plan_revisions + the old version rows).
pub async fn fork_plan_on(
    conn: &mut PgConnection,
    plan_id: Uuid,
    version: i32,
) -> ApiResult<(Uuid, i32)> {
    let new_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO plans (id, user_id, plan_date, version)
         SELECT $1, user_id, plan_date, $3 FROM plans WHERE id = $2",
        new_id,
        plan_id,
        version + 1
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "INSERT INTO plan_tasks
           (id, plan_id, kind, title, chapter_id, question_count, estimated_minutes,
            source_session_id, status, added_by_revision, protected, task_key, created_at)
         SELECT gen_random_uuid(), $1, kind, title, chapter_id, question_count,
                estimated_minutes, source_session_id, status, added_by_revision,
                protected, task_key, created_at
         FROM plan_tasks WHERE plan_id = $2 ORDER BY created_at, id",
        new_id,
        plan_id
    )
    .execute(&mut *conn)
    .await?;
    Ok((new_id, version + 1))
}

pub async fn mark_linked_task_done_on(
    conn: &mut PgConnection,
    user_id: Uuid,
    task_key: Option<Uuid>,
) -> ApiResult<()> {
    if let Some(task_key) = task_key {
        sqlx::query!(
            "UPDATE plan_tasks pt SET status = 'done'
             FROM plans p
             WHERE pt.plan_id = p.id AND p.user_id = $1
               AND p.plan_date = CURRENT_DATE AND pt.status = 'pending'
               AND pt.task_key = $2",
            user_id,
            task_key
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn count_today_revisions_on(conn: &mut PgConnection, user_id: Uuid) -> ApiResult<i64> {
    Ok(sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!"
           FROM plan_revisions r JOIN plans p ON p.id = r.plan_id
           WHERE p.user_id = $1 AND p.plan_date = CURRENT_DATE"#,
        user_id
    )
    .fetch_one(&mut *conn)
    .await?)
}

// --- learner state (AI-01, AI-17) --------------------------------------------

fn elo_update(ability: f32, difficulty: &str, correct: bool, k: f32) -> f32 {
    let d = match difficulty {
        "easy" => 1350.0,
        "hard" => 1650.0,
        _ => 1500.0,
    };
    let expected = 1.0 / (1.0 + 10f32.powf((d - ability) / 400.0));
    ability + k * (if correct { 1.0 } else { 0.0 } - expected)
}

/// Elo-style per-chapter update over this session's answered attempts.
/// Skipped answers are not evidence. Uncertainty is expressed as the evidence
/// count (selection policy reads it); the display layer shows honest
/// evidence levels (AI-02).
pub async fn update_learner_state(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    session_id: Uuid,
) -> ApiResult<()> {
    let rows = sqlx::query!(
        "SELECT qv.chapter_id, qv.difficulty, a.correct
         FROM attempts a JOIN question_versions qv ON qv.id = a.question_version_id
         WHERE a.session_id = $1 AND a.chosen_index IS NOT NULL",
        session_id
    )
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(());
    }
    let mut per_chapter: HashMap<Uuid, Vec<(String, bool)>> = HashMap::new();
    for r in &rows {
        per_chapter
            .entry(r.chapter_id)
            .or_default()
            .push((r.difficulty.clone(), r.correct == Some(true)));
    }
    for (chapter_id, answers) in per_chapter {
        let existing = sqlx::query!(
            "SELECT ability, evidence_count, independent_count
             FROM learner_concept_state WHERE user_id = $1 AND chapter_id = $2",
            user_id,
            chapter_id
        )
        .fetch_optional(pool)
        .await?;
        let (mut ability, mut evidence, mut independent) = match &existing {
            Some(r) => (r.ability, r.evidence_count, r.independent_count),
            None => (1500.0, 0, 0),
        };
        for (difficulty, correct) in &answers {
            let k = f32::max(8.0, 32.0 - 2.0 * independent as f32);
            ability = elo_update(ability, difficulty, *correct, k);
            evidence += 1;
            independent += 1; // slice 1 has no assisted paths yet
        }
        sqlx::query!(
            "INSERT INTO learner_concept_state
               (user_id, chapter_id, ability, evidence_count, independent_count)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id, chapter_id) DO UPDATE
               SET ability = $3, evidence_count = $4, independent_count = $5,
                   updated_at = now()",
            user_id,
            chapter_id,
            ability,
            evidence,
            independent
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Rebuild one chapter estimate from the learner's answer history after an
/// editorial answer-key correction changes effective correctness.
pub async fn recompute_learner_chapter_on(
    conn: &mut PgConnection,
    user_id: Uuid,
    chapter_id: Uuid,
) -> ApiResult<()> {
    let rows = sqlx::query!(
        r#"SELECT a.correct, qv.difficulty
           FROM attempts a
           JOIN question_versions qv ON qv.id = a.question_version_id
           WHERE a.user_id = $1 AND qv.chapter_id = $2
             AND a.chosen_index IS NOT NULL
           ORDER BY a.created_at, a.id"#,
        user_id,
        chapter_id
    )
    .fetch_all(&mut *conn)
    .await?;
    if rows.is_empty() {
        sqlx::query!(
            "DELETE FROM learner_concept_state WHERE user_id = $1 AND chapter_id = $2",
            user_id,
            chapter_id
        )
        .execute(&mut *conn)
        .await?;
        return Ok(());
    }

    let mut ability = 1500.0;
    let mut evidence = 0i32;
    for row in &rows {
        let k = f32::max(8.0, 32.0 - 2.0 * evidence as f32);
        ability = elo_update(ability, &row.difficulty, row.correct == Some(true), k);
        evidence += 1;
    }
    sqlx::query!(
        r#"INSERT INTO learner_concept_state
             (user_id, chapter_id, ability, evidence_count, independent_count)
           VALUES ($1, $2, $3, $4, $4)
           ON CONFLICT (user_id, chapter_id) DO UPDATE
             SET ability = $3, evidence_count = $4, independent_count = $4,
                 updated_at = now()"#,
        user_id,
        chapter_id,
        ability,
        evidence
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

// --- plan revision (PLAN-01, AI-07 minimal) ----------------------------------

/// After a tutor session with missed questions, add one re-practice task and
/// receipt within the daily revision caps. Revision sessions never spawn
/// further revisions here (anti-loop, §8.6).
pub async fn maybe_create_revision(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    session_id: Uuid,
    incorrect: i64,
    skipped: i64,
) -> ApiResult<()> {
    let missed = incorrect + skipped;
    if missed == 0 {
        return Ok(());
    }
    // Re-practice uses the same transparent 1.5-minutes-per-question baseline.
    let estimated_minutes = missed.saturating_mul(3).saturating_add(1) / 2;
    get_or_create_today(pool, user_id).await?;

    let mut tx = pool.begin().await?;
    // Plan-task completion and automatic revisions use the same lock order:
    // practice session first, then learner row. The source-event FK below
    // therefore cannot form a user/session lock cycle with submit retries.
    let source_exists = sqlx::query_scalar!(
        "SELECT id FROM practice_sessions
         WHERE id = $1 AND user_id = $2 AND status = 'submitted'
         FOR NO KEY UPDATE",
        session_id,
        user_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if source_exists.is_none() {
        tx.commit().await?;
        return Ok(());
    }
    sqlx::query_scalar!("SELECT id FROM users WHERE id = $1 FOR UPDATE", user_id)
        .fetch_one(&mut *tx)
        .await?;
    let current = sqlx::query!(
        "SELECT id, version FROM plans
         WHERE user_id = $1 AND plan_date = CURRENT_DATE
         ORDER BY version DESC LIMIT 1 FOR UPDATE",
        user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT source_event_id FROM plan_revisions
         WHERE source_event_id = $1",
    )
    .bind(session_id)
    .fetch_optional(&mut *tx)
    .await?;
    if existing.is_some() {
        tx.commit().await?;
        return Ok(());
    }
    if count_today_revisions_on(&mut *tx, user_id).await? >= i64::from(MAX_PLAN_REVISIONS_PER_DAY) {
        tx.commit().await?;
        return Ok(());
    }
    let automatic_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM plan_revisions r
         JOIN plans p ON p.id = r.plan_id
         WHERE p.user_id = $1 AND p.plan_date = CURRENT_DATE AND r.automatic",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    if automatic_count >= MAX_AUTOMATIC_PLAN_REVISIONS_PER_DAY {
        tx.commit().await?;
        return Ok(());
    }

    let (new_plan_id, to_version) = fork_plan_on(&mut *tx, current.id, current.version).await?;
    let revision_id = Uuid::new_v4();
    let title = format!("Re-practice: {} missed question(s)", missed);
    sqlx::query!(
        "INSERT INTO plan_tasks
           (id, plan_id, kind, title, question_count, estimated_minutes,
            source_session_id, added_by_revision)
         VALUES ($1, $2, 'revision', $3, $4, $5, $6, $7)",
        Uuid::new_v4(),
        new_plan_id,
        title,
        missed as i32,
        estimated_minutes.clamp(1, 480) as i32,
        session_id,
        revision_id
    )
    .execute(&mut *tx)
    .await?;
    let receipt = json!({
        "policy_version": "ai08-v1",
        "triggering": {
            "session_id": session_id,
            "incorrect": incorrect,
            "skipped": skipped,
        },
        "checks": [
            "daily_automatic_revision_limit_ok",
            "daily_total_revision_limit_ok",
            "capacity_ok: single task added",
            "revision_sessions_spawn_no_further_revisions",
        ],
        "diff": {"added_tasks": [title]},
    });
    let explanation = format!(
        "{} missed question(s) in your last session; a focused re-practice task was added.",
        missed
    );
    sqlx::query!(
        "INSERT INTO plan_revisions
           (id, plan_id, from_version, to_version, reason_code, explanation,
            automatic, receipt, source_event_id)
         VALUES ($1, $2, $3, $4, 'incorrect_answers', $5, true, $6, $7)",
        revision_id,
        new_plan_id,
        current.version,
        to_version,
        explanation,
        receipt,
        session_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
