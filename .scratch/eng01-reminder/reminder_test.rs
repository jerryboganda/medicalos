/// ENG-01 reminder tail: the hourly QOTD reminder job sends one factual
/// in-app notification per eligible learner per day — and only when the
/// learner opted into QOTD with an exam, has not answered today, keeps
/// plan reminders on, and sits outside their quiet hours. The engagement
/// kill switch silences everything, and the job re-enqueues itself hourly.
#[tokio::test]
async fn qotd_reminder_honours_preferences_quiet_hours_and_dedupes() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");

    async fn enable_qotd(app: &Router, token: &str, exam_id: Uuid) {
        let (status, body) = call(
            app.clone(),
            request(
                "PUT",
                "/v1/me/engagement/settings",
                Some(token),
                Some(serde_json::json!({"qotd_exam_id": exam_id})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    let token_a = register_and_login(app.clone()).await;
    enable_qotd(&app, &token_a, ids.exam_id).await;

    let token_b = register_and_login(app.clone()).await;
    enable_qotd(&app, &token_b, ids.exam_id).await;
    // b muted plan reminders entirely.
    let (status, body) = call(
        app.clone(),
        request(
            "PATCH",
            "/v1/me/notifications",
            Some(&token_b),
            Some(serde_json::json!({"plan_reminders": false})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let token_c = register_and_login(app.clone()).await;
    enable_qotd(&app, &token_c, ids.exam_id).await;
    // c sits inside a quiet window that covers the current UTC hour.
    let hour = i32::from(chrono::Utc::now().time().hour());
    let start = (hour + 23) % 24;
    let end = (hour + 2) % 24;
    let (status, body) = call(
        app.clone(),
        request(
            "PATCH",
            "/v1/me/notifications",
            Some(&token_c),
            Some(serde_json::json!({"quiet_hours_start": start, "quiet_hours_end": end})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let reminders_of = |inbox: &Value| -> Vec<Value> {
        inbox["notifications"]
            .as_array()
            .expect("notifications")
            .iter()
            .filter(|n| n["category"] == "plan" && n["deep_link"] == "/today")
            .cloned()
            .collect()
    };

    // One reminder pass: only a is reminded.
    api::routes::jobs::enqueue(&state.pool, "qotd_reminders", serde_json::json!({}), None)
        .await
        .expect("enqueue");
    let processed = api::routes::jobs::process_due_jobs(&state).await.expect("process");
    assert!(processed >= 1, "the reminder job ran");

    let (status, inbox_a) = call(
        app.clone(),
        request("GET", "/v1/me/notifications", Some(&token_a), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{inbox_a}");
    let reminders_a = reminders_of(&inbox_a);
    assert_eq!(reminders_a.len(), 1, "{inbox_a}");
    assert_eq!(reminders_a[0]["title"], "Question of the day", "{inbox_a}");
    assert!(
        reminders_a[0]["body"].as_str().unwrap().contains("available"),
        "copy stays factual: {inbox_a}"
    );

    let (status, inbox_b) = call(
        app.clone(),
        request("GET", "/v1/me/notifications", Some(&token_b), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        reminders_of(&inbox_b).is_empty(),
        "muted learner is never reminded: {inbox_b}"
    );

    let (status, inbox_c) = call(
        app.clone(),
        request("GET", "/v1/me/notifications", Some(&token_c), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        reminders_of(&inbox_c).is_empty(),
        "quiet hours are honoured: {inbox_c}"
    );

    // Same-day dedupe: a second pass sends nothing new.
    api::routes::jobs::enqueue(&state.pool, "qotd_reminders", serde_json::json!({}), None)
        .await
        .expect("enqueue");
    api::routes::jobs::process_due_jobs(&state).await.expect("process");
    let (status, inbox_a2) = call(
        app.clone(),
        request("GET", "/v1/me/notifications", Some(&token_a), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reminders_of(&inbox_a2).len(), 1, "one per day: {inbox_a2}");

    // The job re-enqueued itself for the next hour.
    let future_jobs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM event_jobs
         WHERE kind = 'qotd_reminders' AND status = 'pending'
           AND run_after > now() + interval '55 minutes'",
    )
    .fetch_one(&state.pool)
    .await
    .expect("count pending");
    assert!(future_jobs >= 1, "self-rescheduled hourly job exists");

    // A learner who already answered today is never reminded.
    let (status, qotd) = call(
        app.clone(),
        request("GET", "/v1/qotd", Some(&token_a), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{qotd}");
    let qv = qotd["question_version_id"].as_str().expect("question version");
    let (status, ans) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/qotd/answers",
            Some(&token_a),
            Some(serde_json::json!({
                "question_version_id": qv, "chosen_index": 0, "elapsed_ms": 4200
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ans}");
    api::routes::jobs::enqueue(&state.pool, "qotd_reminders", serde_json::json!({}), None)
        .await
        .expect("enqueue");
    api::routes::jobs::process_due_jobs(&state).await.expect("process");
    let (status, inbox_a3) = call(
        app.clone(),
        request("GET", "/v1/me/notifications", Some(&token_a), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        reminders_of(&inbox_a3).len(),
        1,
        "answered learners get no reminder: {inbox_a3}"
    );

    // The global kill switch silences every send.
    let (status, _) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/admin/flags",
            Some(&token_a),
            Some(serde_json::json!({"key": "engagement_mechanics", "value": false})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "flag set");
    let token_d = register_and_login(app.clone()).await;
    enable_qotd(&app, &token_d, ids.exam_id).await;
    api::routes::jobs::enqueue(&state.pool, "qotd_reminders", serde_json::json!({}), None)
        .await
        .expect("enqueue");
    api::routes::jobs::process_due_jobs(&state).await.expect("process");
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE category = 'plan' AND deep_link = '/today'",
    )
    .fetch_one(&state.pool)
    .await
    .expect("count");
    assert_eq!(total, 1, "kill switch silences every send");
}
