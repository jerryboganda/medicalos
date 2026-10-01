use super::*;

async fn create_password_account(app: &Router) -> String {
    let email = format!("device-session-{}@example.test", Uuid::new_v4());
    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/register",
            None,
            Some(serde_json::json!({"email": email, "password": "longenough"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "register: {body}");
    email
}

async fn login_token(app: &Router, email: &str) -> String {
    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            "/v1/auth/login",
            None,
            Some(serde_json::json!({"email": email, "password": "longenough"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login: {body}");
    body["token"].as_str().expect("login token").to_string()
}

async fn register_device(app: &Router, token: &str, key: &str) -> (StatusCode, Value) {
    call(
        app.clone(),
        request(
            "POST",
            "/v1/me/devices",
            Some(token),
            Some(serde_json::json!({"device_key": key, "label": key})),
        ),
    )
    .await
}

#[tokio::test]
async fn session_policy_failure_preserves_policy_and_existing_access() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    seed::seed(&state.pool).await.expect("seed");
    let email = create_password_account(&app).await;
    let first = login_token(&app, &email).await;
    let second = login_token(&app, &email).await;
    let first_hash = api::auth::sha256_hex(&first);
    sqlx::query(&format!(
        "ALTER TABLE auth_sessions ADD CONSTRAINT reject_fixture_session_retirement
         CHECK (token_hash <> '{first_hash}' OR revoked_at IS NULL) NOT VALID"
    ))
    .execute(&state.pool)
    .await
    .expect("inject owned-session retirement failure");
    let (status, result) = call(
        app.clone(),
        request(
            "PATCH",
            "/v1/me/session-policy",
            Some(&first),
            Some(serde_json::json!({"single_active_session": true})),
        ),
    )
    .await;
    sqlx::query("ALTER TABLE auth_sessions DROP CONSTRAINT reject_fixture_session_retirement")
        .execute(&state.pool)
        .await
        .expect("remove session failure fixture");
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{result}");
    // A fresh login must not retire either prior session after a failed enable.
    let fresh = login_token(&app, &email).await;
    for token in [&first, &second, &fresh] {
        let (status, body) = call(
            app.clone(),
            request("GET", "/v1/me/today", Some(token), None),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "failed enable must preserve access: {body}"
        );
    }
}

#[tokio::test]
async fn device_revocation_retires_every_bound_bearer_and_blocks_pack_opens() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");

    let email = create_password_account(&app).await;
    let device_a_token = login_token(&app, &email).await;
    let same_device_login_token = login_token(&app, &email).await;
    let device_b_token = login_token(&app, &email).await;
    let (status, device_a) = register_device(&app, &device_a_token, "session-device-a").await;
    assert_eq!(status, StatusCode::OK, "device A: {device_a}");
    let device_a_id = device_a["device_id"]
        .as_str()
        .expect("device A id")
        .to_string();
    let (status, same_device) = register_device(&app, &device_a_token, "session-device-a").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "same bearer re-register: {same_device}"
    );
    assert_eq!(same_device["device_id"], device_a["device_id"]);

    let (status, switch) = register_device(&app, &device_a_token, "session-device-c").await;
    assert_eq!(status, StatusCode::CONFLICT, "session switch: {switch}");
    assert_eq!(switch["error"]["code"], "device_session_conflict");

    // A fresh password login can register the same owned device. Its bearer
    // shares the device binding so one device revocation retires both.
    let (status, same_device) =
        register_device(&app, &same_device_login_token, "session-device-a").await;
    assert_eq!(status, StatusCode::OK, "same-device login: {same_device}");
    assert_eq!(same_device["device_id"], device_a["device_id"]);
    let (status, device_b) = register_device(&app, &device_b_token, "session-device-b").await;
    assert_eq!(status, StatusCode::OK, "device B: {device_b}");
    let legacy_unbound_token = login_token(&app, &email).await;

    // Fill the account's configured device slots through the public API. The
    // default cap is five, but stop on the API's own limit response.
    let mut filler_index = 0;
    loop {
        assert!(filler_index < 20, "device limit was not enforced");
        let key = format!("session-device-filler-{filler_index}");
        let token = login_token(&app, &email).await;
        let (status, filler) = register_device(&app, &token, &key).await;
        if status == StatusCode::FORBIDDEN {
            assert_eq!(filler["error"]["code"], "devices_exhausted", "{filler}");
            break;
        }
        assert_eq!(status, StatusCode::OK, "fill device slot: {filler}");
        filler_index += 1;
    }

    let outsider = register_and_login(app.clone()).await;
    let (status, refusal) = call(
        app.clone(),
        request(
            "DELETE",
            &format!("/v1/me/devices/{device_a_id}"),
            Some(&outsider),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "cross-user revoke: {refusal}"
    );

    // A valid bearer reaches the pack manifest and receives the ordinary
    // lease refusal before revocation.
    let manifest_uri = format!(
        "/v2/packs/{}/manifest?chapters={}&device_id=session-device-a",
        ids.exam_id, ids.chapter3
    );
    let (status, before_revoke) = call(
        app.clone(),
        request("GET", &manifest_uri, Some(&device_a_token), None),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "manifest before revoke: {before_revoke}"
    );
    assert_eq!(before_revoke["error"]["code"], "pack_lease_required");

    let (status, revoked) = call(
        app.clone(),
        request(
            "DELETE",
            &format!("/v1/me/devices/{device_a_id}"),
            Some(&device_b_token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "revoke: {revoked}");
    assert_eq!(revoked["revoked"], true);

    let (status, old_key_registration) =
        register_device(&app, &legacy_unbound_token, "session-device-a").await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "pre-revocation bearer cannot reactivate the key: {old_key_registration}"
    );
    assert_eq!(old_key_registration["error"]["code"], "device_revoked");

    // Fix the timestamp boundary deterministically: a session no newer than
    // the revocation must fail closed, including an equal transaction time.
    sqlx::query!(
        "UPDATE auth_sessions SET created_at =
         (SELECT revoked_at FROM user_devices WHERE id = $1)
         WHERE token_hash = $2",
        device_a_id.parse::<Uuid>().unwrap(),
        api::auth::sha256_hex(&legacy_unbound_token)
    )
    .execute(&state.pool)
    .await
    .expect("equal revocation timestamp fixture");
    let (status, same_timestamp) =
        register_device(&app, &legacy_unbound_token, "session-device-a").await;
    assert_eq!(status, StatusCode::CONFLICT, "{same_timestamp}");
    assert_eq!(same_timestamp["error"]["code"], "device_revoked");

    let (status, _) = call(
        app.clone(),
        request("GET", "/v1/me/today", Some(&device_a_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "revoked bearer");
    let (status, _) = call(
        app.clone(),
        request("GET", "/v1/me/today", Some(&same_device_login_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "second revoked bearer");
    let (status, revoked_manifest) = call(
        app.clone(),
        request("GET", &manifest_uri, Some(&device_a_token), None),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "revoked pack manifest: {revoked_manifest}"
    );
    let (status, _) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v2/packs/{}/resources", ids.exam_id),
            Some(&device_a_token),
            Some(serde_json::json!({
                "device_id": "session-device-a",
                "chapters": [ids.chapter3],
                "question_version_ids": []
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "revoked pack open");

    let (status, other_device) = call(
        app.clone(),
        request("GET", "/v1/me/today", Some(&device_b_token), None),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "other device remains active: {other_device}"
    );

    let (status, revoked_devices) = call(
        app.clone(),
        request("GET", "/v1/me/devices", Some(&device_b_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "device list: {revoked_devices}");
    let listed_devices = revoked_devices["devices"].as_array().unwrap();
    assert!(listed_devices
        .iter()
        .any(|device| { device["device_id"] == device_a_id && device["revoked_at"].is_string() }));
    assert!(listed_devices.iter().any(|device| {
        device["device_id"] == device_b["device_id"] && device["revoked_at"].is_null()
    }));

    let (status, repeated) = call(
        app.clone(),
        request(
            "DELETE",
            &format!("/v1/me/devices/{device_a_id}"),
            Some(&device_b_token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "repeat revoke: {repeated}");

    // Revocation frees one device slot; filling it again makes reactivation
    // fail at the configured limit rather than exceeding the cap.
    let replacement_token = login_token(&app, &email).await;
    let (status, replacement) =
        register_device(&app, &replacement_token, "session-device-replacement").await;
    assert_eq!(status, StatusCode::OK, "fill freed slot: {replacement}");

    let later_token = login_token(&app, &email).await;
    let (status, at_limit) = register_device(&app, &later_token, "session-device-a").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "reactivation at limit: {at_limit}"
    );
    assert_eq!(at_limit["error"]["code"], "devices_exhausted");

    // After another device is revoked, a post-revocation login can explicitly
    // re-register the old key. Its former bearers remain revoked.
    let (status, freed) = call(
        app.clone(),
        request(
            "DELETE",
            &format!(
                "/v1/me/devices/{}",
                replacement["device_id"].as_str().unwrap()
            ),
            Some(&device_b_token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "free device slot: {freed}");
    let (status, reactivated) = register_device(&app, &later_token, "session-device-a").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "revoked key re-registration after freeing a slot: {reactivated}"
    );
    assert_eq!(reactivated["device_id"], device_a_id);
    let (status, _) = call(
        app.clone(),
        request("GET", "/v1/me/today", Some(&later_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "fresh re-registered session");
    let (status, _) = call(
        app.clone(),
        request("GET", "/v1/me/today", Some(&device_a_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "old bearer stays revoked");

    let (status, devices) = call(
        app,
        request("GET", "/v1/me/devices", Some(&later_token), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "reactivated device list: {devices}");
    assert!(devices["devices"]
        .as_array()
        .unwrap()
        .iter()
        .any(|device| device["device_id"] == device_a_id && device["revoked_at"].is_null()));
}

#[tokio::test]
async fn coach_reports_the_extractive_adapter_even_when_an_openai_key_is_configured() {
    let _guard = LOCK.lock().await;
    let mut state = setup().await;
    Arc::get_mut(&mut state)
        .expect("unique app state before router creation")
        .openai_api_key = Some("test-key-without-a-provider-call".into());
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let email = create_password_account(&app).await;
    let token = login_token(&app, &email).await;

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
    assert_eq!(status, StatusCode::OK, "practice session: {session}");
    let session_id = session["session_id"].as_str().expect("session id");
    let (status, answer) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{session_id}/answers"),
            Some(&token),
            Some(serde_json::json!({
                "item_index": 0,
                "chosen_index": 0,
                "idempotency_key": "honest-coach-adapter-answer"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "answer question: {answer}");
    let version_id: Uuid = session["items"][0]["question_version_id"]
        .as_str()
        .expect("question version id")
        .parse()
        .expect("question version UUID");

    let (status, turn) = call(
        app,
        coach_req(
            &token,
            Some(version_id),
            "explain",
            "Explain this simply.",
            "honest-coach-adapter-turn",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "coach turn: {turn}");
    assert_eq!(turn["adapter"], "extractive");
    assert_eq!(turn["model"], "reviewed-content");
    assert!(turn["answer"]
        .as_str()
        .unwrap()
        .contains("Key learning point:"));
}
