//! AI-03 mistake hypotheses and PROG-01 mastery heat-map. Both are honest,
//! computed views over recorded evidence: hypotheses are labelled as
//! hypotheses (never diagnoses, §8), and every number traces to attempts
//! or learner_concept_state — nothing is fabricated (§2.3).

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct HeatmapQuery {
    /// PROG-01 drill-down: only this system's chapters.
    pub system_id: Option<Uuid>,
    /// PROG-01 difficulty filter: accuracy computed over this difficulty only.
    pub difficulty: Option<String>,
    /// PROG-01 trend: recent-window accuracy (days) alongside the overall one.
    pub trend_days: Option<i64>,
}

/// GET /v1/me/mistake-hypotheses — AI-03: chapters where the learner keeps
/// missing questions, with evidence counts. A hypothesis needs ≥2 misses to
/// surface (single misses are noise, not patterns). The response says what
/// it is: a pattern to review, not a diagnosis.
pub async fn mistake_hypotheses(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT cn.id AS chapter_id, cn.name AS chapter_name,
                  COUNT(*) AS "misses!",
                  MAX(a.created_at) AS last_miss
           FROM attempts a
           JOIN curriculum_nodes cn ON cn.id = (
               SELECT chapter_id FROM question_versions qv
               WHERE qv.id = a.question_version_id)
           WHERE a.user_id = $1 AND a.correct = FALSE
           GROUP BY cn.id, cn.name
           HAVING COUNT(*) >= 2
           ORDER BY COUNT(*) DESC"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let hypotheses: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "chapter_id": r.chapter_id,
                "chapter_name": r.chapter_name,
                "misses": r.misses,
                "last_miss": r.last_miss,
                "kind": "hypothesis",
                "note": "Repeated misses here. Review before assuming a misconception.",
            })
        })
        .collect();
    Ok(Json(json!({ "hypotheses": hypotheses })))
}

/// GET /v1/me/heatmap — PROG-01: per-system rollup of the learner's chapter
/// mastery. Mastery bands come from config (mastery_bands), ability from
/// learner_concept_state (Elo, real evidence only). Chapters without
/// evidence are reported as unassessed — never guessed.
pub async fn mastery_heatmap(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(q): Query<HeatmapQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if let Some(d) = &q.difficulty {
        if !matches!(d.as_str(), "easy" | "medium" | "hard") {
            return Err(crate::error::ApiError::unprocessable(
                "invalid_difficulty",
                "difficulty must be easy, medium, or hard",
            ));
        }
    }
    if let Some(days) = q.trend_days {
        if !(1..=365).contains(&days) {
            return Err(crate::error::ApiError::unprocessable(
                "invalid_window",
                "trend_days must be 1-365",
            ));
        }
    }
    // Mastery bands: config override, else the §24 default quad-points.
    let bands: Vec<i64> = sqlx::query!(
        r#"SELECT value->0 AS "lo!", value->1 AS "hi!" FROM app_settings
           WHERE key = 'mastery_bands'"#
    )
    .fetch_optional(&state.pool)
    .await?
    .map(|r| vec![r.lo.as_i64().unwrap_or(1400), r.hi.as_i64().unwrap_or(1600)])
    .unwrap_or_else(|| vec![1400, 1600]);
    let (weak_at, strong_at) = (bands[0] as f32, bands[1] as f32);

    let rows = sqlx::query!(
        r#"SELECT system.id AS system_id, system.name AS system_name,
                  chapter.id AS chapter_id, chapter.name AS chapter_name,
                  lcs.ability AS "ability?", lcs.evidence_count AS "evidence_count?"
           FROM curriculum_nodes system
           JOIN curriculum_nodes chapter ON chapter.parent_id = system.id
           LEFT JOIN learner_concept_state lcs
             ON lcs.chapter_id = chapter.id AND lcs.user_id = $1
           WHERE system.kind = 'system'
             AND ($2::uuid IS NULL OR system.id = $2)
           ORDER BY system.display_order, chapter.display_order"#,
        user.user_id,
        q.system_id
    )
    .fetch_all(&state.pool)
    .await?;
    // Optional real-evidence overlays: accuracy restricted to one difficulty,
    // and a recent-window accuracy next to the overall one (trend direction).
    let overlays: std::collections::HashMap<Uuid, (i64, i64, Option<i64>)> =
        if q.difficulty.is_some() || q.trend_days.is_some() {
            let days = q.trend_days.unwrap_or(30).clamp(1, 365);
            let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
            let rows = sqlx::query!(
                r#"SELECT qv.chapter_id,
                      COUNT(*) FILTER (WHERE a.correct IS NOT NULL) AS "answered!",
                      COUNT(*) FILTER (WHERE a.correct = TRUE) AS "correct!",
                      COUNT(*) FILTER (WHERE a.correct = TRUE
                             AND a.created_at >= $2) AS "recent_correct!"
               FROM attempts a
               JOIN question_versions qv ON qv.id = a.question_version_id
               WHERE a.user_id = $1 AND a.assisted = FALSE
                 AND ($3::text IS NULL OR qv.difficulty = $3)
               GROUP BY qv.chapter_id"#,
                user.user_id,
                cutoff,
                q.difficulty
            )
            .fetch_all(&state.pool)
            .await?;
            rows.into_iter()
                .map(|r| {
                    (
                        r.chapter_id,
                        (r.answered, r.correct, Some(r.recent_correct)),
                    )
                })
                .collect()
        } else {
            std::collections::HashMap::new()
        };

    let mut systems: Vec<serde_json::Value> = Vec::new();
    for r in rows {
        let band = match (r.ability, r.evidence_count) {
            (Some(ability), Some(count)) if count > 0 => {
                if ability < weak_at {
                    "weak"
                } else if ability >= strong_at {
                    "strong"
                } else {
                    "developing"
                }
            }
            _ => "unassessed",
        };
        let mut entry = json!({
            "chapter_id": r.chapter_id,
            "chapter_name": r.chapter_name,
            "ability": r.ability,
            "evidence_count": r.evidence_count,
            "band": band,
        });
        if let Some((answered, correct, recent_correct)) = overlays.get(&r.chapter_id) {
            entry["filtered_accuracy"] = if *answered > 0 {
                json!(Some((*correct * 100 / *answered) as i64))
            } else {
                json!(None::<i64>)
            };
            if let Some(recent) = recent_correct {
                entry["recent_answered"] = json!(answered);
                entry["recent_correct"] = json!(recent);
            }
        }
        if let Some(sys) = systems
            .iter_mut()
            .find(|s| s["system_id"] == r.system_id.to_string())
        {
            sys["chapters"].as_array_mut().unwrap().push(entry);
        } else {
            systems.push(json!({
                "system_id": r.system_id,
                "system_name": r.system_name,
                "chapters": [entry],
            }));
        }
    }
    Ok(Json(
        json!({ "systems": systems, "bands": {"weak_below": weak_at, "strong_at": strong_at} }),
    ))
}
