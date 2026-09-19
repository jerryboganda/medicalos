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
    Arc::new(AppState {
        pool,
        min_time_limit_seconds: 30,
        free_daily_questions: 10,
        expose_test_auth_tokens: true,
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

fn tenant_request(
    method: &str,
    uri: &str,
    token: &str,
    tenant_id: Uuid,
    body: Option<Value>,
) -> Request<Body> {
    let mut req = request(method, uri, Some(token), body);
    req.headers_mut().insert(
        "X-Tenant-Id",
        tenant_id.to_string().parse().expect("tenant header"),
    );
    req
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
    let verification_token = v["verification_token"]
        .as_str()
        .expect("test verification token");
    let (status, verified) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/verify-email",
            None,
            Some(serde_json::json!({"token": verification_token})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "verify: {verified}");
    let device_id = format!("test-device-{}", Uuid::new_v4());
    let (status, v) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/login",
            None,
            Some(serde_json::json!({
                "email": email,
                "password": "correct horse",
                "device_id": device_id,
                "device_name": "Integration test"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login: {v}");
    v["token"].as_str().expect("token").to_string()
}

#[tokio::test]
async fn qb13_hint_records_assistance_and_calculators_use_shared_engine() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let ids = seed::seed(&state.pool).await.expect("seed");
    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;

    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor",
                "chapter_id": ids.chapter1,
                "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid = session["session_id"].as_str().expect("session id");

    let (status, hint) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/items/0/hint"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{hint}");
    assert!(hint["hint"].as_str().is_some_and(|value| !value.is_empty()));

    let (status, answer) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({
                "item_index": 0,
                "chosen_index": 0,
                "idempotency_key": "qb13-assisted"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let assisted: bool = sqlx::query_scalar(
        "SELECT assisted FROM attempts WHERE session_id = $1 AND item_index = 0",
    )
    .bind(Uuid::parse_str(sid).unwrap())
    .fetch_one(&state.pool)
    .await
    .expect("assisted attempt");
    assert!(
        assisted,
        "using a tutor hint must persist assisted evidence"
    );

    let (status, denied_after_answer) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/items/0/hint"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{denied_after_answer}");

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

    let (evidence_count, independent_count): (i32, i32) = sqlx::query_as(
        "SELECT evidence_count, independent_count FROM learner_concept_state WHERE chapter_id = $1",
    )
    .bind(ids.chapter1)
    .fetch_one(&state.pool)
    .await
    .expect("learner state after assisted answer");
    assert_eq!(evidence_count, 1);
    assert_eq!(
        independent_count, 0,
        "assisted evidence must not count as independent"
    );

    let (status, timed) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "timed",
                "chapter_id": ids.chapter1,
                "question_count": 1,
                "time_limit_seconds": 30
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{timed}");
    let timed_sid = timed["session_id"].as_str().expect("timed session id");
    let (status, denied) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{timed_sid}/items/0/hint"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{denied}");

    let (status, calc) = call(
        app.clone(),
        request(
            "POST",
            "/v1/tools/calculate",
            Some(&token),
            Some(serde_json::json!({
                "calculator": "bmi",
                "inputs": {"weight_kg": 70.0, "height_m": 1.75}
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{calc}");
    assert!((calc["value"].as_f64().unwrap() - 22.86).abs() < 0.02);
    assert_eq!(calc["unit"], "kg/m2");

    let (status, invalid_calc) = call(
        app.clone(),
        request(
            "POST",
            "/v1/tools/calculate",
            Some(&token),
            Some(serde_json::json!({
                "calculator": "bmi",
                "inputs": {"weight_kg": 70.0}
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{invalid_calc}");
    assert_eq!(invalid_calc["error"]["code"], "invalid_calculator_input");
}

async fn current_user_id(app: Router, token: &str) -> Uuid {
    let (status, contexts) = call(app, request("GET", "/v1/me/contexts", Some(token), None)).await;
    assert_eq!(status, StatusCode::OK, "{contexts}");
    Uuid::parse_str(
        contexts["personal"]["user_id"]
            .as_str()
            .expect("personal user id"),
    )
    .expect("valid personal user id")
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
    let verification_token = v["verification_token"]
        .as_str()
        .expect("test verification token")
        .to_string();

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
            Some(serde_json::json!({
                "email": email,
                "password": "wrong password",
                "device_id": "auth-test",
                "device_name": "Test device"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "wrong password rejected");

    let (status, _) = call(app.clone(), request("GET", "/v1/me/today", None, None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "missing token rejected");

    let (status, verified) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/verify-email",
            None,
            Some(serde_json::json!({"token": verification_token})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{verified}");

    let (status, v) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/login",
            None,
            Some(serde_json::json!({
                "email": email,
                "password": "longenough",
                "device_id": "auth-test",
                "device_name": "Test device"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(v["token"].as_str().is_some());
}

#[tokio::test]
async fn auth_challenge_flows_fail_closed_without_delivery() {
    let _g = LOCK.lock().await;
    let test_state = setup().await;
    let state = Arc::new(AppState {
        pool: test_state.pool.clone(),
        min_time_limit_seconds: test_state.min_time_limit_seconds,
        free_daily_questions: test_state.free_daily_questions,
        expose_test_auth_tokens: false,
    });
    let app = router(state);
    let email = format!("no-delivery-{}@example.test", Uuid::new_v4());

    for _ in 0..2 {
        let (status, body) = call(
            app.clone(),
            request(
                "POST",
                "/v1/auth/register",
                None,
                Some(serde_json::json!({"email": email, "password": "correct horse"})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
        assert_eq!(body["error"]["code"], "auth_email_delivery_unavailable");
    }

    let (status, body) = call(
        app,
        request(
            "POST",
            "/v1/auth/forgot-password",
            None,
            Some(serde_json::json!({"email": email})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["error"]["code"], "auth_email_delivery_unavailable");
}

#[tokio::test]
async fn core07_account_security_lifecycle() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let email = format!("account-{}@example.test", Uuid::new_v4());
    let password = "correct horse battery";

    let (status, registered) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/register",
            None,
            Some(serde_json::json!({"email": email, "password": password})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{registered}");
    assert_eq!(registered["verification_required"], true);
    let verification_token = registered["verification_token"]
        .as_str()
        .expect("test seam exposes verification token");

    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/login",
            None,
            Some(serde_json::json!({
                "email": email,
                "password": password,
                "device_id": "device-a",
                "device_name": "Laptop"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"]["code"], "email_not_verified");

    let (status, verified) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/verify-email",
            None,
            Some(serde_json::json!({"token": verification_token})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{verified}");

    let login = |device_id: &str, device_name: &str, password: &str| {
        request(
            "POST",
            "/v1/auth/login",
            None,
            Some(serde_json::json!({
                "email": email,
                "password": password,
                "device_id": device_id,
                "device_name": device_name
            })),
        )
    };
    let (status, device_a) = call(app.clone(), login("device-a", "Laptop", password)).await;
    assert_eq!(status, StatusCode::OK, "{device_a}");
    let token_a = device_a["token"]
        .as_str()
        .expect("access token")
        .to_string();
    let refresh_a = device_a["refresh_token"]
        .as_str()
        .expect("refresh token")
        .to_string();
    assert!(device_a["session_id"].as_str().is_some());
    assert!(device_a["expires_at"].as_str().is_some());

    let (status, device_b) = call(app.clone(), login("device-b", "Phone", password)).await;
    assert_eq!(status, StatusCode::OK, "{device_b}");
    let token_b = device_b["token"]
        .as_str()
        .expect("access token")
        .to_string();

    let (status, third) = call(app.clone(), login("device-c", "Tablet", password)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{third}");
    assert_eq!(third["error"]["code"], "device_limit_reached");

    let (status, sessions) = call(
        app.clone(),
        request("GET", "/v1/me/sessions", Some(&token_a), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{sessions}");
    assert_eq!(sessions["sessions"].as_array().unwrap().len(), 2);
    assert_eq!(
        sessions["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["current"] == true)
            .count(),
        1
    );

    let (status, refreshed) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/refresh",
            None,
            Some(serde_json::json!({"refresh_token": refresh_a})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{refreshed}");
    let token_a2 = refreshed["token"].as_str().expect("rotated access token");
    let refresh_a2 = refreshed["refresh_token"]
        .as_str()
        .expect("rotated refresh token");
    assert_ne!(token_a2, token_a);
    assert_ne!(refresh_a2, refresh_a);

    let (status, _) = call(
        app.clone(),
        request("GET", "/v1/me/sessions", Some(&token_a), None),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "rotated access token is dead"
    );
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/refresh",
            None,
            Some(serde_json::json!({"refresh_token": refresh_a})),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "refresh replay is rejected"
    );

    let (status, signed_out) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/sessions/sign-out-others",
            Some(token_a2),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{signed_out}");
    assert_eq!(signed_out["revoked"], 1);
    let (status, _) = call(
        app.clone(),
        request("GET", "/v1/me/sessions", Some(&token_b), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, forgot) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/forgot-password",
            None,
            Some(serde_json::json!({"email": email})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{forgot}");
    let reset_token = forgot["reset_token"]
        .as_str()
        .expect("test seam exposes reset token");
    let new_password = "new correct horse battery";
    let (status, reset) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/reset-password",
            None,
            Some(serde_json::json!({"token": reset_token, "password": new_password})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reset}");

    let (status, _) = call(
        app.clone(),
        request("GET", "/v1/me/sessions", Some(token_a2), None),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "password reset revokes sessions"
    );
    let (status, _) = call(app.clone(), login("device-a", "Laptop", password)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "old password is invalid");
    let (status, relogin) = call(app.clone(), login("device-a", "Laptop", new_password)).await;
    assert_eq!(status, StatusCode::OK, "{relogin}");
    let final_token = relogin["token"].as_str().expect("access token");

    let (status, deletion) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/account/deletion",
            Some(final_token),
            Some(serde_json::json!({"password": new_password})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{deletion}");
    assert_eq!(deletion["status"], "pending");
    let (status, _) = call(
        app.clone(),
        request("GET", "/v1/me/sessions", Some(final_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = call(app.clone(), login("device-a", "Laptop", new_password)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"]["code"], "account_deletion_pending");
}

#[tokio::test]
async fn core08_notification_preferences_push_registration_and_empty_inbox() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state);
    let token = register_and_login(app.clone()).await;

    let (status, defaults) = call(
        app.clone(),
        request("GET", "/v1/me/notification-preferences", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{defaults}");
    assert_eq!(defaults["timezone"], "UTC");
    assert!(defaults["quiet_start"].is_null());
    assert!(defaults["quiet_end"].is_null());
    assert_eq!(defaults["categories"]["plan_review_reminders"], true);
    assert_eq!(defaults["categories"]["mock_assignment"], true);
    assert_eq!(defaults["categories"]["competition"], true);
    assert_eq!(defaults["categories"]["duel_invitation"], true);
    assert_eq!(defaults["categories"]["report_resolved"], true);
    assert_eq!(defaults["categories"]["subscription_events"], true);

    let updated_body = serde_json::json!({
        "timezone": "Asia/Karachi",
        "quiet_start": "22:30",
        "quiet_end": "07:00",
        "categories": {
            "plan_review_reminders": false,
            "mock_assignment": true,
            "competition": false,
            "duel_invitation": false,
            "report_resolved": true,
            "subscription_events": true
        }
    });
    let (status, updated) = call(
        app.clone(),
        request(
            "PUT",
            "/v1/notification-preferences",
            Some(&token),
            Some(updated_body.clone()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated, updated_body);

    let (status, persisted) = call(
        app.clone(),
        request("GET", "/v1/me/notification-preferences", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{persisted}");
    assert_eq!(persisted, updated_body);

    let (status, invalid) = call(
        app.clone(),
        request(
            "PUT",
            "/v1/notification-preferences",
            Some(&token),
            Some(serde_json::json!({
                "timezone": "Asia/Karachi",
                "quiet_start": "22:30",
                "quiet_end": null,
                "categories": updated_body["categories"].clone()
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{invalid}");
    assert_eq!(invalid["error"]["code"], "invalid_quiet_hours");

    for push_token in ["apns-token-a", "apns-token-b"] {
        let (status, registered) = call(
            app.clone(),
            request(
                "POST",
                "/v1/push-tokens",
                Some(&token),
                Some(serde_json::json!({
                    "device_id": "phone-1",
                    "platform": "ios",
                    "token": push_token
                })),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{registered}");
        assert_eq!(registered["registered"], true);
    }

    let other_token = register_and_login(app.clone()).await;
    let (status, other_defaults) = call(
        app.clone(),
        request(
            "GET",
            "/v1/me/notification-preferences",
            Some(&other_token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{other_defaults}");
    assert_eq!(other_defaults["timezone"], "UTC");
    assert_eq!(other_defaults["categories"]["plan_review_reminders"], true);

    let (status, inbox) = call(
        app.clone(),
        request("GET", "/v1/notifications", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{inbox}");
    assert_eq!(inbox["notifications"].as_array().unwrap().len(), 0);

    let (status, missing) = call(
        app,
        request(
            "POST",
            &format!("/v1/notifications/{}/read", Uuid::new_v4()),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");
    assert_eq!(missing["error"]["code"], "notification_not_found");
}

#[tokio::test]
async fn core10_seed_maps_taxonomy_and_questions_to_versioned_concepts() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    seed::seed(&state.pool)
        .await
        .expect("seed synthetic content");

    let chapter_mapping = sqlx::query(
        "SELECT COUNT(*) AS n, COUNT(DISTINCT concept_version_id) AS concepts
         FROM curriculum_node_concepts cnc
         JOIN concept_versions cv ON cv.id = cnc.concept_version_id
         JOIN curriculum_nodes cn ON cn.id = cnc.node_id
         WHERE cn.kind = 'chapter' AND cv.status = 'published' AND cv.version = 1",
    )
    .fetch_one(&state.pool)
    .await
    .expect("seeded chapters map to published concept versions");
    assert_eq!(chapter_mapping.get::<i64, _>("n"), 2);
    assert_eq!(chapter_mapping.get::<i64, _>("concepts"), 2);

    let question_mapping = sqlx::query(
        "SELECT COUNT(*) AS n
         FROM question_version_concepts qvc
         JOIN question_versions qv ON qv.id = qvc.question_version_id
         JOIN curriculum_node_concepts cnc
           ON cnc.node_id = qv.chapter_id
          AND cnc.concept_version_id = qvc.concept_version_id
         WHERE qvc.relation = 'primary'",
    )
    .fetch_one(&state.pool)
    .await
    .expect("seeded questions map to their chapter concept version");
    assert_eq!(question_mapping.get::<i64, _>("n"), 4);
}

#[tokio::test]
async fn authenticated_user_has_explicit_personal_and_tenant_contexts() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let token = register_and_login(app.clone()).await;

    let (status, contexts) = call(
        app.clone(),
        request("GET", "/v1/me/contexts", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{contexts}");
    assert_eq!(contexts["personal"]["kind"], "personal");
    assert!(contexts["personal"]["user_id"].as_str().is_some());
    assert_eq!(contexts["tenants"].as_array().unwrap().len(), 0);
    assert_eq!(contexts["platform_roles"].as_array().unwrap().len(), 0);

    let (status, missing_scope) = call(
        app.clone(),
        request("GET", "/v1/tenant/context", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{missing_scope}");
    assert_eq!(missing_scope["error"]["code"], "tenant_scope_required");
}

#[tokio::test]
async fn tenant_roles_and_audit_are_scoped_and_idempotent() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let owner_token = register_and_login(app.clone()).await;
    let support_token = register_and_login(app.clone()).await;
    let admin_token = register_and_login(app.clone()).await;
    let learner_token = register_and_login(app.clone()).await;
    let outsider_token = register_and_login(app.clone()).await;

    let owner_id = current_user_id(app.clone(), &owner_token).await;
    let support_id = current_user_id(app.clone(), &support_token).await;
    let admin_id = current_user_id(app.clone(), &admin_token).await;
    let learner_id = current_user_id(app.clone(), &learner_token).await;
    let outsider_id = current_user_id(app.clone(), &outsider_token).await;

    // Test-only bootstrap: production deliberately has no self-elevation API.
    sqlx::query("INSERT INTO platform_roles (user_id, role) VALUES ($1, $2), ($3, $4)")
        .bind(owner_id)
        .bind("platform_owner")
        .bind(support_id)
        .bind("support")
        .execute(&state.pool)
        .await
        .expect("bootstrap platform roles");

    let (status, first_tenant) = call(
        app.clone(),
        request(
            "POST",
            "/v1/platform/tenants",
            Some(&owner_token),
            Some(serde_json::json!({
                "name": "North Medical College",
                "initial_administrator_user_id": admin_id
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first_tenant}");
    let first_tenant_id = Uuid::parse_str(first_tenant["tenant_id"].as_str().expect("tenant id"))
        .expect("valid tenant id");
    let first_audit_id = Uuid::parse_str(
        first_tenant["audit_event_id"]
            .as_str()
            .expect("tenant audit event"),
    )
    .expect("valid tenant audit event");

    let (status, second_tenant) = call(
        app.clone(),
        request(
            "POST",
            "/v1/platform/tenants",
            Some(&owner_token),
            Some(serde_json::json!({
                "name": "South Medical College",
                "initial_administrator_user_id": admin_id
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{second_tenant}");
    let second_tenant_id = Uuid::parse_str(
        second_tenant["tenant_id"]
            .as_str()
            .expect("second tenant id"),
    )
    .expect("valid second tenant id");

    let (status, owner_contexts) = call(
        app.clone(),
        request("GET", "/v1/me/contexts", Some(&owner_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{owner_contexts}");
    assert_eq!(owner_contexts["platform_roles"][0], "platform_owner");
    assert_eq!(owner_contexts["tenants"].as_array().unwrap().len(), 0);

    let (status, admin_contexts) = call(
        app.clone(),
        request("GET", "/v1/me/contexts", Some(&admin_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{admin_contexts}");
    assert_eq!(admin_contexts["tenants"].as_array().unwrap().len(), 2);
    assert_eq!(
        admin_contexts["tenants"][0]["roles"][0],
        "institution_administrator"
    );

    let (status, active_tenant) = call(
        app.clone(),
        tenant_request(
            "GET",
            "/v1/tenant/context",
            &admin_token,
            first_tenant_id,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{active_tenant}");
    assert_eq!(active_tenant["tenant_id"], first_tenant_id.to_string());
    assert_eq!(active_tenant["roles"][0], "institution_administrator");

    let assignment = serde_json::json!({"user_id": learner_id, "role": "learner"});
    let (status, added) = call(
        app.clone(),
        tenant_request(
            "POST",
            "/v1/tenant/memberships",
            &admin_token,
            first_tenant_id,
            Some(assignment.clone()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{added}");
    assert_eq!(added["changed"], true);
    let membership_id = added["membership_id"]
        .as_str()
        .expect("membership id")
        .to_owned();
    let membership_audit_id = added["audit_event_id"]
        .as_str()
        .expect("membership audit event")
        .to_owned();

    let (status, duplicate) = call(
        app.clone(),
        tenant_request(
            "POST",
            "/v1/tenant/memberships",
            &admin_token,
            first_tenant_id,
            Some(assignment),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{duplicate}");
    assert_eq!(duplicate["changed"], false);
    assert_eq!(duplicate["membership_id"], membership_id);
    assert!(duplicate["audit_event_id"].is_null());

    let (status, second_role) = call(
        app.clone(),
        tenant_request(
            "POST",
            "/v1/tenant/memberships",
            &admin_token,
            first_tenant_id,
            Some(serde_json::json!({"user_id": learner_id, "role": "instructor"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{second_role}");
    assert_eq!(second_role["changed"], true);
    assert_eq!(second_role["aggregate_version"], 2);

    let (status, learner_context) = call(
        app.clone(),
        tenant_request(
            "GET",
            "/v1/tenant/context",
            &learner_token,
            first_tenant_id,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{learner_context}");
    assert_eq!(learner_context["roles"].as_array().unwrap().len(), 2);
    assert!(learner_context["roles"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("learner")));
    assert!(learner_context["roles"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("instructor")));

    let (status, cross_tenant) = call(
        app.clone(),
        tenant_request(
            "GET",
            "/v1/tenant/context",
            &learner_token,
            second_tenant_id,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{cross_tenant}");
    assert_eq!(cross_tenant["error"]["code"], "tenant_access_denied");

    let (status, non_admin) = call(
        app.clone(),
        tenant_request(
            "POST",
            "/v1/tenant/memberships",
            &learner_token,
            first_tenant_id,
            Some(serde_json::json!({"user_id": outsider_id, "role": "learner"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{non_admin}");
    assert_eq!(non_admin["error"]["code"], "tenant_admin_required");

    let (status, support_denied) = call(
        app.clone(),
        request(
            "POST",
            "/v1/platform/tenants",
            Some(&support_token),
            Some(serde_json::json!({
                "name": "Support Must Not Create",
                "initial_administrator_user_id": outsider_id
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{support_denied}");
    assert_eq!(support_denied["error"]["code"], "platform_owner_required");

    let (status, owner_audit) = call(
        app.clone(),
        tenant_request(
            "GET",
            &format!("/v1/tenant/audit/{first_audit_id}"),
            &admin_token,
            first_tenant_id,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{owner_audit}");
    assert_eq!(owner_audit["actor_user_id"], owner_id.to_string());
    assert_eq!(owner_audit["tenant_id"], first_tenant_id.to_string());
    assert_eq!(owner_audit["subject_user_id"], admin_id.to_string());
    assert_eq!(owner_audit["aggregate_type"], "tenant");
    assert_eq!(owner_audit["aggregate_id"], first_tenant_id.to_string());
    assert_eq!(owner_audit["aggregate_version"], 1);
    assert_eq!(owner_audit["action"], "tenant.created");
    assert_eq!(owner_audit["privacy_scope"], "tenant_admin");
    assert!(owner_audit["occurred_at"].as_str().is_some());
    assert!(owner_audit["received_at"].as_str().is_some());
    assert!(owner_audit["correlation_id"].as_str().is_some());
    assert!(owner_audit.get("device").is_some());

    let membership_audit_id = Uuid::parse_str(&membership_audit_id).expect("membership audit UUID");
    let (status, membership_audit) = call(
        app.clone(),
        tenant_request(
            "GET",
            &format!("/v1/tenant/audit/{membership_audit_id}"),
            &admin_token,
            first_tenant_id,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{membership_audit}");
    assert_eq!(membership_audit["subject_user_id"], learner_id.to_string());
    assert_eq!(membership_audit["aggregate_type"], "tenant_membership");
    assert_eq!(membership_audit["action"], "tenant_membership.role_added");
    assert_eq!(membership_audit["payload"]["role"], "learner");

    let (status, isolated_audit) = call(
        app.clone(),
        tenant_request(
            "GET",
            &format!("/v1/tenant/audit/{first_audit_id}"),
            &admin_token,
            second_tenant_id,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{isolated_audit}");
    assert_eq!(isolated_audit["error"]["code"], "audit_event_not_found");
}

#[tokio::test]
async fn learner_goals_are_versioned_validated_isolated_and_reversible() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;
    let other_token = register_and_login(app.clone()).await;

    let (status, initial) = call(
        app.clone(),
        request("GET", "/v1/me/goals", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{initial}");
    assert_eq!(initial["version"], 0);
    assert!(initial["daily_minutes"].is_null());
    assert!(initial["exam_date"].is_null());
    assert_eq!(
        initial["protected_commitments"].as_array().unwrap().len(),
        0
    );
    assert_eq!(initial["can_undo"], false);

    let original = serde_json::json!({
        "expected_version": 0,
        "daily_minutes": 90,
        "exam_date": "2099-06-30",
        "protected_commitments": [
            {"title": "Night shift", "date": "2099-06-01"}
        ]
    });
    let (status, saved) = call(
        app.clone(),
        request("PUT", "/v1/me/goals", Some(&token), Some(original.clone())),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["version"], 1);
    assert_eq!(saved["daily_minutes"], 90);
    assert_eq!(saved["exam_date"], "2099-06-30");
    assert_eq!(saved["changed"], true);
    assert_eq!(saved["can_undo"], false);
    assert_eq!(saved["protected_commitments"][0]["title"], "Night shift");
    let created_at = saved["created_at"]
        .as_str()
        .expect("saved goal snapshot exposes audit timestamp")
        .to_owned();

    // Re-saving the same semantic snapshot must not manufacture history.
    let mut same = original.clone();
    same["expected_version"] = serde_json::json!(1);
    let (status, unchanged) = call(
        app.clone(),
        request("PUT", "/v1/me/goals", Some(&token), Some(same)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{unchanged}");
    assert_eq!(unchanged["version"], 1);
    assert_eq!(unchanged["changed"], false);
    assert_eq!(unchanged["created_at"], created_at);

    // Another learner still has an unconfigured profile.
    let (status, other) = call(
        app.clone(),
        request("GET", "/v1/me/goals", Some(&other_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{other}");
    assert_eq!(other["version"], 0);
    assert!(other["daily_minutes"].is_null());

    // Stale clients cannot overwrite the current snapshot.
    let (status, stale) = call(
        app.clone(),
        request(
            "PUT",
            "/v1/me/goals",
            Some(&token),
            Some(serde_json::json!({
                "expected_version": 0,
                "daily_minutes": 60,
                "exam_date": "2099-06-30",
                "protected_commitments": []
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{stale}");
    assert_eq!(stale["error"]["code"], "goal_version_conflict");

    for invalid in [
        serde_json::json!({
            "expected_version": 1,
            "daily_minutes": 0,
            "exam_date": "2099-06-30",
            "protected_commitments": []
        }),
        serde_json::json!({
            "expected_version": 1,
            "daily_minutes": 60,
            "exam_date": "2000-01-01",
            "protected_commitments": []
        }),
        serde_json::json!({
            "expected_version": 1,
            "daily_minutes": 60,
            "exam_date": "2099-06-30",
            "protected_commitments": [{"title": "   ", "date": "2099-06-01"}]
        }),
        serde_json::json!({
            "expected_version": 1,
            "daily_minutes": 60,
            "exam_date": "2099-06-30",
            "protected_commitments": [{"title": "Past commitment", "date": "2000-01-01"}]
        }),
    ] {
        let (status, invalid_body) = call(
            app.clone(),
            request("PUT", "/v1/me/goals", Some(&token), Some(invalid)),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{invalid_body}");
    }

    let (status, changed) = call(
        app.clone(),
        request(
            "PUT",
            "/v1/me/goals",
            Some(&token),
            Some(serde_json::json!({
                "expected_version": 1,
                "daily_minutes": 60,
                "exam_date": "2099-07-15",
                "protected_commitments": [
                    {"title": "Night shift", "date": "2099-06-01"},
                    {"title": "Protected course deadline", "date": "2099-06-20"}
                ]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    assert_eq!(changed["version"], 2);
    assert_eq!(changed["changed"], true);
    assert_eq!(changed["can_undo"], true);

    // Existing automatic planning may read constraints later, but it must not
    // silently rewrite this learner-owned record.
    let (status, today) = call(
        app.clone(),
        request("GET", "/v1/me/today", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{today}");
    let (status, after_planning) = call(
        app.clone(),
        request("GET", "/v1/me/goals", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{after_planning}");
    assert_eq!(after_planning["version"], 2);
    assert_eq!(after_planning["exam_date"], "2099-07-15");
    assert_eq!(
        after_planning["protected_commitments"][1]["title"],
        "Protected course deadline"
    );

    let (status, undone) = call(
        app.clone(),
        request(
            "POST",
            "/v1/me/goals/undo",
            Some(&token),
            Some(serde_json::json!({"expected_version": 2})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{undone}");
    assert_eq!(undone["version"], 3);
    assert_eq!(undone["daily_minutes"], 90);
    assert_eq!(undone["exam_date"], "2099-06-30");
    assert_eq!(undone["protected_commitments"].as_array().unwrap().len(), 1);
    assert_eq!(undone["can_undo"], true);
}

#[tokio::test]
async fn full_loop_cold_start_answer_submit_revision_undo() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // CORE-02: learner-owned constraints exist before automatic planning and
    // must survive both plan creation and evidence-driven plan revision.
    let (status, goals) = call(
        app.clone(),
        request(
            "PUT",
            "/v1/me/goals",
            Some(&token),
            Some(serde_json::json!({
                "expected_version": 0,
                "daily_minutes": 75,
                "exam_date": "2099-08-31",
                "protected_commitments": [
                    {"title": "Protected exam course", "date": "2099-08-01"}
                ]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{goals}");
    assert_eq!(goals["version"], 1);

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
    assert!(
        result["time_taken_seconds"]
            .as_i64()
            .is_some_and(|seconds| seconds >= 0),
        "submission time must be a non-negative server-derived duration: {result}"
    );
    assert_eq!(
        result["score"].as_i64().unwrap(),
        result["correct"].as_i64().unwrap() * 100 / result["total"].as_i64().unwrap(),
        "score remains the integer percentage"
    );
    assert_eq!(
        result["correct"].as_i64().unwrap()
            + result["incorrect"].as_i64().unwrap()
            + result["skipped"].as_i64().unwrap(),
        2
    );

    let (status, submitted_detail) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/practice/sessions/{sid}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{submitted_detail}");
    assert_eq!(submitted_detail["result"], result);

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

    let (status, goals_after_revision) = call(
        app.clone(),
        request("GET", "/v1/me/goals", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{goals_after_revision}");
    assert_eq!(goals_after_revision["version"], 1);
    assert_eq!(goals_after_revision["daily_minutes"], 75);
    assert_eq!(goals_after_revision["exam_date"], "2099-08-31");
    assert_eq!(
        goals_after_revision["protected_commitments"][0]["title"],
        "Protected exam course"
    );

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
async fn revision_session_includes_explicit_skips() {
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
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": ids.chapter1, "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid: Uuid = session["session_id"].as_str().unwrap().parse().unwrap();

    let (status, skipped) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(&token),
            Some(serde_json::json!({
                "item_index": 0, "idempotency_key": "skip-revision-contract"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{skipped}");
    assert!(skipped["correct"].is_null());

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
    assert_eq!(result["skipped"], 1);

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
    assert_eq!(
        status,
        StatusCode::OK,
        "an explicit skip is a missed question and must be revisable: {revision}"
    );
    assert_eq!(revision["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn qb12_builder_exposes_tree_filters_and_truthful_availability() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    let (status, builder) = call(
        app.clone(),
        request("GET", "/v1/practice/builder", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{builder}");
    let nodes = builder["nodes"].as_array().expect("builder nodes");
    assert!(
        nodes.len() >= 4,
        "subject/system/chapter tree is present: {builder}"
    );
    let chapter1 = nodes
        .iter()
        .find(|node| node["id"] == ids.chapter1.to_string())
        .expect("chapter1 node");
    assert_eq!(chapter1["available"], 2);
    assert_eq!(chapter1["attempted"], 0);
    assert_eq!(chapter1["unattempted"], 2);
    assert_eq!(builder["selection"]["all"], 4);
    assert_eq!(builder["selection"]["matching"], 4);

    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor",
                "chapter_ids": [ids.chapter1, ids.chapter2],
                "pool": "all",
                "difficulties": ["easy"],
                "high_yield": false,
                "question_count": 100
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    assert_eq!(session["requested_count"], 100);
    assert_eq!(session["available_count"], 2);
    assert_eq!(session["question_count"], 2);
    assert_eq!(session["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        session["availability_message"],
        "Only 2 questions are available for your current selection."
    );
    for item in session["items"].as_array().unwrap() {
        assert_eq!(item["difficulty"], "easy");
    }

    let (status, high_yield) = call(
        app.clone(),
        request(
            "GET",
            &format!(
                "/v1/practice/builder?chapter_ids={},{}&pool=all&difficulties=easy&high_yield=true",
                ids.chapter1, ids.chapter2
            ),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{high_yield}");
    assert_eq!(high_yield["selection"]["matching"], 1);
    assert_eq!(high_yield["selection"]["all"], 1);
}

#[tokio::test]
async fn qb12_pools_use_latest_attempt_and_marks_persist() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;

    // q1 is the only easy + high-yield question in chapter1. First answer it
    // incorrectly, then correctly in a later session: latest evidence wins.
    let mut previous_sid = None;
    for (n, chosen_index) in [(0, 1), (1, 0)] {
        let (status, session) = call(
            app.clone(),
            request(
                "POST",
                "/v1/practice/sessions",
                Some(&token),
                Some(serde_json::json!({
                    "preset": "tutor",
                    "chapter_ids": [ids.chapter1],
                    "pool": "all",
                    "difficulties": ["easy"],
                    "high_yield": true,
                    "question_count": 1,
                    "takeover": n > 0
                })),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{session}");
        let sid = Uuid::parse_str(session["session_id"].as_str().unwrap()).unwrap();
        assert_eq!(
            session["items"][0]["question_version_id"],
            ids.question_versions[0].to_string()
        );
        let (status, answer) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/practice/sessions/{sid}/answers"),
                Some(&token),
                Some(serde_json::json!({
                    "item_index": 0,
                    "chosen_index": chosen_index,
                    "idempotency_key": format!("qb12-latest-{n}")
                })),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        let (status, submitted) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/practice/sessions/{sid}/submit"),
                Some(&token),
                None,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{submitted}");
        previous_sid = Some(sid);
    }
    assert!(previous_sid.is_some());

    // q2 is the only medium question in chapter1; record an explicit skip.
    let (status, skipped_session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor",
                "chapter_ids": [ids.chapter1],
                "pool": "all",
                "difficulties": ["medium"],
                "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{skipped_session}");
    let skipped_sid = Uuid::parse_str(skipped_session["session_id"].as_str().unwrap()).unwrap();
    let (status, skipped) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{skipped_sid}/answers"),
            Some(&token),
            Some(serde_json::json!({
                "item_index": 0,
                "idempotency_key": "qb12-skip-latest"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{skipped}");
    let (status, submitted) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{skipped_sid}/submit"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{submitted}");

    let (status, missed) = call(
        app.clone(),
        request(
            "GET",
            &format!(
                "/v1/practice/builder?chapter_ids={}&pool=incorrect_skipped",
                ids.chapter1
            ),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{missed}");
    assert_eq!(missed["selection"]["incorrect_skipped"], 1);
    assert_eq!(missed["selection"]["matching"], 1);

    let (status, easy_missed) = call(
        app.clone(),
        request(
            "GET",
            &format!(
                "/v1/practice/builder?chapter_ids={}&pool=incorrect_skipped&difficulties=easy&high_yield=true",
                ids.chapter1
            ),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{easy_missed}");
    assert_eq!(
        easy_missed["selection"]["matching"], 0,
        "later correct evidence removes q1 from incorrect + skipped"
    );

    let (status, unattempted) = call(
        app.clone(),
        request(
            "GET",
            &format!(
                "/v1/practice/builder?chapter_ids={}&pool=unattempted",
                ids.chapter1
            ),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{unattempted}");
    assert_eq!(unattempted["selection"]["matching"], 0);

    let q3 = ids.question_versions[2];
    let (status, marked) = call(
        app.clone(),
        request(
            "PUT",
            &format!("/v1/questions/versions/{q3}/mark"),
            Some(&token),
            Some(serde_json::json!({"marked": true})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{marked}");
    assert_eq!(marked["marked"], true);

    let (status, marked_builder) = call(
        app.clone(),
        request(
            "GET",
            &format!(
                "/v1/practice/builder?chapter_ids={}&pool=marked",
                ids.chapter2
            ),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{marked_builder}");
    assert_eq!(marked_builder["selection"]["marked"], 1);
    assert_eq!(marked_builder["selection"]["matching"], 1);

    let (status, marked_session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor",
                "chapter_ids": [ids.chapter2],
                "pool": "marked",
                "question_count": 10
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{marked_session}");
    assert_eq!(marked_session["items"].as_array().unwrap().len(), 1);
    let marked_sid = marked_session["session_id"].as_str().unwrap();
    let (status, detail) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/practice/sessions/{marked_sid}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["items"][0]["marked"], true);

    let (status, unmarked) = call(
        app.clone(),
        request(
            "PUT",
            &format!("/v1/questions/versions/{q3}/mark"),
            Some(&token),
            Some(serde_json::json!({"marked": false})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{unmarked}");
    assert_eq!(unmarked["marked"], false);
}

#[tokio::test]
async fn core07_single_active_study_session_requires_explicit_takeover() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let token = register_and_login(app.clone()).await;
    let first_body = serde_json::json!({
        "preset": "tutor",
        "chapter_id": ids.chapter1,
        "question_count": 1
    });

    let (status, first) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(first_body.clone()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let first_id = first["session_id"].as_str().expect("session id");

    let (status, conflict) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(first_body.clone()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{conflict}");
    assert_eq!(conflict["error"]["code"], "active_study_session");
    assert_eq!(conflict["error"]["details"]["session_id"], first_id);

    let (status, replacement) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&token),
            Some(serde_json::json!({
                "preset": "tutor",
                "chapter_id": ids.chapter1,
                "question_count": 1,
                "takeover": true
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replacement}");
    assert_ne!(replacement["session_id"], first_id);

    let (status, old) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/practice/sessions/{first_id}"),
            Some(&token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{old}");
    assert_eq!(old["status"], "abandoned");
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
                                   "question_count": 1, "takeover": true}),
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
                                       "question_count": 2, "takeover": n > 0}),
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
