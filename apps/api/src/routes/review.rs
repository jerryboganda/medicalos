// ---- PLAN-03: review-debt recovery numbers -----------------------------------

/// Honest debt report: what is due now, how much was actually cleared in the
/// last week, and how many days the backlog would take at that real rate.
/// No rate history means no projection.
pub async fn review_debt(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now();
    let week_ago = now - chrono::Duration::days(7);
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
        week_ago
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
            "no review history yet - no projection without evidence"
        },
    })))
}
