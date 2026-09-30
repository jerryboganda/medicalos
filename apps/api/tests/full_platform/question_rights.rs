use super::*;

const SOURCE_REF: &str = "Synthetic question source";
const SOURCE_REFS: [&str; 2] = [SOURCE_REF, "Synthetic question appendix"];
const MEDIA_REFS: [&str; 1] = ["synthetic-question-image"];

fn question_body(chapter_id: Uuid, rights_ref: Option<&str>) -> Value {
    let mut body = serde_json::json!({
        "chapter_id": chapter_id,
        "difficulty": "medium",
        "vignette": "Synthetic rights fixture vignette.",
        "lead_in": "Which synthetic answer applies?",
        "options": [
            {"text": "First", "rationale": "Synthetic first rationale."},
            {"text": "Second", "rationale": "Synthetic second rationale."}
        ],
        "correct_index": 0,
        "key_learning_point": "Synthetic rights fixtures stay within scope.",
        "source_ref": SOURCE_REF,
        "source_refs": SOURCE_REFS,
        "media_refs": MEDIA_REFS
    });
    if let Some(rights_ref) = rights_ref {
        body["rights_ref"] = serde_json::json!(rights_ref);
    }
    body
}

async fn create_question(
    app: &Router,
    author: &str,
    chapter_id: Uuid,
    rights_ref: Option<&str>,
) -> Uuid {
    let (status, body) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/admin/questions",
            Some(author),
            Some(question_body(chapter_id, rights_ref)),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "draft creation: {body}");
    body["version_id"].as_str().unwrap().parse().unwrap()
}

#[allow(clippy::too_many_arguments)] // Fixture mirrors the rights API fields.
async fn create_rights(
    app: &Router,
    token: &str,
    ref_code: &str,
    permitted_uses: &[&str],
    asset_refs: &[&str],
    audiences: &[&str],
    valid_to: Option<&str>,
    seat_limit: Option<i32>,
) -> Uuid {
    let (status, body) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/admin/content-rights",
            Some(token),
            Some(serde_json::json!({
                "ref_code": ref_code,
                "licensor": "Synthetic question fixture",
                "territory": "worldwide",
                "permitted_uses": permitted_uses,
                "valid_from": "2020-01-01",
                "valid_to": valid_to,
                "asset_refs": asset_refs,
                "audiences": audiences,
                "seat_limit": seat_limit
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "rights fixture {ref_code}: {body}");
    body["rights_id"].as_str().unwrap().parse().unwrap()
}

async fn approve_question(app: &Router, author: &str, reviewer: &str, version_id: Uuid) {
    for (token, action) in [(author, "submit"), (reviewer, "approve")] {
        let (status, body) = workflow(app.clone(), token, action, [version_id]).await;
        assert_eq!(status, StatusCode::OK, "{action}: {body}");
        assert_eq!(
            body["results"][0]["status"],
            if action == "submit" {
                "in_review"
            } else {
                "approved"
            },
            "{body}"
        );
    }
}

async fn publish_result(app: &Router, reviewer: &str, version_id: Uuid) -> Value {
    let (status, body) = workflow(app.clone(), reviewer, "publish", [version_id]).await;
    assert_eq!(status, StatusCode::OK, "publish response: {body}");
    body["results"][0].clone()
}

#[tokio::test]
async fn question_publication_denies_missing_revoked_expired_and_wrong_use_rights() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;

    let missing = create_question(&app, &author, ids.chapter1, None).await;

    let expired_ref = "QUESTION-RIGHTS-EXPIRED";
    create_rights(
        &app,
        &author,
        expired_ref,
        &["display", "derivatives"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        Some("2020-01-01"),
        None,
    )
    .await;
    let expired = create_question(&app, &author, ids.chapter1, Some(expired_ref)).await;

    let revoked_ref = "QUESTION-RIGHTS-REVOKED";
    let revoked_id = create_rights(
        &app,
        &author,
        revoked_ref,
        &["display", "derivatives"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    let (status, body) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/content-rights/{revoked_id}/revoke"),
            Some(&author),
            Some(serde_json::json!({"reason": "Synthetic fixture revocation"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "revoke fixture: {body}");
    let revoked = create_question(&app, &author, ids.chapter1, Some(revoked_ref)).await;

    let wrong_use_ref = "QUESTION-RIGHTS-WRONG-USE";
    create_rights(
        &app,
        &author,
        wrong_use_ref,
        &["document_extraction"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    let wrong_use = create_question(&app, &author, ids.chapter1, Some(wrong_use_ref)).await;

    for (version_id, expected_code) in [
        (missing, "rights_ref_required"),
        (revoked, "rights_unavailable"),
        (expired, "rights_unavailable"),
        (wrong_use, "rights_use_not_permitted"),
    ] {
        approve_question(&app, &author, &reviewer, version_id).await;
        let result = publish_result(&app, &reviewer, version_id).await;
        assert_eq!(result["error"]["code"], expected_code, "{result}");
    }
}

#[tokio::test]
async fn question_publication_rejects_out_of_scope_assets_and_nonlearner_audiences() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;

    let out_of_scope_ref = "QUESTION-RIGHTS-INCOMPLETE-ASSETS";
    create_rights(
        &app,
        &author,
        out_of_scope_ref,
        &["display", "derivatives"],
        &[SOURCE_REF, SOURCE_REFS[1]],
        &["learners"],
        None,
        None,
    )
    .await;
    let out_of_scope = create_question(&app, &author, ids.chapter1, Some(out_of_scope_ref)).await;

    let wrong_audience_ref = "QUESTION-RIGHTS-WRONG-AUDIENCE";
    create_rights(
        &app,
        &author,
        wrong_audience_ref,
        &["display", "derivatives"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["instructors"],
        None,
        None,
    )
    .await;
    let wrong_audience =
        create_question(&app, &author, ids.chapter1, Some(wrong_audience_ref)).await;

    let capped_ref = "QUESTION-RIGHTS-SEAT-LIMITED";
    create_rights(
        &app,
        &author,
        capped_ref,
        &["display", "derivatives"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        Some(10),
    )
    .await;
    let seat_limited = create_question(&app, &author, ids.chapter1, Some(capped_ref)).await;

    for (version_id, expected_code) in [
        (out_of_scope, "rights_asset_scope_incomplete"),
        (wrong_audience, "rights_audience_not_permitted"),
        (seat_limited, "rights_seat_limited"),
    ] {
        approve_question(&app, &author, &reviewer, version_id).await;
        let result = publish_result(&app, &reviewer, version_id).await;
        assert_eq!(result["error"]["code"], expected_code, "{result}");
    }
}

#[tokio::test]
async fn question_publication_accepts_current_scoped_learner_rights_for_an_independent_reviewer() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    assert_ne!(author, reviewer);

    let rights_ref = "QUESTION-RIGHTS-VALID";
    create_rights(
        &app,
        &author,
        rights_ref,
        &["display"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    let version_id = create_question(&app, &author, ids.chapter1, Some(rights_ref)).await;
    approve_question(&app, &author, &reviewer, version_id).await;

    let result = publish_result(&app, &reviewer, version_id).await;
    assert_eq!(result["status"], "published", "{result}");
}

#[tokio::test]
async fn question_publication_requires_content_publish_permission() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;

    let rights_ref = "QUESTION-RIGHTS-OWNER-PUBLISH";
    create_rights(
        &app,
        &author,
        rights_ref,
        &["display"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    let version_id = create_question(&app, &author, ids.chapter1, Some(rights_ref)).await;
    approve_question(&app, &author, &reviewer, version_id).await;

    let medical_reviewer = role_session_with(&state, &app, &["medical_reviewer"], true).await;
    let (status, denied) = call(
        app.clone(),
        request(
            "POST",
            "/v1/admin/assessment-workflow",
            Some(&medical_reviewer),
            Some(serde_json::json!({"action":"publish","version_ids":[version_id]})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{denied}");
    assert_eq!(denied["error"]["code"], "permission_required", "{denied}");

    let owner = role_session_with(&state, &app, &["platform_owner"], true).await;
    let (status, published) = call(
        app,
        request(
            "POST",
            "/v1/admin/assessment-workflow",
            Some(&owner),
            Some(serde_json::json!({"action":"publish","version_ids":[version_id]})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{published}");
    assert_eq!(
        published["results"][0]["status"], "published",
        "{published}"
    );
}

#[tokio::test]
async fn question_submitter_cannot_replace_original_author_for_review_or_publication() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let (author_id, author) = register(app.clone(), "question-original-author".into()).await;
    let (_, submitter) = register(app.clone(), "question-submitter".into()).await;
    let (_, reviewer) = register(app.clone(), "question-independent-reviewer".into()).await;

    let rights_ref = "QUESTION-RIGHTS-AUTHOR-PROVENANCE";
    create_rights(
        &app,
        &author,
        rights_ref,
        &["display"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    let version_id = create_question(&app, &author, ids.chapter1, Some(rights_ref)).await;

    let (status, submitted) = workflow(app.clone(), &submitter, "submit", [version_id]).await;
    assert_eq!(status, StatusCode::OK, "{submitted}");
    assert_eq!(
        submitted["results"][0]["status"], "in_review",
        "{submitted}"
    );
    let recorded_author = sqlx::query_scalar!(
        r#"SELECT created_by AS "created_by!" FROM question_versions WHERE id = $1"#,
        version_id
    )
    .fetch_one(&state.pool)
    .await
    .expect("recorded question author");
    assert_eq!(recorded_author, author_id);

    let (status, self_approval) = workflow(app.clone(), &author, "approve", [version_id]).await;
    assert_eq!(status, StatusCode::OK, "{self_approval}");
    assert_eq!(
        self_approval["results"][0]["error"]["code"], "separation_violation",
        "{self_approval}"
    );

    let (status, approved) = workflow(app.clone(), &reviewer, "approve", [version_id]).await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    assert_eq!(approved["results"][0]["status"], "approved", "{approved}");

    let (status, self_publication) = workflow(app, &author, "publish", [version_id]).await;
    assert_eq!(status, StatusCode::OK, "{self_publication}");
    assert_eq!(
        self_publication["results"][0]["error"]["code"], "separation_violation",
        "{self_publication}"
    );
}

#[tokio::test]
async fn question_workflow_fails_closed_when_legacy_author_is_unknown() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let version_id = create_question(&app, &author, ids.chapter1, None).await;

    sqlx::query!(
        "UPDATE question_versions SET created_by = NULL WHERE id = $1",
        version_id
    )
    .execute(&state.pool)
    .await
    .expect("model legacy draft with unknown author");

    let (status, response) = workflow(app.clone(), &author, "submit", [version_id]).await;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(
        response["results"][0]["error"]["code"], "author_provenance_required",
        "{response}"
    );
    let question_status = sqlx::query_scalar!(
        r#"SELECT status AS "status!" FROM question_versions WHERE id = $1"#,
        version_id
    )
    .fetch_one(&state.pool)
    .await
    .expect("legacy draft status");
    assert_eq!(question_status, "draft");
}

#[tokio::test]
async fn question_submission_rolls_back_when_its_audit_cannot_be_written() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let version = create_question(&app, &author, ids.chapter1, None).await;
    sqlx::query(
        "ALTER TABLE audit_events ADD CONSTRAINT reject_fixture_submit_audit
         CHECK (action <> 'assessment_submitted') NOT VALID",
    )
    .execute(&state.pool)
    .await
    .expect("inject audit write failure");
    let (status, result) = workflow(app.clone(), &author, "submit", [version]).await;
    sqlx::query("ALTER TABLE audit_events DROP CONSTRAINT reject_fixture_submit_audit")
        .execute(&state.pool)
        .await
        .expect("remove audit failure fixture");
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(
        result["results"][0]["error"]["code"], "internal",
        "{result}"
    );
    let (status, search) = call(
        app,
        admin_req("GET", "/v1/admin/questions", Some(&author), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{search}");
    let item = search["questions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["version_id"] == serde_json::json!(version))
        .expect("draft remains visible");
    assert_eq!(
        item["status"], "draft",
        "failed audit must not submit the draft: {search}"
    );
}

#[tokio::test]
async fn question_review_rolls_back_when_its_audit_cannot_be_written() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    for action in ["approve", "reject"] {
        let version = create_question(&app, &author, ids.chapter1, None).await;
        let (_, submitted) = workflow(app.clone(), &author, "submit", [version]).await;
        assert_eq!(submitted["results"][0]["status"], "in_review");
        sqlx::query(
            "ALTER TABLE audit_events ADD CONSTRAINT reject_fixture_review_audit
             CHECK (action NOT IN ('assessment_approved', 'assessment_rejected')) NOT VALID",
        )
        .execute(&state.pool)
        .await
        .expect("inject review audit failure");
        let (status, result) = workflow(app.clone(), &reviewer, action, [version]).await;
        sqlx::query("ALTER TABLE audit_events DROP CONSTRAINT reject_fixture_review_audit")
            .execute(&state.pool)
            .await
            .expect("remove review audit fixture");
        assert_eq!(status, StatusCode::OK, "{result}");
        assert_eq!(
            result["results"][0]["error"]["code"], "internal",
            "{result}"
        );
        let (_, search) = call(
            app.clone(),
            admin_req("GET", "/v1/admin/questions", Some(&author), None),
        )
        .await;
        let item = search["questions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["version_id"] == serde_json::json!(version))
            .expect("in-review item remains visible");
        assert_eq!(item["status"], "in_review", "{search}");
        let review_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM assessment_reviews WHERE question_version_id = $1",
        )
        .bind(version)
        .fetch_one(&state.pool)
        .await
        .expect("inspect review receipt after rollback");
        assert_eq!(review_count, 0, "failed {action} must not retain a review");
    }
}

#[tokio::test]
async fn question_publication_requires_recorded_independent_review() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let publisher = register_and_login(app.clone()).await;
    for author_review in [false, true] {
        let version = create_question(&app, &author, ids.chapter1, None).await;
        sqlx::query(
            "UPDATE question_versions SET status = 'approved',
             reviewed_by = CASE WHEN $2 THEN created_by ELSE NULL END WHERE id = $1",
        )
        .bind(version)
        .bind(author_review)
        .execute(&state.pool)
        .await
        .expect("model legacy approval without independent provenance");
        let result = publish_result(&app, &publisher, version).await;
        assert_eq!(
            result["error"]["code"], "independent_review_required",
            "{result}"
        );
    }
}

#[tokio::test]
async fn question_competing_reviews_commit_only_one_decision() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let version = create_question(&app, &author, ids.chapter1, None).await;
    let (_, submitted) = workflow(app.clone(), &author, "submit", [version]).await;
    assert_eq!(submitted["results"][0]["status"], "in_review");
    let mut held = state
        .pool
        .begin()
        .await
        .expect("hold question before decisions");
    sqlx::query("SELECT id FROM question_versions WHERE id = $1 FOR UPDATE")
        .bind(version)
        .fetch_one(&mut *held)
        .await
        .expect("lock question fixture");
    let approve_app = app.clone();
    let approve_token = reviewer.clone();
    let approve =
        tokio::spawn(
            async move { workflow(approve_app, &approve_token, "approve", [version]).await },
        );
    wait_for_blocked_statements(&state.pool, "FROM question_versions", 1).await;
    let reject = tokio::spawn(async move { workflow(app, &reviewer, "reject", [version]).await });
    wait_for_blocked_statements(&state.pool, "FROM question_versions", 2).await;
    held.commit()
        .await
        .expect("release question for competing reviews");
    let results = [
        approve.await.expect("approve task"),
        reject.await.expect("reject task"),
    ];
    assert_eq!(
        results
            .iter()
            .filter(|(_, body)| body["results"][0]["status"].is_string())
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|(_, body)| body["results"][0]["error"]["code"] == "invalid_transition")
            .count(),
        1,
        "{results:?}"
    );
    let review_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assessment_reviews WHERE question_version_id = $1",
    )
    .bind(version)
    .fetch_one(&state.pool)
    .await
    .expect("committed review count");
    assert_eq!(review_count, 1);
}

async fn wait_for_blocked_statements(pool: &sqlx::PgPool, fragment: &str, expected: i64) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let blocked: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM pg_stat_activity
                   WHERE wait_event_type = 'Lock' AND query ILIKE $1
               "#,
        )
        .bind(format!("%{fragment}%"))
        .fetch_one(pool)
        .await
        .expect("inspect test lock wait");
        if blocked >= expected {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "expected {expected} statements waiting on the fixture lock: {fragment}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn question_publication_waits_for_concurrent_rights_revocation() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;

    let rights_ref = "QUESTION-RIGHTS-CONCURRENT-REVOKE";
    let rights_id = create_rights(
        &app,
        &author,
        rights_ref,
        &["display", "derivatives"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    let version_id = create_question(&app, &author, ids.chapter1, Some(rights_ref)).await;
    approve_question(&app, &author, &reviewer, version_id).await;

    let mut revoke_first = state.pool.begin().await.expect("begin lock-order fixture");
    sqlx::query!(
        "SELECT id FROM content_rights WHERE id = $1 FOR UPDATE",
        rights_id
    )
    .fetch_one(&mut *revoke_first)
    .await
    .expect("lock grant row");

    let revoke_app = app.clone();
    let revoke_token = author.clone();
    let revoke = tokio::spawn(async move {
        call(
            revoke_app,
            admin_req(
                "PATCH",
                &format!("/v1/admin/content-rights/{rights_id}/revoke"),
                Some(&revoke_token),
                Some(serde_json::json!({"reason": "Concurrent synthetic revocation"})),
            ),
        )
        .await
    });
    wait_for_blocked_statements(&state.pool, "UPDATE content_rights", 1).await;

    let publish_app = app.clone();
    let publish_token = reviewer.clone();
    let publish = tokio::spawn(async move {
        workflow(publish_app, &publish_token, "publish", [version_id]).await
    });
    wait_for_blocked_statements(&state.pool, "FROM content_rights", 1).await;

    revoke_first
        .commit()
        .await
        .expect("commit revocation first");

    let (revoke_status, revoke_body) = revoke.await.expect("revocation task");
    assert_eq!(revoke_status, StatusCode::OK, "revoke: {revoke_body}");
    let (publish_status, publish_body) = publish.await.expect("publication task");
    assert_eq!(publish_status, StatusCode::OK, "publish: {publish_body}");
    assert_eq!(
        publish_body["results"][0]["error"]["code"], "rights_unavailable",
        "revocation committed before publication: {publish_body}"
    );
}
