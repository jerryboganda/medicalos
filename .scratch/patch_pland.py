import io

# --- program.rs: replan (PLAN-02), gap report (PLAN-04), selection policy (AI-17)
p = 'apps/api/src/routes/program.rs'
s = io.open(p, encoding='utf-8').read()

addition = '''
// ---- PLAN-02: capacity-driven replanning (§8.6) -----------------------------

#[derive(Deserialize)]
pub struct ReplanReq {
    /// The learner's real daily budget in minutes (5-480).
    pub daily_minutes: i32,
}

/// The learner (or a capacity change) sets a new daily budget; today's plan
/// is forked into a new version whose pending tasks fit the budget. Each
/// task costs question_count minutes (disclosed, deterministic). Done work
/// is never touched; trimmed tasks are reported, never hidden.
pub async fn replan_plan(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<ReplanReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if !(5..=480).contains(&req.daily_minutes) {
        return Err(ApiError::unprocessable(
            "invalid_capacity",
            "daily_minutes must be 5-480",
        ));
    }
    let (old_plan_id, from_version) =
        crate::agent::get_or_create_today(&state.pool, user.user_id).await?;
    let tasks = sqlx::query!(
        "SELECT id, title, question_count, status FROM plan_tasks
         WHERE plan_id = $1 ORDER BY created_at",
        old_plan_id
    )
    .fetch_all(&state.pool)
    .await?;
    let committed: i32 = tasks.iter().map(|t| t.question_count).sum();
    if committed <= req.daily_minutes {
        return Ok(Json(json!({
            "replanned": false,
            "reason": "within_capacity",
            "committed_minutes": committed,
            "daily_minutes": req.daily_minutes,
        })));
    }
    let (new_plan_id, to_version) =
        crate::agent::fork_plan(&state.pool, old_plan_id, from_version).await?;
    let mut kept: Vec<String> = Vec::new();
    let mut deferred: Vec<String> = Vec::new();
    let mut budget = req.daily_minutes;
    for t in &tasks {
        if t.status == "done" || t.question_count <= budget {
            budget -= t.question_count;
            kept.push(t.title.clone());
        } else {
            deferred.push(t.title.clone());
        }
    }
    // Drop the deferred tasks from the new version (the old version rows
    // keep them for the audit trail).
    for t in &tasks {
        if deferred.contains(&t.title) {
            sqlx::query!(
                "DELETE FROM plan_tasks WHERE plan_id = $1 AND id = $2",
                new_plan_id,
                t.id
            )
            .execute(&state.pool)
            .await?;
        }
    }
    let revision_id = Uuid::new_v4();
    let receipt = json!({
        "daily_minutes": req.daily_minutes,
        "committed_minutes_before": committed,
        "kept": kept,
        "deferred": deferred,
    });
    let explanation = format!(
        "Plan trimmed to your {}-minute budget: {} task(s) deferred.",
        req.daily_minutes,
        deferred.len()
    );
    sqlx::query!(
        "INSERT INTO plan_revisions
           (id, plan_id, from_version, to_version, reason_code, explanation,
            automatic, receipt)
         VALUES ($1, $2, $3, $4, 'capacity_change', $5, false, $6)",
        revision_id,
        new_plan_id,
        from_version,
        to_version,
        explanation,
        receipt
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({
        "replanned": true,
        "plan_id": new_plan_id,
        "version": to_version,
        "kept_tasks": kept.len(),
        "deferred_tasks": deferred.len(),
        "deferred": deferred,
    })))
}

// ---- PLAN-04: exam-switch knowledge-gap report (§8.4) -----------------------

/// Where the learner stands against a different exam's curriculum: per-chapter
/// independent-attempt coverage, honest about what "covered" means.
pub async fn exam_switch_gap_report(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(to_exam_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let exam_exists = sqlx::query!("SELECT 1 AS one FROM exams WHERE id = $1", to_exam_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("exam_not_found"))?;
    let _ = exam_exists;
    let rows = sqlx::query!(
        r#"SELECT c.id AS chapter_id, c.name AS chapter_name,
                  COALESCE(COUNT(DISTINCT a.id), 0) AS "attempts!"
           FROM curriculum_nodes c
           LEFT JOIN question_versions qv ON qv.chapter_id = c.id
           LEFT JOIN attempts a ON a.question_version_id = qv.id
                AND a.user_id = $1 AND a.assisted = FALSE
                AND a.correct IS NOT NULL
           WHERE c.exam_id = $2 AND c.kind = 'chapter'
           GROUP BY c.id, c.name ORDER BY c.name"#,
        user.user_id,
        to_exam_id
    )
    .fetch_all(&state.pool)
    .await?;
    // §8.8 honesty: fewer than 10 independent attempts is low evidence —
    // the report says so instead of inventing readiness.
    const COVERED_ATTEMPTS: i64 = 10;
    let chapters: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "chapter_id": r.chapter_id,
                "chapter_name": r.chapter_name,
                "independent_attempts": r.attempts,
                "covered": r.attempts >= COVERED_ATTEMPTS,
                "evidence": if r.attempts >= COVERED_ATTEMPTS { "covered" } else if r.attempts > 0 { "low_evidence" } else { "no_evidence" },
            })
        })
        .collect();
    let covered = chapters
        .iter()
        .filter(|c| c["covered"] == serde_json::Value::from(true))
        .count();
    Ok(Json(json!({
        "to_exam_id": to_exam_id,
        "coverage_rule": "at least 10 independent (non-assisted) answered attempts",
        "total_chapters": chapters.len(),
        "covered_chapters": covered,
        "chapters": chapters,
    })))
}

// ---- AI-17: transparent selection policy disclosure (§8.5) ------------------

/// The estimator is deterministic and disclosed: per-chapter ability from
/// real attempts with a shrinking K, difficulty anchors, and the selection
/// rules that turn those numbers into tasks. Nothing here is invented.
pub async fn selection_policy(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT c.name AS chapter_name, lcs.ability AS "ability?",
                  lcs.evidence_count AS "evidence!", lcs.independent_count AS "independent!"
           FROM learner_concept_state lcs
           JOIN curriculum_nodes c ON c.id = lcs.chapter_id
           WHERE lcs.user_id = $1 ORDER BY lcs.ability ASC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let k_of = |independent: i32| (32.0 - 2.0 * independent as f32).max(8.0);
    let chapters: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "chapter": r.chapter_name,
                "ability": r.ability,
                "current_k": k_of(r.independent),
                "evidence_count": r.evidence,
            })
        })
        .collect();
    Ok(Json(json!({
        "estimator": {
            "model": "elo_baseline",
            "base": 1500.0,
            "difficulty_anchors": {"easy": 1350.0, "medium": 1500.0, "hard": 1650.0},
            "k_rule": "max(8, 32 - 2 x independent_count) \u2014 shrinks as evidence grows",
            "counted_evidence": "independent, answered attempts only (skips and assisted answers excluded)",
        },
        "selection_rules": [
            "cold start: one modest practice task on the first chapter",
            "revision-on-missed: a capped re-practice task after missed questions",
            "re-test queue: deterministic intervals 1/3/7/14 days, family variant preferred",
            "review queue: due cards first (most at risk), then new cards under daily caps",
        ],
        "your_chapters": chapters,
    })))
}
'''

s = s.rstrip() + '\n' + addition
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('program.rs endpoints ok')

# --- review.rs: review debt (PLAN-03)
p = 'apps/api/src/routes/review.rs'
s = io.open(p, encoding='utf-8').read()
addition = '''
// ---- PLAN-03: review-debt recovery numbers (§12.4) ---------------------------

/// Honest debt report: what is due now, how much was actually cleared in the
/// last week, and how many days the backlog would take at that real rate.
/// No rate history means no projection — never a invented number.
pub async fn review_debt(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now();
    let due: i64 = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM cards
           WHERE user_id = $1 AND suspended = false
             AND (state->>'state')::int <> 0
             AND (state->>'due')::timestamptz <= $2"#,
        user.user_id,
        now
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    let recent: i64 = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*), 0) AS "n!" FROM review_events
           WHERE user_id = $1 AND reviewed_at >= $2"#,
        user.user_id,
        now - chrono::Duration::days(7)
    )
    .fetch_one(&state.pool)
    .await?
    .n;
    let backlog_days = if recent > 0 {
        Some((due as f64 / (recent as f64 / 7.0)).ceil() as i64)
    } else {
        None
    };
    Ok(Json(json!({
        "due_now": due,
        "completed_last_7_days": recent,
        "daily_rate": if recent > 0 { Some(recent as f64 / 7.0) } else { None },
        "projected_backlog_days": backlog_days,
        "note": if recent > 0 {
            "projection at your actual last-7-day rate"
        } else {
            "no review history yet \u2014 no projection without evidence"
        },
    })))
}
'''

s = s.rstrip() + '\n' + addition
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('review.rs debt ok')
