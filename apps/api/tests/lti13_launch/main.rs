//! INST-06: LTI 1.3 launch against a mocked campus LMS (1EdTech core
//! launch + deep linking). The mock platform serves the JWKS the tool
//! fetches, and the test plays the platform's browser: it follows the
//! login-initiation redirect, mints the signed id_token the platform would
//! form-post, and verifies the tool's deep-linking response against the
//! tool's own JWKS endpoint. API-only — no client surfaces involved.

mod lti_test_keys;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::util::ServiceExt;

use api::state::AppState;
use lti_test_keys::{TEST_PLATFORM_KEY_PEM, TEST_PLATFORM_PUBLIC_PEM, TEST_TOOL_KEY_PEM};

/// Mirrors integration.rs's setup with the LTI signing key provisioned.
async fn setup_lti() -> Arc<AppState> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/medos_ci".into());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect to test database");
    api::schema::apply_up(&pool).await.expect("apply schema");
    sqlx::query(
        "DO $$ DECLARE r RECORD; BEGIN FOR r IN (SELECT tablename FROM pg_tables WHERE schemaname = 'public') LOOP EXECUTE 'TRUNCATE TABLE public.' || quote_ident(r.tablename) || ' CASCADE'; END LOOP; END $$;",
    )
    .execute(&pool)
    .await
    .expect("clean data");

    Arc::new(AppState {
        pool,
        min_time_limit_seconds: 30,
        free_daily_questions: 10,
        free_mock_attempts: 3,
        free_analytics_drills: 2,
        free_daily_library: 3,
        community_min_sample: 2,
        admin_token: Some("test-admin".into()),
        free_daily_coach_turns: 20,
        openai_api_key: None,
        openai_base_url: "https://api.openai.com/v1".into(),
        pack_signing_key: Some("0123456789abcdef0123456789abcdef".into()),
        oidc_credential_key: Some("test-oidc-encryption-key-with-32-plus-chars".into()),
        public_api_base_url: "http://127.0.0.1:8080/api".into(),
        public_app_url: "http://127.0.0.1:5173".into(),
        zitadel: None,
        lti_tool_key: Some(TEST_TOOL_KEY_PEM.into()),
    })
}

async fn call(app: Router, req: Request<Body>) -> (StatusCode, Value) {
    let response = app.oneshot(req).await.expect("oneshot");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let value = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body)
            .unwrap_or(Value::String(String::from_utf8_lossy(&body).into_owned()))
    };
    (status, value)
}

async fn call_raw(app: Router, req: Request<Body>) -> axum::http::Response<Body> {
    app.oneshot(req).await.expect("oneshot")
}

fn form_request(uri: &str, pairs: &[(&str, &str)]) -> Request<Body> {
    let body: String = pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .expect("form request")
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("get request")
}

async fn register(app: &Router, email: &str) -> (StatusCode, Value, String) {
    let (status, body) = call(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/v1/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({"email": email, "password": "correct horse"}).to_string(),
            ))
            .expect("register"),
    )
    .await;
    let token = body["token"].as_str().unwrap_or_default().to_owned();
    (status, body, token)
}

/// The mocked platform: serves the JWKS the tool fetches at launch.
async fn spawn_mock_platform() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock LMS");
    let addr = listener.local_addr().expect("addr");
    use rsa::pkcs8::DecodePublicKey;
    use rsa::traits::PublicKeyParts;
    let public =
        rsa::RsaPublicKey::from_public_key_pem(TEST_PLATFORM_PUBLIC_PEM).expect("platform pub key");
    let n = URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public.e().to_bytes_be());
    let app = Router::new().route(
        "/jwks.json",
        get(move || async move {
            Json(json!({"keys": [{
                "kty": "RSA", "alg": "RS256", "use": "sig",
                "kid": "lms-key-1", "n": n, "e": e,
            }]}))
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock LMS");
    });
    format!("http://{addr}")
}

fn mint_id_token(
    nonce: &str,
    email: &str,
    message_type: &str,
    deep_link_settings: Option<Value>,
) -> String {
    let mut claims = json!({
        "iss": "https://lms.campus.test",
        "aud": "client-123",
        "sub": "lms-user-1",
        "exp": (chrono::Utc::now() + chrono::Duration::minutes(5)).timestamp(),
        "iat": chrono::Utc::now().timestamp(),
        "nonce": nonce,
        "email": email,
        "email_verified": true,
        "https://purl.imsglobal.org/spec/lti/claim/message_type": message_type,
        "https://purl.imsglobal.org/spec/lti/claim/version": "1.3.0",
        "https://purl.imsglobal.org/spec/lti/claim/deployment_id": "dep-1",
        "https://purl.imsglobal.org/spec/lti/claim/resource_link": {"id": "rl-1"},
        "https://purl.imsglobal.org/spec/lti/claim/roles":
            ["http://purl.imsglobal.org/vocab/lis/v2/membership#Learner"],
    });
    if let Some(settings) = deep_link_settings {
        claims["https://purl.imsglobal.org/spec/lti/claim/deep_linking_settings"] = settings;
    }
    let platform_key = jsonwebtoken::EncodingKey::from_rsa_pem(TEST_PLATFORM_KEY_PEM.as_bytes())
        .expect("platform key");
    let header = jsonwebtoken::Header {
        kid: Some("lms-key-1".into()),
        ..jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256)
    };
    jsonwebtoken::encode(&header, &claims, &platform_key).expect("encode id_token")
}

#[tokio::test]
async fn lti13_login_launch_deeplink_round_trip() {
    let state = setup_lti().await;
    let app = Router::new();
    let _ = app; // the real router carries the LTI routes
    let app = api::router(state.clone());
    api::seed::seed(&state.pool).await.expect("seed");

    let lms_base = spawn_mock_platform().await;

    // The institution's staff registers the campus LMS as a platform.
    let (status, staff_reg, staff_token) = register(&app, "lms-staff@example.test").await;
    assert_eq!(status, StatusCode::OK, "{staff_reg}");
    let staff_id: uuid::Uuid = staff_reg["user_id"].as_str().unwrap().parse().unwrap();
    let (status, inst) = call(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/v1/institutions")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {staff_token}"))
            .body(Body::from(
                json!({"name": "Campus with an LMS"}).to_string(),
            ))
            .expect("institution"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{inst}");
    let institution_id: uuid::Uuid = inst["institution_id"].as_str().unwrap().parse().unwrap();

    // The learner the LMS will launch: exists, member of the institution.
    let (status, learner_reg, _learner_token) = register(&app, "learner@lms.test").await;
    assert_eq!(status, StatusCode::OK, "{learner_reg}");
    let learner_id: uuid::Uuid = learner_reg["user_id"].as_str().unwrap().parse().unwrap();
    let (status, added) = call(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(&format!("/v1/institutions/{institution_id}/members"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {staff_token}"))
            .body(Body::from(
                json!({"user_id": learner_id, "role": "learner"}).to_string(),
            ))
            .expect("member"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{added}");

    let (status, platform) = call(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(&format!(
                "/v1/institutions/{institution_id}/interop/lti-platforms"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {staff_token}"))
            .body(Body::from(
                json!({
                    "issuer": "https://lms.campus.test",
                    "client_id": "client-123",
                    "deployment_id": "dep-1",
                    "auth_login_url": format!("{lms_base}/auth/login"),
                    "key_set_url": format!("{lms_base}/jwks.json"),
                    "display_name": "Mock campus LMS"
                })
                .to_string(),
            ))
            .expect("platform"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{platform}");

    // Third-party initiated login: 303 to the platform's auth URL with the
    // LTI OIDC parameters; state and nonce come back out of the Location.
    let response = call_raw(
        app.clone(),
        get_request(
            "/v1/lti/login?iss=https%3A%2F%2Flms.campus.test&login_hint=lms-user-1\
             &target_link_uri=http%3A%2F%2F127.0.0.1%3A5173%2Fsession&client_id=client-123",
        ),
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::SEE_OTHER,
        "login must redirect"
    );
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .expect("redirect location")
        .to_owned();
    assert!(
        location.starts_with(&format!("{lms_base}/auth/login?")),
        "{location}"
    );
    for expected in [
        "scope=openid",
        "response_type=id_token",
        "response_mode=form_post",
        "client_id=client-123",
    ] {
        assert!(location.contains(expected), "{expected} in {location}");
    }
    let query: Vec<(String, String)> = location
        .split('?')
        .nth(1)
        .expect("query")
        .split('&')
        .map(|pair| {
            let (k, v) = pair.split_once('=').expect("pair");
            (k.to_owned(), v.to_owned())
        })
        .collect();
    let state_value = query
        .iter()
        .find(|(k, _)| k == "state")
        .expect("state")
        .1
        .clone();
    let nonce = query
        .iter()
        .find(|(k, _)| k == "nonce")
        .expect("nonce")
        .1
        .clone();

    // The launch refuses a forged state before anything else happens.
    let id_token = mint_id_token(&nonce, "learner@lms.test", "LtiResourceLinkRequest", None);
    let (status, body) = call(
        app.clone(),
        form_request(
            "/v1/lti/launch",
            &[("id_token", &id_token), ("state", "forged-state")],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");

    // The real launch: verified email + institution membership links the
    // LMS subject once, then issues the shared app session.
    let (status, launched) = call(
        app.clone(),
        form_request(
            "/v1/lti/launch",
            &[("id_token", &id_token), ("state", &state_value)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{launched}");
    assert_eq!(
        launched["message_type"], "LtiResourceLinkRequest",
        "{launched}"
    );
    let session_token = launched["token"]
        .as_str()
        .expect("session token")
        .to_owned();
    assert_eq!(
        launched["user_id"].as_str().unwrap(),
        learner_id.to_string(),
        "linked to the member with the verified email"
    );

    // The issued session is a real app session.
    let (status, me) = call(
        app.clone(),
        Request::builder()
            .method("GET")
            .uri("/v1/me/today")
            .header(header::AUTHORIZATION, format!("Bearer {session_token}"))
            .body(Body::empty())
            .expect("me"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{me}");

    // Replay of the consumed state is refused.
    let (status, _) = call(
        app.clone(),
        form_request(
            "/v1/lti/launch",
            &[("id_token", &id_token), ("state", &state_value)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "state replay refused");

    // Tool JWKS serves the key the platform will verify us by.
    let (status, jwks) = call(app.clone(), get_request("/v1/lti/jwks.json")).await;
    assert_eq!(status, StatusCode::OK, "{jwks}");
    assert_eq!(jwks["keys"][0]["kty"], "RSA", "{jwks}");
    assert_eq!(jwks["keys"][0]["kid"], "medicalos-lti-1", "{jwks}");

    // Deep linking: launch as the staff user (linked by verified email),
    // then sign a response over chosen content items.
    let dl_nonce = "dl-nonce";
    let dl_token = mint_id_token(
        dl_nonce,
        "lms-staff@example.test",
        "LtiDeepLinkingRequest",
        Some(json!({
            "deep_link_return_url": format!("{lms_base}/deep-link-return"),
            "accept_types": ["ltiResourceLink"],
            "accept_presentation_document_targets": ["iframe"],
            "data": {"picker": "exams"}
        })),
    );
    // The staff launch needs its own login initiation (fresh state).
    let response = call_raw(
        app.clone(),
        get_request(
            "/v1/lti/login?iss=https%3A%2F%2Flms.campus.test&login_hint=lms-staff-1\
             &target_link_uri=http%3A%2F%2F127.0.0.1%3A5173%2Fcontent-picker&client_id=client-123",
        ),
    )
    .await;
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .expect("redirect location")
        .to_owned();
    let query: Vec<(String, String)> = location
        .split('?')
        .nth(1)
        .expect("query")
        .split('&')
        .map(|pair| {
            let (k, v) = pair.split_once('=').expect("pair");
            (k.to_owned(), v.to_owned())
        })
        .collect();
    let staff_state = query
        .iter()
        .find(|(k, _)| k == "state")
        .expect("state")
        .1
        .clone();
    let dl_id_token = mint_id_token(
        &query.iter().find(|(k, _)| k == "nonce").expect("nonce").1,
        "lms-staff@example.test",
        "LtiDeepLinkingRequest",
        Some(json!({
            "deep_link_return_url": format!("{lms_base}/deep-link-return"),
            "accept_types": ["ltiResourceLink"],
            "data": {"picker": "exams"}
        })),
    );
    let _ = dl_nonce;
    let (status, launched) = call(
        app.clone(),
        form_request(
            "/v1/lti/launch",
            &[("id_token", &dl_id_token), ("state", &staff_state)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{launched}");
    assert_eq!(
        launched["message_type"], "LtiDeepLinkingRequest",
        "{launched}"
    );

    let (status, response) = call(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/v1/lti/deep-links")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {}", launched["token"].as_str().unwrap()))
            .body(Body::from(
                json!({"items": [{"title": "Cardiology drill", "url": "https://medicalos.example/session?exam=1"}]})
                    .to_string(),
            ))
            .expect("deep links"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{response}");
    let response_jwt = response["jwt"].as_str().expect("response jwt");
    let post_url = response["post_url"].as_str().expect("post url");
    assert_eq!(
        post_url,
        format!("{lms_base}/deep-link-return"),
        "return URL came from the launch, not the request"
    );

    // The platform verifies our response against the tool's JWKS.
    let tool_jwks: Value = jwks;
    let header = jsonwebtoken::decode_header(response_jwt).expect("response header");
    assert_eq!(header.kid.as_deref(), Some("medicalos-lti-1"));
    let jwk_value = tool_jwks["keys"]
        .as_array()
        .and_then(|keys| {
            keys.iter()
                .find(|key| key["kid"] == json!(header.kid.as_deref().unwrap_or_default()))
        })
        .expect("tool jwk")
        .clone();
    let jwk: jsonwebtoken::jwk::Jwk = serde_json::from_value(jwk_value).expect("jwk");
    let key = jsonwebtoken::DecodingKey::from_jwk(&jwk).expect("decoding key");
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&["client-123"]);
    validation.validate_exp = true;
    let (_, claims) =
        jsonwebtoken::decode::<Value>(response_jwt, &key, &validation).expect("verify response");
    assert_eq!(
        claims["https://purl.imsglobal.org/spec/lti/claim/message_type"],
        "LtiDeepLinkingResponse"
    );
    assert_eq!(
        claims["https://purl.imsglobal.org/spec/lti/claim/data"]["picker"], "exams",
        "the platform's data echo comes back"
    );
    let items = claims["https://purl.imsglobal.org/spec/lti/claim/content_items"]
        .as_array()
        .expect("content items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["type"], "ltiResourceLink");
    assert_eq!(items[0]["title"], "Cardiology drill");
}
