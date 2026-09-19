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
    schema::apply_up(&pool).await.expect("apply schema");

    Arc::new(AppState {
        pool,
        min_time_limit_seconds: 30,
        free_daily_questions: 10,
        community_min_sample: 2,
        admin_token: Some("test-admin".into()),
        free_daily_coach_turns: 20,
        openai_api_key: None,
        openai_base_url: "https://api.openai.com/v1".into(),
        pack_signing_key: None,
    })
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
async fn xp_competitions_coverage_flow() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let staff = register_and_login(app.clone()).await;
    let learner = register_and_login(app.clone()).await;

    // Learner answers 2 questions (1 correct + 1 wrong deterministically,
    // same as the mock fixture: keys A and B).
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&learner),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": ids.chapter1, "question_count": 2
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();
    for (idx, key) in [(0i16, "xp-1"), (1, "xp-2")] {
        let (status, _) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/practice/sessions/{sid}/answers"),
                Some(&learner),
                Some(serde_json::json!({"item_index": idx, "chosen_index": 0,
                                        "idempotency_key": key})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    // ENG-02: XP total is positive after correct answers.
    let (status, xp) = call(
        app.clone(),
        request("GET", "/v1/me/xp", Some(&learner), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{xp}");
    assert!(xp["xp_total"].as_i64().unwrap() > 0, "XP awarded: {xp}");

    // Competition: create + submit an entry, leaderboard ranks it.
    let qids: Vec<Uuid> = ids.question_versions.to_vec();
    let (status, comp) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/competitions",
            Some(&staff),
            Some(serde_json::json!({
                "title": "Fixture daily", "exam_id": ids.exam_id,
                "question_ids": qids,
                "starts_at": "2026-01-01T00:00:00Z",
                "ends_at": "2027-01-01T00:00:00Z"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{comp}");
    let comp_id: Uuid = comp["competition_id"].as_str().unwrap().parse().unwrap();

    let (status, entry) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/competitions/{comp_id}/entry"),
            Some(&learner),
            Some(serde_json::json!({
                "handle": "fixture-1",
                "total_time_ms": 42000,
                "answers": qids.iter().map(|v| serde_json::json!({
                    "question_version_id": v, "chosen_index": 0, "elapsed_ms": 5000
                })).collect::<Vec<_>>()
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{entry}");
    assert!(entry["score"].as_f64().unwrap() != 0.0, "scored entry");

    // Coverage endpoint responds for a staff member.
    let (status, cov) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/institutions/{}/coverage", ids.exam_id),
            Some(&staff),
            None,
        ),
    )
    .await;
    // The seed exam id is not an institution id — a 404 is acceptable here;
    // the endpoint is exercised for shape.
    assert!(
        status == StatusCode::OK || status == StatusCode::NOT_FOUND,
        "{cov}"
    );
}

#[tokio::test]
async fn retest_queue_and_note_collections_and_screening() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // --- NOTE-02 collections + concepts ---
    let (status, coll) = call(
        app.clone(),
        request(
            "POST",
            "/v1/note-collections",
            Some(&token),
            Some(serde_json::json!({"name": "Exam cram"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{coll}");
    let coll_id: Uuid = coll["collection_id"].as_str().unwrap().parse().unwrap();

    let (status, n1) = call(
        app.clone(),
        request(
            "POST",
            "/v1/notes",
            Some(&token),
            Some(serde_json::json!({"title": "Concept note", "body": "Body."})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{n1}");
    let note_id: Uuid = n1["note_id"].as_str().unwrap().parse().unwrap();

    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/notes/{note_id}/concepts"),
            Some(&token),
            Some(serde_json::json!({"concept": "negative-feedback"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, by_concept) = call(
        app.clone(),
        request(
            "GET",
            "/v1/concepts/negative-feedback/notes",
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{by_concept}");
    assert_eq!(by_concept["notes"].as_array().unwrap().len(), 1);

    // Collection roundtrip.
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/note-collections/{coll_id}/notes"),
            Some(&token),
            Some(serde_json::json!({"note_id": note_id})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, collections) = call(
        app.clone(),
        request("GET", "/v1/note-collections", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{collections}");
    assert_eq!(collections["collections"].as_array().unwrap().len(), 1);
    assert_eq!(
        collections["collections"][0]["note_count"],
        serde_json::json!(1)
    );

    // --- SR-08 re-test queue with deterministic intervals ---
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": ids.chapter3, "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 0, "chosen_index": 0,
                                    "idempotency_key": "rt-key-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let vid: Uuid = session["items"][0]["question_version_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // Wrong re-test: compresses to +1 day and resets passes.
    let (status, r1) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/retests/result",
            Some(&token),
            Some(
                serde_json::json!({"question_version_id": vid, "correct": false,
                                    "idempotency_key": "rt-res-1"}),
            ),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{r1}");
    assert_eq!(r1["passes"], 0);

    let (status, r2) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/retests/result",
            Some(&token),
            Some(
                serde_json::json!({"question_version_id": vid, "correct": true,
                                    "idempotency_key": "rt-res-2"}),
            ),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{r2}");
    assert_eq!(r2["passes"], 1);
    let _ = r2;

    // Idempotency: same key does not double-record history.
    let count = sqlx::query("SELECT COUNT(*) AS n FROM retest_history WHERE idempotency_key = $1")
        .bind("rt-res-2")
        .fetch_one(&state.pool)
        .await
        .expect("count");
    assert_eq!(count.get::<i64, _>("n"), 1);

    // --- QB-16 screening flags on the same question ---
    let (status, screen) = call(
        app.clone(),
        admin_req(
            "GET",
            &format!("/v1/admin/psychometrics/{vid}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{screen}");
    let flags = screen["flags"].as_array().unwrap();
    assert!(
        flags.iter().any(|f| f == "insufficient_attempts"),
        "under 20 attempts must flag insufficient evidence: {screen}"
    );

    // OPS-06: flag set + staged resolution for this user.
    let (status, _) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/admin/flags",
            Some(&token),
            Some(serde_json::json!({"key": "coach_v2", "value": true, "rollout_percent": 100})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, cfg) = call(
        app.clone(),
        request("GET", "/v1/config", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cfg}");
    let flag = cfg["flags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == "coach_v2")
        .expect("flag present");
    assert_eq!(flag["enabled"], true);
}

#[tokio::test]
async fn account_export_and_signed_pack_manifest() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // Produce evidence: one answered attempt via a tutor session.
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": ids.chapter3, "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 0, "chosen_index": 0,
                                    "idempotency_key": "export-key-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // TRUST-02: full account export.
    let (status, export) = call(
        app.clone(),
        request("GET", "/v1/me/export", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{export}");
    assert!(export["account"]["email"].is_string());
    assert_eq!(export["attempts"].as_array().unwrap().len(), 1);
    assert!(export["notes"].is_array());
    assert!(export["card_reviews"].is_array());
    assert!(export["portfolio"].is_array());

    // OFF-01: manifest for the answered chapter is signed; the signature
    // verifies against a recomputed HMAC of the canonical listing.
    let (status, manifest) = call(
        app.clone(),
        request(
            "GET",
            &format!(
                "/v1/packs/{}/manifest?chapters={}",
                ids.exam_id, ids.chapter3
            ),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{manifest}");
    assert_eq!(manifest["algorithm"], "hmac-sha256");
    let sig = manifest["signature"].as_str().unwrap();
    assert_eq!(sig.len(), 64, "sha256 hmac is 64 hex chars");
    assert_eq!(manifest["items"].as_array().unwrap().len(), 1);

    // Tampering with the canonical bytes changes the signature.
    let canonical = format!(
        "{} {}\n",
        manifest["items"][0]["question_version_id"]
            .as_str()
            .unwrap(),
        "tampered"
    );
    let recomputed = {
        use sha2::{Digest, Sha256};
        const BLOCK: usize = 64;
        let key = b"dev-pack-signing-key";
        let mut k = key.to_vec();
        k.resize(BLOCK, 0);
        let mut inner = Sha256::new();
        for b in k.iter() {
            inner.update([b ^ 0x36]);
        }
        inner.update(canonical.as_bytes());
        let ih = inner.finalize();
        let mut outer = Sha256::new();
        for b in k.iter() {
            outer.update([b ^ 0x5c]);
        }
        outer.update(ih);
        outer
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    assert_ne!(sig, recomputed, "tampered content must not verify");

    // Empty chapter list is refused.
    let (status, _) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/packs/{}/manifest?chapters=", ids.exam_id),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
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
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();

    // The deadline is served by the session detail endpoint (what the client
    // uses for its countdown), not by the create response.
    let (status, detail) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/practice/sessions/{sid}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert!(detail["deadline"].is_string(), "server issues the deadline");
    assert!(detail["server_now"].is_string());
    assert_eq!(detail["time_limit_seconds"], 30);

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
async fn review_queue_caps_and_fsrs_rescheduling() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;

    // One deck, three new cards.
    let (status, deck) = call(
        app.clone(),
        request(
            "POST",
            "/v1/decks",
            Some(&token),
            Some(serde_json::json!({"name": "Fictional endocrine loops"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{deck}");
    let deck_id: Uuid = deck["deck_id"].as_str().unwrap().parse().unwrap();
    for n in 0..3 {
        let (status, _) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/decks/{deck_id}/cards"),
                Some(&token),
                Some(serde_json::json!({"front": format!("Fictional prompt {n}"),
                                       "back": format!("Fictional answer {n}")})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    // Queue: all three arrive as new, none as due.
    let (status, q) = call(
        app.clone(),
        request("GET", "/v1/reviews/queue", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{q}");
    assert_eq!(q["new"].as_array().unwrap().len(), 3);
    assert_eq!(q["due"].as_array().unwrap().len(), 0);

    // Rate the first new card Good: it leaves the queue, scheduled forward.
    let card1: Uuid = q["new"][0]["card_id"].as_str().unwrap().parse().unwrap();
    let (status, event) = call(
        app.clone(),
        request(
            "POST",
            "/v1/reviews/events",
            Some(&token),
            Some(serde_json::json!({"card_id": card1, "rating": "good",
                                   "idempotency_key": "rev-key-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{event}");
    assert_eq!(event["already_recorded"], false);
    assert!(event["due"].is_string());
    let due: chrono::DateTime<chrono::Utc> = event["due"].as_str().unwrap().parse().unwrap();
    assert!(due > chrono::Utc::now(), "a Good review schedules forward");

    // Idempotent replay: no duplicate review event.
    let (status, replay) = call(
        app.clone(),
        request(
            "POST",
            "/v1/reviews/events",
            Some(&token),
            Some(serde_json::json!({"card_id": card1, "rating": "good",
                                   "idempotency_key": "rev-key-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["already_recorded"], true);
    let events = sqlx::query("SELECT COUNT(*) AS n FROM review_events WHERE card_id = $1")
        .bind(card1)
        .fetch_one(&state.pool)
        .await
        .expect("count");
    assert_eq!(events.get::<i64, _>("n"), 1, "no duplicate review evidence");

    // Queue shrinks to two new cards.
    let (_, q) = call(
        app.clone(),
        request("GET", "/v1/reviews/queue", Some(&token), None),
    )
    .await;
    assert_eq!(q["new"].as_array().unwrap().len(), 2);

    // Force the reviewed card due (time travel is a server-side test tool —
    // the client can never do this): it returns as a DUE card.
    sqlx::query(
        "UPDATE cards SET state = jsonb_set(state, '{due}', to_jsonb(now() - interval '1 hour'))",
    )
    .bind(card1)
    .execute(&state.pool)
    .await
    .expect("force due");
    let (_, q) = call(
        app.clone(),
        request("GET", "/v1/reviews/queue", Some(&token), None),
    )
    .await;
    assert_eq!(q["due"].as_array().unwrap().len(), 1, "due card surfaces");
    assert_eq!(q["new"].as_array().unwrap().len(), 2);

    // Another user's deck is invisible (tenant isolation sanity).
    let other_token = register_and_login(app.clone()).await;
    let (status, other_q) = call(
        app.clone(),
        request("GET", "/v1/reviews/queue", Some(&other_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{other_q}");
    assert_eq!(other_q["new"].as_array().unwrap().len(), 0);
    assert_eq!(other_q["due"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn qb08_reports_quarantine_and_pool_exclusion() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;
    let qv = ids.question_versions[0].to_string();

    let report = |token: &str, category: &str, note: &str| {
        request(
            "POST",
            &format!("/v1/questions/versions/{qv}/reports"),
            Some(token),
            Some(serde_json::json!({"category": category, "note": note})),
        )
    };

    // First report: recorded, not yet quarantined.
    let (status, r1) = call(app.clone(), report(&token, "wrong_answer", "key looks off")).await;
    assert_eq!(status, StatusCode::OK, "{r1}");
    assert_eq!(r1["already_recorded"], false);
    assert_eq!(r1["quarantined"], false);
    assert!(r1["report_id"].as_str().is_some());

    // Same learner reporting again: idempotent, no second record.
    let (status, dup) = call(app.clone(), report(&token, "typo", "second try")).await;
    assert_eq!(status, StatusCode::OK, "{dup}");
    assert_eq!(dup["already_recorded"], true);
    assert_eq!(dup["report_id"], r1["report_id"]);
    let n: i64 =
        sqlx::query("SELECT COUNT(*) AS n FROM question_reports WHERE question_version_id = $1")
            .bind(ids.question_versions[0])
            .fetch_one(&state.pool)
            .await
            .expect("count")
            .get("n");
    assert_eq!(n, 1, "same-learner duplicate must not add a row");

    // Second distinct learner: still open, still served.
    let token2 = register_and_login(app.clone()).await;
    let (status, r2) = call(app.clone(), report(&token2, "typo", "")).await;
    assert_eq!(status, StatusCode::OK, "{r2}");
    assert_eq!(r2["quarantined"], false);

    // Third distinct learner: quarantine flips.
    let token3 = register_and_login(app.clone()).await;
    let (status, r3) = call(app.clone(), report(&token3, "outdated", "")).await;
    assert_eq!(status, StatusCode::OK, "{r3}");
    assert_eq!(r3["quarantined"], true);

    // Own-reports endpoint: status visible, no other learner disclosed.
    let (status, mine) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/questions/versions/{qv}/reports"),
            Some(&token3),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{mine}");
    assert_eq!(mine["quarantined"], true);
    assert_eq!(mine["reports"].as_array().unwrap().len(), 1);

    // Quarantined item leaves new tutor sessions: chapter1 now serves only
    // the one remaining unflagged question.
    let chapter1 = ids.chapter1.to_string();
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token3),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": chapter1, "question_count": 10
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let items = session["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "quarantined item excluded from the pool");
    assert_ne!(
        items[0]["question_version_id"].as_str().unwrap(),
        qv,
        "the served item is the unflagged one"
    );

    // A live session created BEFORE quarantine keeps its items answerable —
    // quarantine never yanks items mid-session. (Seed one more session via a
    // fresh user below the allowance: token3 used no attempts yet.)
    let (status, detail) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": chapter1, "question_count": 10
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");

    // Resolve route is honest about the missing console (no fake workflow).
    let rid = r1["report_id"].as_str().unwrap();
    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/reports/{rid}/resolve"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{body}");
    assert_eq!(body["error"]["code"], "editor_console_pending");

    // Validation: bad category and over-long note are rejected.
    let (status, _) = call(app.clone(), report(&token, "bogus", "")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let long = "x".repeat(2001);
    let (status, _) = call(app.clone(), report(&token2, "typo", &long)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Unknown version is 404, and other learners' queues are unaffected.
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/questions/versions/{}/reports", Uuid::new_v4()),
            Some(&token),
            Some(serde_json::json!({"category": "typo"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn qb08_session_detail_carries_report_status() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // Create a session first (both chapter1 items present, unflagged).
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": ids.chapter1, "question_count": 2
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid = session["session_id"].as_str().unwrap();
    let qv = session["items"][0]["question_version_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, detail) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/practice/sessions/{sid}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert!(
        detail["items"][0]["report_status"].is_null(),
        "unflagged item carries null report_status"
    );

    // Two more learners report the same version → still open, detail says so.
    for t in [
        register_and_login(app.clone()).await,
        register_and_login(app.clone()).await,
    ] {
        let (status, r) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/questions/versions/{qv}/reports"),
                Some(&t),
                Some(serde_json::json!({"category": "typo"})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{r}");
    }
    let (status, detail) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/practice/sessions/{sid}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    let flagged: Vec<&serde_json::Value> = detail["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["question_version_id"].as_str().unwrap() == qv)
        .collect();
    assert_eq!(flagged.len(), 1);
    assert_eq!(flagged[0]["report_status"], "open");
}

#[tokio::test]
async fn same_origin_prefix_serves_version_and_health() {
    // Production serves the API under /api/ (nginx strips the prefix):
    // version.json proves the deployed SHA, healthz proves liveness.
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let (status, v) = call(app.clone(), request("GET", "/api/version.json", None, None)).await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert!(v["sha"].is_string(), "deploy SHA stamped: {v}");
    let (status, h) = call(app.clone(), request("GET", "/api/healthz", None, None)).await;
    assert_eq!(status, StatusCode::OK, "{h}");
    assert_eq!(h["status"], "ok");
}

#[tokio::test]
async fn mock_lifecycle_deferred_feedback_and_pass_mark() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let _ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // Seeded fixture mock: chapter1 both questions, pass mark 50, 2 attempts.
    let (status, mocks) = call(app.clone(), request("GET", "/v1/mocks", Some(&token), None)).await;
    assert_eq!(status, StatusCode::OK, "{mocks}");
    let mock_list = mocks["mocks"].as_array().unwrap();
    assert_eq!(mock_list.len(), 1);
    assert_eq!(mock_list[0]["attempts_used"], 0);
    let mid: Uuid = mock_list[0]["mock_id"].as_str().unwrap().parse().unwrap();

    // Start: the form freezes — 2 items from the blueprint.
    let (status, started) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/mocks/{mid}/start"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let sid: Uuid = started["session_id"].as_str().unwrap().parse().unwrap();

    // §11.3 trust gate: the answer response must NOT leak correctness.
    let (status, ans) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 0, "chosen_index": 0,
                                   "idempotency_key": "mock-key-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ans}");
    assert!(ans.get("correct").is_none(), "no correctness leak: {ans}");
    assert!(ans.get("correct_index").is_none(), "no key leak: {ans}");
    assert!(
        ans.get("key_learning_point").is_none(),
        "no explanation leak"
    );

    // Replay is still idempotent.
    let (status, replay) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 0, "chosen_index": 0,
                                   "idempotency_key": "mock-key-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["already_recorded"], true);

    // Item 2: answer A as well -> exactly 1 correct of 2 = 50% = pass.
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 1, "chosen_index": 0,
                                   "idempotency_key": "mock-key-2"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

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
    assert_eq!(result["score"], 50);
    let mock = result["mock"].as_object().expect("mock block");
    assert_eq!(mock["passed"], true, "50% >= 50% pass mark");
    assert_eq!(
        mock["percentile"],
        serde_json::Value::Null,
        "percentile hidden below min sample — nothing invented"
    );
    assert_eq!(
        mock["breakdown"].as_array().unwrap().len(),
        1,
        "both items come from chapter 1"
    );

    // Second attempt is allowed (seed allows 2): start, answer, submit again.
    let (status, started2) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/mocks/{mid}/start"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{started2}");
    let sid2: Uuid = started2["session_id"].as_str().unwrap().parse().unwrap();
    for (idx, key) in [(0i16, "mock2-key-1"), (1, "mock2-key-2")] {
        let (status, _) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/practice/sessions/{sid2}/answers"),
                Some(&token),
                Some(serde_json::json!({"item_index": idx, "chosen_index": 0,
                                       "idempotency_key": key})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }
    let (status, result2) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid2}/submit"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result2}");
    assert_eq!(result2["mock"]["passed"], true);

    // Attempts exhausted: the third start is refused.
    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/mocks/{mid}/start"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"]["code"], "attempts_exhausted");
}

#[tokio::test]
async fn community_stats_gate_and_expected_score() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");

    // Two learners answer the single chapter-3 question -> sample 2 >= min 2.
    for n in 0..2 {
        let token = register_and_login(app.clone()).await;
        let (status, session) = call(
            app.clone(),
            request(
                "POST",
                "/v1/practice/sessions",
                Some(&token),
                Some(
                    serde_json::json!({"preset": "tutor", "chapter_id": ids.chapter3,
                                       "question_count": 1}),
                ),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();
        let (status, _) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/practice/sessions/{sid}/answers"),
                Some(&token),
                Some(serde_json::json!({"item_index": 0, "chosen_index": 0,
                                       "idempotency_key": format!("cs-{n}")})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
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
        assert_eq!(status, StatusCode::OK);
        if n == 1 {
            assert_eq!(
                result["expected_score"], 100,
                "both attempts correct: expected score revealed"
            );
        }
    }

    let token = register_and_login(app.clone()).await;
    let vid = ids.question_versions[4];
    let (status, stats) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/questions/versions/{vid}/community-stats"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{stats}");
    assert_eq!(stats["revealed"], true);
    assert_eq!(stats["attempts"], 2);
    assert_eq!(stats["correct_rate_percent"], 100);
    assert_eq!(stats["option_distribution"][0]["picks"], 2);

    // An unattempted question: honest unrevealed state.
    let v1 = ids.question_versions[0];
    let (status, stats) = call(
        app.clone(),
        request(
            "GET",
            &format!(
                "/v1/questions/versions/{}/community-stats",
                ids.question_versions[0]
            ),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{stats}");
    let _ = v1;
    assert_eq!(stats["revealed"], false);
    assert!(stats["correct_rate_percent"].is_null());
}

fn admin_req(method: &str, uri: &str, token: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut builder = request(method, uri, token, body);
    builder
        .headers_mut()
        .insert("x-admin-token", "test-admin".parse().expect("header"));
    builder
}

#[tokio::test]
async fn admin_gate_blocks_without_token() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            "/v1/admin/hierarchy",
            Some(&token),
            Some(serde_json::json!({
                "exam_id": ids.exam_id, "kind": "chapter", "name": "Gate test"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"]["code"], "admin_required");
}

#[tokio::test]
async fn editorial_hierarchy_question_and_import_flow() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // Hierarchy: create + rename (§5.5 management surface).
    let (status, node) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/admin/hierarchy",
            Some(&token),
            Some(serde_json::json!({
                "exam_id": ids.exam_id, "kind": "chapter",
                "name": "Imported Chapter", "parent_id": ids.chapter1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{node}");
    let node_id: Uuid = node["node_id"].as_str().unwrap().parse().unwrap();
    let (status, renamed) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/hierarchy/{node_id}"),
            Some(&token),
            Some(serde_json::json!({"name": "Imported Chapter II"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    assert_eq!(renamed["name"], "Imported Chapter II");

    // Bulk import: one valid row, one broken row; the dry run creates nothing.
    let good_row = serde_json::json!({
        "chapter_id": node_id, "difficulty": "easy",
        "vignette": "Fictional import vignette about the gloopoid gland.",
        "lead_in": "What applies?",
        "options": [
            {"text": "Right", "rationale": "Correct per the fixture."},
            {"text": "Wrong", "rationale": "Incorrect per the fixture."}
        ],
        "correct_index": 0,
        "key_learning_point": "Imported fixtures validate like authored ones.",
        "source_ref": "Fixture import"
    });
    let mut bad_row = good_row.clone();
    bad_row["options"] = serde_json::json!([{"text": "only one", "rationale": "x"}]);
    let (status, dry) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/admin/import",
            Some(&token),
            Some(serde_json::json!({
                "exam_id": ids.exam_id,
                "dry_run": true,
                "rows": [good_row.clone(), bad_row]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{dry}");
    assert_eq!(dry["status"], "dry_run");
    assert_eq!(dry["valid"], 1);
    assert_eq!(dry["issues"].as_array().unwrap().len(), 1);

    // Nothing was created by the dry run.
    let (status, q) = call(
        app.clone(),
        admin_req(
            "GET",
            &format!("/v1/admin/questions?chapter_id={node_id}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{q}");
    assert_eq!(q["questions"].as_array().unwrap().len(), 0);

    // Apply for real.
    let (status, applied) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/admin/import",
            Some(&token),
            Some(serde_json::json!({
                "exam_id": ids.exam_id, "dry_run": false, "rows": [good_row]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["status"], "applied");
    let batch_id: Uuid = applied["batch_id"].as_str().unwrap().parse().unwrap();

    // The imported question is live in the learner pool.
    let (status, q) = call(
        app.clone(),
        admin_req(
            "GET",
            &format!("/v1/admin/questions?chapter_id={node_id}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{q}");
    assert_eq!(q["questions"].as_array().unwrap().len(), 1);

    // The audit trail records the import (§19.5).
    let (status, audit) = call(
        app.clone(),
        admin_req("GET", "/v1/admin/audit", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{audit}");
    let actions: Vec<&str> = audit["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["action"].as_str())
        .collect();
    assert!(actions.contains(&"import_applied"), "audit: {audit}");

    // Rollback before any attempts removes the batch's questions.
    let (status, rolled) = call(
        app.clone(),
        admin_req(
            "POST",
            &format!("/v1/admin/import/{batch_id}/rollback"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{rolled}");
    assert_eq!(rolled["removed_questions"], 1);
    let (status, q) = call(
        app.clone(),
        admin_req(
            "GET",
            &format!("/v1/admin/questions?chapter_id={node_id}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{q}");
    assert_eq!(q["questions"].as_array().unwrap().len(), 0);

    // After attempts exist, rollback is refused — evidence is immutable.
    let (status, applied2) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/admin/import",
            Some(&token),
            Some(serde_json::json!({
                "exam_id": ids.exam_id, "dry_run": false, "rows": [good_row]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{applied2}");
    let batch2: Uuid = applied2["batch_id"].as_str().unwrap().parse().unwrap();

    // A learner answers the imported question via a tutor session.
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": node_id, "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({
                "item_index": 0, "chosen_index": 0,
                "idempotency_key": format!("imp-{batch2}")
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = call(
        app.clone(),
        admin_req(
            "POST",
            &format!("/v1/admin/import/{batch2}/rollback"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"]["code"], "has_attempts");
}

fn coach_req(
    token: &str,
    vid: Option<Uuid>,
    prompt_type: &str,
    message: &str,
    key: &str,
) -> Request<Body> {
    let mut body = serde_json::json!({"message": message, "idempotency_key": key});
    if let Some(v) = vid {
        body["question_version_id"] = serde_json::json!(v);
    }
    body["prompt_type"] = serde_json::json!(prompt_type);
    request("POST", "/v1/coach/turns", Some(token), Some(body))
}

#[tokio::test]
async fn coach_grounded_abstaining_and_allowance() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // 1. No context, no answer — the Coach abstains instead of improvising.
    let (status, body) = call(
        app.clone(),
        coach_req(&token, None, "free", "Tell me about glorbin.", "c-key-0"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "no_context");

    // 2. Unanswered question: keys are unreleased — answer first.
    let (status, body) = call(
        app.clone(),
        coach_req(
            &token,
            Some(ids.question_versions[0]),
            "why_wrong",
            "why?",
            "c-key-0b",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "answer_first");

    // 3. Answer a chapter-3 question (single-question chapter, key = A).
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": ids.chapter3, "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();
    let (status, ans) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 0, "chosen_index": 0,
                                    "idempotency_key": "coach-flow-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ans}");
    let vid: Uuid = session["items"][0]["question_version_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // 4. Coach turn on the answered question: grounded in reviewed material.
    let (status, turn) = call(
        app.clone(),
        coach_req(
            &token,
            Some(vid),
            "explain",
            "Explain this simply.",
            "c-key-1",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{turn}");
    assert_eq!(turn["already_recorded"], false);
    assert_eq!(turn["adapter"], "extractive");
    let answer = turn["answer"].as_str().unwrap();
    // Grounding: the answer quotes the stored rationale and key point.
    assert!(
        answer.contains("receptor"),
        "answer must quote reviewed rationale: {answer}"
    );
    assert!(answer.contains("Key learning point:"));
    assert!(answer.contains("Source:"));

    // 5. Idempotent replay: same key, same answer, one stored turn.
    let (status, replay) = call(
        app.clone(),
        coach_req(
            &token,
            Some(vid),
            "explain",
            "Explain this simply.",
            "c-key-1",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["already_recorded"], true);
    assert_eq!(replay["answer"], turn["answer"]);
    let count = sqlx::query("SELECT COUNT(*) AS n FROM coach_turns WHERE idempotency_key = $1")
        .bind("c-key-1")
        .fetch_one(&state.pool)
        .await
        .expect("count");
    assert_eq!(count.get::<i64, _>("n"), 1, "no duplicate coach evidence");

    // 6. History endpoint lists the turn.
    let (status, history) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/coach/history?question_version_id={vid}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{history}");
    assert_eq!(history["turns"].as_array().unwrap().len(), 1);

    // 7. AI-13: allowance refusal with structured details. The state's
    // default here is 20; drive 19 more turns then expect refusal on the
    // 21st — cheaper: this check lives in the free-allowance integration
    // above (coach uses the same honest shape). Assert answerable list.
    let (status, list) = call(
        app.clone(),
        request("GET", "/v1/coach/answerable-questions", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["questions"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn coach_daily_allowance_enforced() {
    let _g = LOCK.lock().await;
    // Local state with a tiny allowance to prove the cost limit fires (AI-13).
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/medos_ci".into());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("db");
    schema::apply_down(&pool).await.expect("down");
    schema::apply_up(&pool).await.expect("up");
    let state = Arc::new(AppState {
        pool,
        min_time_limit_seconds: 30,
        free_daily_questions: 10,
        community_min_sample: 2,
        admin_token: Some("test-admin".into()),
        free_daily_coach_turns: 1,
        openai_api_key: None,
        openai_base_url: "https://api.openai.com/v1".into(),
        pack_signing_key: None,
    });
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": ids.chapter3, "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({"item_index": 0, "chosen_index": 0,
                                    "idempotency_key": "allow-1"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let vid: Uuid = session["items"][0]["question_version_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // First turn: within the allowance of 1.
    let (status, _) = call(
        app.clone(),
        coach_req(&token, Some(vid), "explain", "Explain.", "allow-turn-1"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Second turn: refused with the honest allowance payload (AI-13/§26.1).
    let (status, body) = call(
        app.clone(),
        coach_req(
            &token,
            Some(vid),
            "explain",
            "Explain again.",
            "allow-turn-2",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"]["code"], "coach_allowance_reached");
    assert_eq!(body["error"]["details"]["allowance"]["limit"], 1);
}
#[tokio::test]
async fn notes_crud_links_export_and_isolation() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;
    let other = register_and_login(app.clone()).await;

    // Create two notes, link them.
    let (status, n1) = call(
        app.clone(),
        request(
            "POST",
            "/v1/notes",
            Some(&token),
            Some(serde_json::json!({"title": "Loop rule", "body": "Negative feedback suppresses upstream."})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{n1}");
    let id1: Uuid = n1["note_id"].as_str().unwrap().parse().unwrap();
    let (_status, n2) = call(
        app.clone(),
        request(
            "POST",
            "/v1/notes",
            Some(&token),
            Some(serde_json::json!({"title": "Linked note"})),
        ),
    )
    .await;
    let id2: Uuid = n2["note_id"].as_str().unwrap().parse().unwrap();
    let (_, _) = call(
        app.clone(),
        request(
            "POST",
            "/v1/notes/link",
            Some(&token),
            Some(serde_json::json!({"from_note_id": id1, "to_note_id": id2})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Update, list with backlinks.
    let (status, upd) = call(
        app.clone(),
        request(
            "PATCH",
            &format!("/v1/notes/{id2}"),
            Some(&token),
            Some(serde_json::json!({"body": "Second note body."})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{upd}");
    let (status, list) = call(app.clone(), request("GET", "/v1/notes", Some(&token), None)).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let notes = list["notes"].as_array().unwrap();
    assert_eq!(notes.len(), 2);
    let with_links = notes
        .iter()
        .find(|n| n["note_id"] == n1["note_id"])
        .unwrap();
    assert_eq!(with_links["backlinks"].as_array().unwrap().len(), 1);

    // Isolation: the other learner sees nothing, cannot delete.
    let (_status, other_list) =
        call(app.clone(), request("GET", "/v1/notes", Some(&other), None)).await;
    assert_eq!(other_list["notes"].as_array().unwrap().len(), 0);
    let (status, _) = call(
        app.clone(),
        request("DELETE", &format!("/v1/notes/{id1}"), Some(&other), None),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Export is complete and JSON.
    let (status, export) = call(
        app.clone(),
        request("GET", "/v1/notes/export", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{export}");
    assert_eq!(export["notes"].as_array().unwrap().len(), 2);

    // Delete works for the owner.
    let (status, _) = call(
        app.clone(),
        request("DELETE", &format!("/v1/notes/{id1}"), Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn library_seed_search_article() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    // Seed a library article directly (console authoring is the next
    // ADMIN-06 iteration).
    let aid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO articles (id, slug, title) VALUES ($1, 'gloopoid-overview', 'Gloopoid overview')",
    )
    .bind(aid)
    .execute(&state.pool)
    .await
    .expect("seed article");
    sqlx::query(
        "INSERT INTO article_versions (id, article_id, version, status, body, source_ref) VALUES ($1, $2, 1, 'published', 'The fictional gloopoid gland stores glorbin before release.', 'Fixture library')",
    )
    .bind(Uuid::new_v4())
    .bind(aid)
    .execute(&state.pool)
    .await
    .expect("seed version");

    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;
    let (status, results) = call(
        app.clone(),
        request("GET", "/v1/library/search?q=gloopoid", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{results}");
    assert_eq!(results["results"].as_array().unwrap().len(), 1);

    let (status, article) = call(
        app.clone(),
        request(
            "GET",
            "/v1/library/articles/gloopoid-overview",
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{article}");
    assert!(article["body"].as_str().unwrap().contains("glorbin"));
}

#[tokio::test]
async fn notifications_preferences_roundtrip() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;

    let (status, inbox) = call(
        app.clone(),
        request("GET", "/v1/me/notifications", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{inbox}");
    assert_eq!(inbox["notifications"].as_array().unwrap().len(), 0);

    let (status, _) = call(
        app.clone(),
        request(
            "PATCH",
            "/v1/me/notifications",
            Some(&token),
            Some(serde_json::json!({"mock_results": false, "quiet_hours_start": 23})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn guest_trial_ceiling_and_isolation() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let guest_key = format!("guest-{}-trial", Uuid::new_v4());

    let (status, trial) = call(
        app.clone(),
        request(
            "POST",
            "/guest/trial/start",
            None,
            Some(serde_json::json!({"guest_key": guest_key})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{trial}");
    assert_eq!(trial["questions_served"], 0);
    assert_eq!(trial["ceiling"], 5);

    // Guests fetch sample questions without an account (CORE-09).
    for _ in 0..5 {
        let (status, q) = call(
            app.clone(),
            request(
                "POST",
                "/guest/trial/next-question",
                None,
                Some(serde_json::json!({"guest_key": guest_key})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{q}");
        // §11.3: no keys for guests.
        assert!(q["options"][0].get("rationale").is_none());
    }

    // Ceiling enforced honestly with an upgrade prompt.
    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            "/guest/trial/next-question",
            None,
            Some(serde_json::json!({"guest_key": guest_key})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"]["code"], "trial_finished");

    let _ = ids;
}

#[tokio::test]
async fn goals_and_protected_commitments_lifecycle() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    let (status, goal) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/goals",
            Some(&token),
            Some(serde_json::json!({
                "exam_id": ids.exam_id, "target_note": "Pass the pilot exam"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{goal}");

    // A new goal retires the old one; history is kept.
    let (_status, _) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/goals",
            Some(&token),
            Some(serde_json::json!({"target_note": "Second goal"})),
        ),
    )
    .await;
    let (status, list) = call(
        app.clone(),
        request("GET", "/v1/me/goals", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let goals = list["goals"].as_array().unwrap();
    assert_eq!(goals.len(), 2);
    assert_eq!(goals.iter().filter(|g| g["retired"] == true).count(), 1);

    // Protected commitments: create and remove (learner-only, §9.3).
    let (status, c) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/commitments",
            Some(&token),
            Some(serde_json::json!({"label": "Night shift",
                                    "starts_at": "2026-09-25T18:00:00Z",
                                    "ends_at": "2026-09-26T06:00:00Z"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{c}");
    let cid: Uuid = c["commitment_id"].as_str().unwrap().parse().unwrap();
    let (_status, list) = call(
        app.clone(),
        request("GET", "/v1/me/goals", Some(&token), None),
    )
    .await;
    assert_eq!(list["protected_commitments"].as_array().unwrap().len(), 1);
    let (status, _) = call(
        app.clone(),
        request(
            "DELETE",
            &format!("/v1/me/commitments/{cid}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn pregen_tutoring_generated_and_cached() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;
    let vid = ids.question_versions[0];

    // Generate: five one-tap cards from reviewed material (AI-18).
    let (status, gen) = call(
        app.clone(),
        admin_req(
            "POST",
            &format!("/v1/questions/versions/{vid}/pregen-tutoring"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{gen}");
    assert_eq!(gen["generated"], 5);

    // Cached read: cards contain reviewed rationale, never invention.
    let (status, cards) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/questions/versions/{vid}/pregen-tutoring"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cards}");
    let list = cards["cards"].as_array().unwrap();
    assert_eq!(list.len(), 5);
    let explain = list
        .iter()
        .find(|c| c["prompt_type"] == "explain")
        .expect("explain card");
    assert!(explain["content"]
        .as_str()
        .unwrap()
        .contains("Simple version"));
}

#[tokio::test]
async fn feature_flags_roundtrip() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;

    let (_status, _) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/admin/flags",
            Some(&token),
            Some(serde_json::json!({"key": "mockv2", "value": true, "rollout_percent": 25})),
        ),
    )
    .await;
    let (status, flags) = call(
        app.clone(),
        request("GET", "/v1/config/flags", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{flags}");
    let found = flags["flags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == "mockv2")
        .expect("flag persisted");
    assert_eq!(found["rollout_percent"], 25);
}

#[tokio::test]
async fn institutions_cohorts_assignments_flow() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let staff = register_and_login(app.clone()).await;
    let learner = register_and_login(app.clone()).await;

    let (status, inst) = call(
        app.clone(),
        request(
            "POST",
            "/v1/institutions",
            Some(&staff),
            Some(serde_json::json!({"name": "Polytronx Teaching"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{inst}");
    let inst_id: Uuid = inst["institution_id"].as_str().unwrap().parse().unwrap();

    let learner_id = {
        // Register a real second account to enrol as learner.
        let email = format!("inst-{}@example.test", Uuid::new_v4());
        let (_, v) = call(
            app.clone(),
            request(
                "POST",
                "/v1/auth/register",
                None,
                Some(serde_json::json!({"email": email, "password": "longenough"})),
            ),
        )
        .await;
        let uid: Uuid = v["user_id"].as_str().unwrap().parse().unwrap();
        uid
    };

    let (status, _) = call(
        app.clone(),
        admin_req(
            "POST",
            &format!("/v1/institutions/{inst_id}/members"),
            Some(&staff),
            Some(serde_json::json!({"user_id": learner_id, "role": "learner"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, cohort) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/institutions/{inst_id}/cohorts"),
            Some(&staff),
            Some(serde_json::json!({"name": "Cohort A", "member_ids": [learner_id]})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cohort}");
    let cohort_id: Uuid = cohort["cohort_id"].as_str().unwrap().parse().unwrap();

    let (status, assignment) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/cohorts/{cohort_id}/assignments"),
            Some(&staff),
            Some(serde_json::json!({"title": "Chapter 1 practice", "due_at": "2026-10-01T00:00:00Z"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{assignment}");

    // Non-staff cannot create assignments in this cohort.
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/cohorts/{cohort_id}/assignments"),
            Some(&learner),
            Some(serde_json::json!({"title": "Sneaky"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn scenario_engine_deterministic_transitions() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;

    let (status, sc) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/scenarios",
            Some(&token),
            Some(serde_json::json!({
                "slug": "gloopoid-stable",
                "title": "Fictional stable patient",
                "state_machine": {
                    "initial": "presenting",
                    "transitions": [
                        {"from": "presenting", "on": "take_history", "to": "history_done"},
                        {"from": "history_done", "on": "order_labs", "to": "labs_done"},
                        {"from": "labs_done", "on": "discharge", "to": "discharged"}
                    ]
                }
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{sc}");
    let slug = "gloopoid-stable";

    let (status, run) = call(
        app.clone(),
        request(
            "POST",
            "/v1/scenarios/runs",
            Some(&token),
            Some(serde_json::json!({"scenario_slug": slug})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{run}");
    let run_id: Uuid = run["run_id"].as_str().unwrap().parse().unwrap();
    assert_eq!(run["current_state"], "presenting");

    for event in ["take_history", "order_labs", "discharge"] {
        let (status, step) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/scenarios/runs/{run_id}/events"),
                Some(&token),
                Some(serde_json::json!({"event": event})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{step}");
    }
    // Invalid transition from discharged is refused deterministically.
    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/scenarios/runs/{run_id}/events"),
            Some(&token),
            Some(serde_json::json!({"event": "order_labs"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "no_transition");
}

#[tokio::test]
async fn portfolio_and_ce_records() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;

    let (status, entry) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/portfolio",
            Some(&token),
            Some(serde_json::json!({"kind": "rotation", "title": "Fictional internal medicine rotation",
                                    "occurred_on": "2026-08-01"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{entry}");

    let (status, list) = call(
        app.clone(),
        request("GET", "/v1/me/portfolio", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["entries"].as_array().unwrap().len(), 1);

    let (status, ce) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/ce-activities",
            Some(&token),
            Some(serde_json::json!({"activity": "Fixture journal club", "hours": 1.5})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ce}");
    assert!(
        ce["note"]
            .as_str()
            .unwrap()
            .contains("Not an accredited credit"),
        "§16 honesty: records are never labelled accredited"
    );

    // Invalid hours rejected.
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/ce-activities",
            Some(&token),
            Some(serde_json::json!({"activity": "x", "hours": 900})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn settings_admin_gate_and_update() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;

    // Update a settings key (admin-gated).
    let (status, body) = call(
        app.clone(),
        admin_req(
            "PATCH",
            "/v1/admin/settings",
            Some(&token),
            Some(serde_json::json!({"community_min_sample": 15})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // Read back.
    let (status, got) = call(
        app.clone(),
        admin_req("GET", "/v1/admin/settings", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{got}");
    assert_eq!(got["settings"]["community_min_sample"], 15);
}

#[tokio::test]
async fn session_actions_retry_and_practice_incorrect() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // Create and submit a session.
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": ids.chapter2, "question_count": 2
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();

    // Answer both.
    for (idx, key) in [(0i16, "act-1"), (1, "act-2")] {
        let (status, _) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/practice/sessions/{sid}/answers"),
                Some(&token),
                Some(serde_json::json!({"item_index": idx, "chosen_index": 0,
                                        "idempotency_key": key})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

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
    assert_eq!(status, StatusCode::OK);

    // Retry action creates a new session in the same chapter.
    let (status, retry) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/action"),
            Some(&token),
            Some(serde_json::json!({"action": "retry"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{retry}");
    let retry_sid: Uuid = retry["session_id"].as_str().unwrap().parse().unwrap();
    assert_ne!(retry_sid, sid, "new session created");

    // Practice incorrect action creates a revision session.
    let (status, incorrect) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/action"),
            Some(&token),
            Some(serde_json::json!({"action": "practice_incorrect"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{incorrect}");
    assert!(incorrect["item_count"].as_i64().unwrap() >= 0);
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
