use super::*;

fn lti_registration(display_name: &str, host: &str) -> Value {
    serde_json::json!({
        "issuer": "https://lms.example.test",
        "client_id": "shared-client",
        "deployment_id": "deployment-1",
        "auth_login_url": format!("https://{host}/auth"),
        "key_set_url": format!("https://{host}/jwks"),
        "display_name": display_name,
    })
}

async fn create_lti_tenant(app: &Router, token: &str, name: &str) -> Uuid {
    let (status, body) = call(
        app.clone(),
        request(
            "POST",
            "/v1/institutions",
            Some(token),
            Some(serde_json::json!({ "name": name })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["institution_id"]
        .as_str()
        .expect("institution id")
        .parse()
        .expect("valid institution id")
}

async fn register_lti_platform(
    app: &Router,
    token: &str,
    institution_id: Uuid,
    body: Value,
) -> (StatusCode, Value) {
    call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/institutions/{institution_id}/interop/lti-platforms"),
            Some(token),
            Some(body),
        ),
    )
    .await
}

#[tokio::test]
async fn lti_platform_registration_rejects_cross_tenant_reassignment() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let owner_token = register_and_login(app.clone()).await;
    let other_admin_token = register_and_login(app.clone()).await;
    let owner_institution = create_lti_tenant(&app, &owner_token, "LTI owner tenant").await;
    let other_institution = create_lti_tenant(&app, &other_admin_token, "Other LTI tenant").await;

    let (status, created) = register_lti_platform(
        &app,
        &owner_token,
        owner_institution,
        lti_registration("Owner initial", "lms.example.test"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let platform_id: Uuid = created["platform_id"]
        .as_str()
        .expect("platform id")
        .parse()
        .expect("valid platform id");

    let (status, updated) = register_lti_platform(
        &app,
        &owner_token,
        owner_institution,
        lti_registration("Owner updated", "updated-lms.example.test"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "same-tenant update: {updated}");
    assert_eq!(updated["platform_id"], platform_id.to_string());

    let (status, conflict) = register_lti_platform(
        &app,
        &other_admin_token,
        other_institution,
        lti_registration("Attacker update", "attacker.example.test"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{conflict}");
    assert_eq!(conflict["error"]["code"], "lti_platform_conflict");
    assert!(
        !conflict
            .to_string()
            .contains(&owner_institution.to_string()),
        "conflict response must not disclose the existing tenant: {conflict}"
    );
    assert!(
        !conflict
            .to_string()
            .contains(&other_institution.to_string()),
        "conflict response must not disclose tenant identifiers: {conflict}"
    );

    let (stored_institution, stored_name, stored_auth_url, stored_key_set_url) =
        sqlx::query_as::<_, (Uuid, String, String, String)>(
            "SELECT institution_id, display_name, auth_login_url, key_set_url
             FROM lti_platforms WHERE id = $1",
        )
        .bind(platform_id)
        .fetch_one(&state.pool)
        .await
        .expect("stored platform configuration");
    assert_eq!(stored_institution, owner_institution);
    assert_eq!(stored_name, "Owner updated");
    assert_eq!(stored_auth_url, "https://updated-lms.example.test/auth");
    assert_eq!(stored_key_set_url, "https://updated-lms.example.test/jwks");
}
