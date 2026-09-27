//! AI-03 mistake hypotheses and PROG-01 mastery heat-map. Both are honest,
//! computed views over recorded evidence: hypotheses are labelled as
//! hypotheses (never diagnoses, §8), and every number traces to attempts
//! or learner_concept_state — nothing is fabricated (§2.3).

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "insights/HeatmapQuery.ts",
        rename = "HeatmapQuery"
    )
)]
pub struct HeatmapQuery {
    /// PROG-01 drill-down: only this system's chapters.
    pub system_id: Option<Uuid>,
    /// PROG-01 difficulty filter: accuracy computed over this difficulty only.
    pub difficulty: Option<String>,
    /// PROG-01 trend: recent-window accuracy (days) alongside the overall one.
    pub trend_days: Option<i64>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "insights/MasteryHeatmapBand.ts",
        rename = "MasteryHeatmapBand"
    )
)]
pub enum MasteryHeatmapBand {
    Weak,
    Developing,
    Strong,
    Unassessed,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "insights/MasteryHeatmapChapter.ts",
        rename = "MasteryHeatmapChapter"
    )
)]
pub struct MasteryHeatmapChapter {
    pub chapter_id: Uuid,
    pub chapter_name: String,
    pub ability: Option<f32>,
    pub evidence_count: Option<i32>,
    pub band: MasteryHeatmapBand,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtered_accuracy: Option<Option<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_answered: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_correct: Option<i64>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "insights/MasteryHeatmapSystem.ts",
        rename = "MasteryHeatmapSystem"
    )
)]
pub struct MasteryHeatmapSystem {
    pub system_id: Uuid,
    pub system_name: String,
    pub chapters: Vec<MasteryHeatmapChapter>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "insights/MasteryHeatmapBands.ts",
        rename = "MasteryHeatmapBands"
    )
)]
pub struct MasteryHeatmapBands {
    pub weak_below: f32,
    pub strong_at: f32,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "insights/MasteryHeatmapResponse.ts",
        rename = "MasteryHeatmapResponse"
    )
)]
pub struct MasteryHeatmapResponse {
    pub systems: Vec<MasteryHeatmapSystem>,
    pub bands: MasteryHeatmapBands,
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
    // COM-01: the difficulty/trend drill-down is the chapter-analytics
    // premium view. The trigger fires from this entitlement check — free
    // accounts get free_analytics_drills drill-downs per day.
    let is_drill = q.difficulty.is_some() || q.trend_days.is_some();
    if is_drill {
        let day = chrono::Utc::now().date_naive();
        let used = sqlx::query!(
            r#"SELECT count AS "count!" FROM entitlement_usage
               WHERE user_id = $1 AND key = 'analytics_drill' AND day = $2"#,
            user.user_id,
            day
        )
        .fetch_optional(&state.pool)
        .await?
        .map(|r| r.count)
        .unwrap_or(0);
        if i64::from(used) >= state.free_analytics_drills {
            return Err(crate::error::ApiError::forbidden_with_details(
                "upgrade_required",
                "chapter-analytics drill-downs beyond the free daily allowance need an upgrade",
                serde_json::json!({
                    "trigger": "chapter_analytics",
                    "entitlement": {
                        "free_daily_drills": state.free_analytics_drills,
                        "used_today": used
                    }
                }),
            ));
        }
        sqlx::query!(
            r#"INSERT INTO entitlement_usage (user_id, key, day, count)
               VALUES ($1, 'analytics_drill', $2, 1)
               ON CONFLICT (user_id, key, day) DO UPDATE
                 SET count = entitlement_usage.count + 1"#,
            user.user_id,
            day
        )
        .execute(&state.pool)
        .await?;
    }
    // Mastery bands: config override, else the §24 default quad-points.
    let mut bands =
        crate::routes::settings::current_i64_list(&state.pool, "mastery_bands", &[1400, 1600])
            .await?;
    if bands.len() != 2
        || !(0..=3000).contains(&bands[0])
        || bands[0] >= bands[1]
        || bands[1] > 3000
    {
        bands = vec![1400, 1600];
    }
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

    let mut systems: Vec<MasteryHeatmapSystem> = Vec::new();
    for r in rows {
        let band = match (r.ability, r.evidence_count) {
            (Some(ability), Some(count)) if count > 0 => {
                if ability < weak_at {
                    MasteryHeatmapBand::Weak
                } else if ability >= strong_at {
                    MasteryHeatmapBand::Strong
                } else {
                    MasteryHeatmapBand::Developing
                }
            }
            _ => MasteryHeatmapBand::Unassessed,
        };
        let (filtered_accuracy, recent_answered, recent_correct) =
            if let Some((answered, correct, recent_correct)) = overlays.get(&r.chapter_id) {
                (
                    Some(if *answered > 0 {
                        Some(*correct * 100 / *answered)
                    } else {
                        None
                    }),
                    recent_correct.map(|_| *answered),
                    recent_correct.as_ref().copied(),
                )
            } else {
                (None, None, None)
            };
        let entry = MasteryHeatmapChapter {
            chapter_id: r.chapter_id,
            chapter_name: r.chapter_name,
            ability: r.ability,
            evidence_count: r.evidence_count,
            band,
            filtered_accuracy,
            recent_answered,
            recent_correct,
        };
        if let Some(sys) = systems.iter_mut().find(|s| s.system_id == r.system_id) {
            sys.chapters.push(entry);
        } else {
            systems.push(MasteryHeatmapSystem {
                system_id: r.system_id,
                system_name: r.system_name,
                chapters: vec![entry],
            });
        }
    }
    Ok(Json(MasteryHeatmapResponse {
        systems,
        bands: MasteryHeatmapBands {
            weak_below: weak_at,
            strong_at,
        },
    }))
}

// ---- CORE-05: per-chapter accuracy trend (§8.8) -------------------------------

#[derive(Deserialize)]
pub struct TrendsQuery {
    pub days: Option<i64>,
    pub chapter_id: Option<Uuid>,
}

/// Weekly accuracy buckets per chapter from real, non-assisted attempts.
/// Weeks with no evidence are absent — the chart shows gaps as gaps.
pub async fn accuracy_trends(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(q): Query<TrendsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let days = q.days.unwrap_or(30).clamp(7, 365);
    let since = chrono::Utc::now() - chrono::Duration::days(days);
    let rows = sqlx::query!(
        r#"SELECT qv.chapter_id, c.name AS chapter_name,
                  date_trunc('week', a.created_at)::date AS week_start,
                  COUNT(*) FILTER (WHERE a.correct IS NOT NULL) AS "answered!",
                  COUNT(*) FILTER (WHERE a.correct = TRUE) AS "correct!"
           FROM attempts a
           JOIN question_versions qv ON qv.id = a.question_version_id
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE a.user_id = $1 AND a.assisted = FALSE
             AND a.correct IS NOT NULL AND a.created_at >= $2
             AND ($3::uuid IS NULL OR qv.chapter_id = $3)
           GROUP BY qv.chapter_id, c.name, week_start
           ORDER BY c.name, week_start"#,
        user.user_id,
        since,
        q.chapter_id
    )
    .fetch_all(&state.pool)
    .await?;
    let mut chapters: Vec<serde_json::Value> = Vec::new();
    for r in rows {
        let accuracy = if r.answered > 0 {
            Some(r.correct * 100 / r.answered)
        } else {
            None
        };
        let bucket = json!({
            "week_start": r.week_start,
            "answered": r.answered,
            "accuracy": accuracy,
        });
        if let Some(ch) = chapters
            .iter_mut()
            .find(|c| c["chapter_id"] == r.chapter_id.to_string())
        {
            ch["buckets"].as_array_mut().unwrap().push(bucket);
        } else {
            chapters.push(json!({
                "chapter_id": r.chapter_id,
                "chapter_name": r.chapter_name,
                "buckets": [bucket],
            }));
        }
    }
    Ok(Json(json!({
        "window_days": days,
        "chapters": chapters,
    })))
}
