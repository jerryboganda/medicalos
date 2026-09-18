//! Seam-level integration tests (tdd skill): everything goes through the HTTP
//! API, never through internals. Tests serialize on a shared schema-wiping
//! fixture because the CI database is a single ephemeral PostgreSQL.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::Value;
use sqlx::Row;
use tower::ServiceExt; // oneshot
use uuid::Uuid;

use api::{router, schema, seed, state::AppState};

static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> Arc<AppState> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/medos_ci".into());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect to test database");
    schema::apply_down(&pool).await.expect("apply down");
    schema::apply_up(&pool).await.expect("apply up");
    Arc::new(AppState { pool })
}

async fn call(app: Router, req: Request<Body>) -> (StatusCode, Value) {
    let uri = req.uri().clone();
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let bytes = http_body_util::BodyExt::collect(resp.into_body())
        .await
        .expect("body")
        .to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        panic!(
            "non-json response to {uri} (status {status}): {e}: {}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (status, v)
}

fn request(method: &str, uri: &str, token: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    builder
        .body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))
        .expect("request")
}

async fn register_and_login(app: Router) -> String {
    let email = format!("learner-{}@example.test", Uuid::new_v4());
    let (_, v) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/register",
            None,
            Some(serde_json::json!({"email": email, "password": "correct horse"})),
        ),
    )
    .await;
    assert!(v["user_id"].as_str().is_some(), "register: {v}");
    let (status, v) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/login",
            None,
            Some(serde_json::json!({"email": email, "password": "correct horse"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login: {v}");
    v["token"].as_str().expect("token").to_string()
}

#[tokio::test]
async fn auth_register_login_and_reject_bad_credentials() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let email = format!("auth-{}@example.test", Uuid::new_v4());

    let (status, v) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/register",
            None,
            Some(serde_json::json!({"email": email, "password": "longenough"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{v}");

    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/register",
            None,
            Some(serde_json::json!({"email": email, "password": "longenough"})),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "duplicate email must conflict"
    );

    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/login",
            None,
            Some(serde_json::json!({"email": email, "password": "wrong password"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "wrong password rejected");

    let (status, _) = call(app.clone(), request("GET", "/v1/me/today", None, None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "missing token rejected");

    let (status, v) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/login",
            None,
            Some(serde_json::json!({"email": email, "password": "longenough"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(v["token"].as_str().is_some());
}

#[tokio::test]
async fn full_loop_cold_start_answer_submit_revision_undo() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // Cold start: a modest plan exists before any evidence (AI-02).
    let (status, today) = call(
        app.clone(),
        request("GET", "/v1/me/today", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{today}");
    assert_eq!(today["version"], 1);
    assert_eq!(today["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(
        today["learner"].as_array().unwrap().len(),
        0,
        "no fake analytics"
    );

    let chapter1 = today["tasks"][0]["chapter_id"].as_str().map(str::to_string);
    let chapter1 = Uuid::parse_str(&chapter1.expect("cold-start task has chapter")).unwrap();

    // Tutor session over the cold-start chapter.
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": chapter1, "question_count": 2
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();
    assert_eq!(session["items"].as_array().unwrap().len(), 2);
    // No answer keys or rationales before answering (§11.3).
    assert!(session["items"][0]["options"][0].get("rationale").is_none());

    // Answer item 0 correctly, with an idempotency key.
    let answer_req = |key: &str, chosen: i16| {
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 0, "chosen_index": chosen,
                                   "idempotency_key": key})),
        )
    };
    // Find the correct index by trying: the fixture option 0 of each question
    // is not guaranteed correct; use the API's own feedback to detect it is
    // well-formed, then assert on structure rather than key correctness.
    let (status, ans) = call(app.clone(), answer_req("key-1", 0)).await;
    assert_eq!(status, StatusCode::OK, "{ans}");
    assert_eq!(ans["already_recorded"], false);
    assert!(ans["correct"].is_boolean());
    assert!(ans["correct_index"].is_i64());
    assert!(ans["options"].as_array().unwrap()[0]["rationale"].is_string());
    assert!(ans["key_learning_point"].is_string());

    // Replay the same key: same answer, no duplicate attempt.
    let (status, replay) = call(app.clone(), answer_req("key-1", 0)).await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["already_recorded"], true);
    assert_eq!(replay["correct"], ans["correct"]);
    let attempts = sqlx::query("SELECT COUNT(*) AS n FROM attempts WHERE session_id = $1")
        .bind(sid)
        .fetch_one(&state.pool)
        .await
        .expect("count");
    assert_eq!(
        attempts.get::<i64, _>("n"),
        1,
        "idempotent replay must not duplicate the attempt (EX-04)"
    );

    // Different key on the same item: first answer wins.
    let (status, _) = call(app.clone(), answer_req("key-1-b", 1)).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Skip item 1 (no chosen_index — a skip is not an attempt at knowledge).
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 1, "idempotency_key": "key-2"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Submit: 1 answered (right or wrong) + 1 skipped.
    let (status, result) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/submit"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["total"], 2);
    assert_eq!(result["skipped"], 1);
    assert_eq!(
        result["correct"].as_i64().unwrap()
            + result["incorrect"].as_i64().unwrap()
            + result["skipped"].as_i64().unwrap(),
        2
    );

    // Double submit is rejected.
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/submit"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Plan: cold-start task done; a justified, persisted revision exists.
    let (status, today) = call(
        app.clone(),
        request("GET", "/v1/me/today", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{today}");
    assert_eq!(today["tasks"][0]["status"], "done");
    let revisions = today["revisions"].as_array().unwrap();
    assert_eq!(
        revisions.len(),
        1,
        "revision created after missed answers: {today}"
    );
    assert_eq!(revisions[0]["automatic"], true);
    assert_eq!(revisions[0]["reason_code"], "incorrect_answers");
    assert!(!revisions[0]["undone"].as_bool().unwrap());
    let rid: Uuid = revisions[0]["id"].as_str().unwrap().parse().unwrap();

    // Learner state: honest sparse-data behavior (AI-02). One answered item,
    // one skip (skips are not evidence).
    let learner = today["learner"].as_array().unwrap();
    assert_eq!(learner.len(), 1);
    assert!(
        learner[0]["mastery_index"].is_null(),
        "no mastery under 10 attempts"
    );
    assert_eq!(learner[0]["evidence_level"], "low_evidence");
    assert_eq!(learner[0]["independent_count"], 1);

    // Undo the revision (AI-07).
    let (status, undone) = call(
        app.clone(),
        request(
            "POST",
            &format!(
                "/v1/plans/{}/revisions/{}/undo",
                today["plan_id"].as_str().unwrap(),
                rid
            ),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{undone}");
    assert_eq!(undone["plan_version"], 3, "undo creates version 3");

    let (_, today) = call(
        app.clone(),
        request("GET", "/v1/me/today", Some(&token), None),
    )
    .await;
    let revisions = today["revisions"].as_array().unwrap();
    assert_eq!(revisions[0]["undone"], true, "revision marked undone");
    // Cold-start task remains; the revision's added task is gone (only one
    // task in the latest version).
    assert_eq!(today["tasks"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn revision_pool_is_exactly_wrong_and_skipped() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(
                serde_json::json!({"preset": "tutor", "chapter_id": ids.chapter2,
                                   "question_count": 2}),
            ),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();

    // Answer both items with option A. Fixture keys are fixed (q3 -> A, q4 ->
    // B), so exactly one answer is wrong regardless of item order — the
    // revision pool is deterministic: 1 wrong + 0 skipped.
    let (_, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 0, "chosen_index": 0,
                                   "idempotency_key": "r-key-1"})),
        ),
    )
    .await;
    let (_, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 1, "chosen_index": 0,
                                   "idempotency_key": "r-key-2"})),
        ),
    )
    .await;
    let (status, result) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/submit"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["incorrect"], 1);
    assert_eq!(result["skipped"], 0);

    // Revision pool = wrong + skipped only.
    let (status, revision) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({"preset": "revision", "source_session_id": sid})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{revision}");
    assert_eq!(
        revision["items"].as_array().unwrap().len(),
        1,
        "only missed questions re-appear"
    );

    // An unanswered (open) session cannot be a revision source.
    let (status, open_session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(
                serde_json::json!({"preset": "tutor", "chapter_id": ids.chapter1,
                                   "question_count": 1}),
            ),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{open_session}");
    let open_sid: Uuid = open_session["session_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({"preset": "revision", "source_session_id": open_sid})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn timed_session_expires_server_side() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // Create a timed session at the floor (default floor is 30s in tests).
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(
                serde_json::json!({"preset": "timed", "chapter_id": ids.chapter1,
                                   "question_count": 2, "time_limit_seconds": 30}),
            ),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    assert!(
        session["deadline"].is_string(),
        "server issues the deadline"
    );
    assert!(session["server_now"].is_string());
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();

    // A timed session without time_limit_seconds is rejected outright.
    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(
                serde_json::json!({"preset": "timed", "chapter_id": ids.chapter1,
                                   "question_count": 2}),
            ),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "time_limit_required");

    // Force expiry server-side (the client cannot extend its timer: EX-08).
    sqlx::query("UPDATE practice_sessions SET deadline = now() - interval '1 second'")
        .bind(sid)
        .execute(&state.pool)
        .await
        .expect("expire session");

    // Answers are rejected once the server deadline has passed.
    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 0, "chosen_index": 0,
                                   "idempotency_key": "expired-key"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"]["code"], "session_expired");

    // Submission after expiry still works — auto-submit semantics (§11.6).
    let (status, result) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/submit"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["skipped"], 2, "unanswered items count as skipped");
}

#[tokio::test]
async fn free_daily_allowance_blocks_new_sessions_with_details() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // Default allowance is 10 questions; each session holds 2. Answer 10
    // across five sessions, then the sixth session must be refused with an
    // honest entitlement payload (COM-01).
    for n in 0..5 {
        let (status, session) = call(
            app.clone(),
            request(
                "POST",
                "/v1/practice/sessions",
                Some(&token),
                Some(
                    serde_json::json!({"preset": "tutor", "chapter_id": ids.chapter2,
                                       "question_count": 2}),
                ),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "session {n}: {session}");
        let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();
        for idx in 0..2 {
            let (status, _) = call(
                app.clone(),
                request(
                    "POST",
                    &format!("/v1/practice/sessions/{sid}/answers"),
                    Some(&token),
                    Some(serde_json::json!({"item_index": idx, "chosen_index": 0,
                                           "idempotency_key": format!("allow-{n}-{idx}")})),
                ),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }
    }

    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(
                serde_json::json!({"preset": "tutor", "chapter_id": ids.chapter2,
                                   "question_count": 2}),
            ),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"]["code"], "free_allowance_reached");
    assert_eq!(body["error"]["details"]["allowance"]["limit"], 10);
    assert_eq!(body["error"]["details"]["allowance"]["used"], 10);
}

#[tokio::test]
async fn migration_up_down_up_is_reversible() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    schema::apply_down(&state.pool).await.expect("down");
    schema::apply_up(&state.pool).await.expect("up");
    schema::apply_down(&state.pool).await.expect("down again");
    schema::apply_up(&state.pool).await.expect("up again");
    let rows = sqlx::query(
        "SELECT COUNT(*) AS n FROM information_schema.tables WHERE table_name = 'plans'",
    )
    .fetch_one(&state.pool)
    .await
    .expect("table check");
    assert_eq!(
        rows.get::<i64, _>("n"),
        1,
        "schema present after up-down-up"
    );
}
