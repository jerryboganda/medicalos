//! Deterministic learner-state updates and plan revisions. No LLM here by
//! design (§9.1): arithmetic, scheduling invariants and plan mutations are
//! ordinary tested code; language models arrive with the Coach slice.

use std::collections::HashMap;

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiResult;

// --- today's plan -----------------------------------------------------------

/// Latest version of today's plan, creating the cold-start plan when needed
/// (AI-02: a modest first plan, one chapter task, no diagnostics demanded).
pub async fn get_or_create_today(pool: &sqlx::PgPool, user_id: Uuid) -> ApiResult<(Uuid, i32)> {
    let latest = sqlx::query!(
        "SELECT id, version FROM plans
         WHERE user_id = $1 AND plan_date = CURRENT_DATE
         ORDER BY version DESC LIMIT 1",
        user_id
    )
    .fetch_optional(pool)
    .await?;
    if let Some(p) = latest {
        return Ok((p.id, p.version));
    }
    let plan_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO plans (id, user_id, version) VALUES ($1, $2, 1)",
        plan_id,
        user_id
    )
    .execute(pool)
    .await?;
    // Single-exam fixture world: the first chapter is the exam's first. The
    // selection policy of 8.8 replaces this when several exams exist.
    let first = sqlx::query!(
        "SELECT id, name FROM curriculum_nodes WHERE kind = 'chapter'
         ORDER BY display_order, name LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    if let Some(ch) = first {
        let title = format!("Tutor practice: {} (10 questions)", ch.name);
        sqlx::query!(
            "INSERT INTO plan_tasks (id, plan_id, kind, title, chapter_id, question_count)
             VALUES ($1, $2, 'practice', $3, $4, 10)",
            Uuid::new_v4(),
            plan_id,
            title,
            ch.id
        )
        .execute(pool)
        .await?;
    }
    Ok((plan_id, 1))
}

/// New plan version copying every task of the current version (audit trail
/// lives in plan_revisions + the old version rows).
pub async fn fork_plan(pool: &sqlx::PgPool, plan_id: Uuid, version: i32) -> ApiResult<(Uuid, i32)> {
    let new_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO plans (id, user_id, plan_date, version)
         SELECT $1, user_id, plan_date, $3 FROM plans WHERE id = $2",
        new_id,
        plan_id,
        version + 1
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        "INSERT INTO plan_tasks
           (id, plan_id, kind, title, chapter_id, question_count, source_session_id,
            status, added_by_revision)
         SELECT gen_random_uuid(), $1, kind, title, chapter_id, question_count,
                source_session_id, status, added_by_revision
         FROM plan_tasks WHERE plan_id = $2",
        new_id,
        plan_id
    )
    .execute(pool)
    .await?;
    Ok((new_id, version + 1))
}

pub async fn mark_matching_task_done(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    chapter_id: Option<Uuid>,
    source_session_id: Option<Uuid>,
) -> ApiResult<()> {
    let (plan_id, _) = get_or_create_today(pool, user_id).await?;
    sqlx::query!(
        "UPDATE plan_tasks SET status = 'done'
         WHERE plan_id = $1 AND status = 'pending'
           AND ((kind = 'practice' AND chapter_id IS NOT DISTINCT FROM $2)
             OR (kind = 'revision' AND source_session_id IS NOT DISTINCT FROM $3))",
        plan_id,
        chapter_id,
        source_session_id
    )
    .execute(pool)
    .await?;
    Ok(())
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

// --- plan revision (PLAN-01, AI-07 minimal) ----------------------------------

/// After a tutor session with missed questions: fork the plan, add one
/// capped re-practice task, and persist the receipt. Revision sessions never
/// spawn further revisions here (anti-loop, §8.6); the SR-08 re-test queue
/// is Phase 2.
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
    let (old_plan_id, from_version) = get_or_create_today(pool, user_id).await?;
    let (new_plan_id, to_version) = fork_plan(pool, old_plan_id, from_version).await?;
    let title = format!("Re-practice: {} missed question(s)", missed);
    sqlx::query!(
        "INSERT INTO plan_tasks (id, plan_id, kind, title, question_count, source_session_id)
         VALUES ($1, $2, 'revision', $3, $4, $5)",
        Uuid::new_v4(),
        new_plan_id,
        title,
        missed as i32,
        session_id
    )
    .execute(pool)
    .await?;
    let receipt = json!({
        "triggering": {
            "session_id": session_id,
            "incorrect": incorrect,
            "skipped": skipped,
        },
        "checks": [
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
            automatic, receipt)
         VALUES ($1, $2, $3, $4, 'incorrect_answers', $5, true, $6)",
        Uuid::new_v4(),
        new_plan_id,
        from_version,
        to_version,
        explanation,
        receipt
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_revision_undone(pool: &sqlx::PgPool, revision_id: Uuid) -> ApiResult<()> {
    let stamp = json!({"undone": true, "undone_at": Utc::now().to_rfc3339()});
    sqlx::query!(
        "UPDATE plan_revisions SET undone = true, receipt = receipt || $2::jsonb WHERE id = $1",
        revision_id,
        stamp
    )
    .execute(pool)
    .await?;
    Ok(())
}
