use super::*;

#[tokio::test]
async fn retest_refuses_correctness_claim_without_submitted_answer_evidence() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let learner = register_and_login(app.clone()).await;
    let (status, result) = call(
        app,
        request(
            "POST",
            "/v1/me/retests/result",
            Some(&learner),
            Some(serde_json::json!({
                "question_version_id": ids.question_versions[0],
                "correct": true,
                "idempotency_key": "forged-retest-pass"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
}
