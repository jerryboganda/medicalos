use super::*;

#[tokio::test]
async fn readiness_requires_database_and_preserves_liveness() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());

    for path in ["/readyz", "/api/readyz"] {
        let (status, body) = call(app.clone(), request("GET", path, None, None)).await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
        assert_eq!(body["status"], "ok");
    }

    state.pool.close().await;

    for path in ["/readyz", "/api/readyz"] {
        let (status, body) = call(app.clone(), request("GET", path, None, None)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path}: {body}");
        assert_eq!(body, serde_json::json!({"status": "unavailable"}));
    }

    let (status, body) = call_text(app, request("GET", "/healthz", None, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}
