//! INST-06: LTI 1.3 launch against a mocked campus LMS (1EdTech core
//! launch + deep linking). The test injects the platform JWKS through a
//! trusted in-process transport and plays the platform's browser: it follows the
//! login-initiation redirect, mints the signed id_token the platform would
//! form-post, and verifies the tool's deep-linking response against the
//! tool's own JWKS endpoint. It also covers the launch route's browser HTML
//! handoff; client surfaces are covered by Playwright.

mod lti_test_keys;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::util::ServiceExt;

use api::error::{ApiError, ApiResult};
use api::state::{AppState, LtiJwksFuture, LtiJwksTransport};
use lti_test_keys::{TEST_PLATFORM_KEY_PEM, TEST_PLATFORM_PUBLIC_PEM, TEST_TOOL_KEY_PEM};

/// Mirrors integration.rs's setup with the LTI signing key provisioned.
async fn setup_lti(jwks_transport: Arc<dyn LtiJwksTransport>) -> Arc<AppState> {
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
        lti_jwks_transport: jwks_transport,
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

async fn bind_device(app: &Router, token: &str, device_key: &str) {
    let (status, body) = call(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/v1/me/devices")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(json!({"device_key": device_key}).to_string()))
            .expect("register test device"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

fn query_param(location: &str, key: &str) -> String {
    location
        .split_once('?')
        .and_then(|(_, query)| {
            query.split('&').find_map(|pair| {
                let (name, value) = pair.split_once('=')?;
                (name == key).then(|| value.to_owned())
            })
        })
        .unwrap_or_else(|| panic!("missing {key} in {location}"))
}

/// Registration answers with the account; the token comes from a login.
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
    let (login_status, login) = call(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/v1/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({"email": email, "password": "correct horse"}).to_string(),
            ))
            .expect("login"),
    )
    .await;
    assert_eq!(login_status, StatusCode::OK, "{login}");
    let token = login["token"].as_str().unwrap_or_default().to_owned();
    bind_device(app, &token, "lti-test-device").await;
    (status, body, token)
}

fn mock_platform_jwks() -> Value {
    use rsa::pkcs8::DecodePublicKey;
    use rsa::traits::PublicKeyParts;
    let public =
        rsa::RsaPublicKey::from_public_key_pem(TEST_PLATFORM_PUBLIC_PEM).expect("platform pub key");
    let n = URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public.e().to_bytes_be());
    json!({"keys": [{
        "kty": "RSA", "alg": "RS256", "use": "sig",
        "kid": "lms-key-1", "n": n, "e": e,
    }]})
}

struct MockJwksTransport {
    allowed_url: String,
    jwks: Value,
}

impl MockJwksTransport {
    fn validate_configured_url(&self, raw: &str) -> ApiResult<()> {
        if raw == self.allowed_url {
            Ok(())
        } else {
            Err(ApiError::unprocessable(
                "invalid_key_set_url",
                "the mock platform transport only accepts its configured JWKS URL",
            ))
        }
    }
}

impl LtiJwksTransport for MockJwksTransport {
    fn validate_key_set_url(&self, raw: &str) -> ApiResult<()> {
        self.validate_configured_url(raw)
    }

    fn fetch_jwks<'a>(&'a self, raw: &'a str) -> LtiJwksFuture<'a> {
        Box::pin(async move {
            self.validate_configured_url(raw)?;
            serde_json::from_value(self.jwks.clone())
                .map_err(|_| ApiError::unprocessable("invalid_key_set", "the mock JWKS is invalid"))
        })
    }
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
    let lms_base = "http://lms.example.test".to_string();
    let jwks_transport = Arc::new(MockJwksTransport {
        allowed_url: format!("{lms_base}/jwks.json"),
        jwks: mock_platform_jwks(),
    });
    let state = setup_lti(jwks_transport).await;
    let app = api::router(state.clone());
    api::seed::seed(&state.pool).await.expect("seed");

    // The institution's staff registers the campus LMS as a platform.
    let (status, staff_reg, staff_token) = register(&app, "lms-staff@example.test").await;
    assert_eq!(status, StatusCode::OK, "{staff_reg}");
    let _staff_id: uuid::Uuid = staff_reg["user_id"].as_str().unwrap().parse().unwrap();
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
            .uri(format!("/v1/institutions/{institution_id}/members").as_str())
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
            .uri(format!(
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
    let platform_id: uuid::Uuid = platform["platform_id"]
        .as_str()
        .expect("registered platform id")
        .parse()
        .expect("platform id is a UUID");

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
    let state_value = query_param(&location, "state");
    let nonce = query_param(&location, "nonce");

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
    bind_device(&app, &session_token, "lti-test-device").await;
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

    // A browser form_post negotiates the same verified launch as an HTML
    // handoff document, which stores the session in the framed app origin.
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
        "browser login redirects"
    );
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .expect("browser login redirect");
    let browser_state = query_param(location, "state");
    let browser_nonce = query_param(location, "nonce");
    let browser_id_token = mint_id_token(
        &browser_nonce,
        "learner@lms.test",
        "LtiResourceLinkRequest",
        None,
    );
    let mut browser_request = form_request(
        "/v1/lti/launch",
        &[("id_token", &browser_id_token), ("state", &browser_state)],
    );
    browser_request
        .headers_mut()
        .insert(header::ACCEPT, "text/html".parse().expect("accept header"));
    let browser_response = call_raw(app.clone(), browser_request).await;
    assert_eq!(browser_response.status(), StatusCode::OK);
    assert_eq!(
        browser_response.headers()[header::CONTENT_TYPE],
        "text/html; charset=utf-8"
    );
    assert_eq!(
        browser_response.headers()[header::CACHE_CONTROL],
        "no-store"
    );
    assert!(browser_response.headers()[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .expect("CSP header")
        .contains("frame-ancestors https:"));
    let browser_html = axum::body::to_bytes(browser_response.into_body(), 1024 * 1024)
        .await
        .expect("browser handoff body");
    let browser_html = String::from_utf8(browser_html.to_vec()).expect("handoff is UTF-8");
    assert!(browser_html.contains("localStorage.setItem('mlos_token'"));
    assert!(browser_html.contains("/lti/handoff"));
    assert!(!browser_html.contains("/lti/handoff?token="));

    // Tool JWKS serves the key the platform will verify us by.
    let (status, jwks) = call(app.clone(), get_request("/v1/lti/jwks.json")).await;
    assert_eq!(status, StatusCode::OK, "{jwks}");
    assert_eq!(jwks["keys"][0]["kty"], "RSA", "{jwks}");
    assert_eq!(jwks["keys"][0]["kid"], "medicalos-lti-1", "{jwks}");

    // Deep linking: launch as the staff user (linked by verified email),
    // then sign a response over chosen content items.
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
    let staff_state = query_param(&location, "state");
    let dl_id_token = mint_id_token(
        &query_param(&location, "nonce"),
        "lms-staff@example.test",
        "LtiDeepLinkingRequest",
        Some(json!({
            "deep_link_return_url": format!("{lms_base}/deep-link-return"),
            "accept_types": ["ltiResourceLink"],
            "data": {"picker": "exams"}
        })),
    );
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
    let deep_link_token = launched["token"]
        .as_str()
        .expect("deep-link session token")
        .to_owned();
    bind_device(&app, &deep_link_token, "lti-test-device").await;

    let (status, response) = call(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/v1/lti/deep-links")
            .header(header::CONTENT_TYPE, "application/json")
            .header(
                header::AUTHORIZATION,
                format!("Bearer {deep_link_token}"),
            )
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
    let claims = jsonwebtoken::decode::<Value>(response_jwt, &key, &validation)
        .expect("verify response")
        .claims;
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

    // An instructor may cancel a deep-link picker. The tool still signs an
    // empty content_items array and returns it to the server-saved LMS URL.
    sqlx::query(
        "INSERT INTO lti_deep_link_pends (user_id, platform_id, settings, expires_at)
         VALUES ($1, $2, $3, now() + interval '5 minutes')
         ON CONFLICT (user_id) DO UPDATE SET
            platform_id = $2, settings = $3, expires_at = now() + interval '5 minutes'",
    )
    .bind(_staff_id)
    .bind(platform_id)
    .bind(json!({
        "deep_link_return_url": format!("{lms_base}/deep-link-return"),
        "accept_types": ["ltiResourceLink"],
        "data": {"picker": "cancel"}
    }))
    .execute(&state.pool)
    .await
    .expect("park cancellation settings");
    let (status, empty_response) = call(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/v1/lti/deep-links")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {staff_token}"))
            .body(Body::from(json!({"items": []}).to_string()))
            .expect("empty deep-link response"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{empty_response}");
    let expected_return_url = format!("{lms_base}/deep-link-return");
    assert_eq!(
        empty_response["post_url"].as_str(),
        Some(expected_return_url.as_str())
    );
    let empty_claims = jsonwebtoken::decode::<Value>(
        empty_response["jwt"].as_str().expect("empty response JWT"),
        &key,
        &validation,
    )
    .expect("verify signed cancellation response")
    .claims;
    assert_eq!(
        empty_claims["https://purl.imsglobal.org/spec/lti/claim/message_type"],
        "LtiDeepLinkingResponse"
    );
    assert_eq!(
        empty_claims["https://purl.imsglobal.org/spec/lti/claim/content_items"],
        json!([])
    );
    assert_eq!(
        empty_claims["https://purl.imsglobal.org/spec/lti/claim/data"]["picker"],
        "cancel"
    );
}
