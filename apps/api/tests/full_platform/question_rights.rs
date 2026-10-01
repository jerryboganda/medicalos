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

async fn display_rights_active(
    pool: &sqlx::PgPool,
    rights_ref: Option<&str>,
    source_refs: &[&str],
    media_refs: &[&str],
) -> bool {
    let source_refs: Vec<String> = source_refs
        .iter()
        .map(|value| (*value).to_owned())
        .collect();
    let media_refs: Vec<String> = media_refs.iter().map(|value| (*value).to_owned()).collect();
    sqlx::query_scalar::<_, bool>("SELECT question_display_rights_active($1, $2, $3, $4)")
        .bind(rights_ref)
        .bind(SOURCE_REF)
        .bind(source_refs)
        .bind(media_refs)
        .fetch_one(pool)
        .await
        .expect("display-rights predicate")
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

async fn create_rights_chapter(pool: &sqlx::PgPool, exam_id: Uuid) -> Uuid {
    let parent_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM curriculum_nodes WHERE exam_id = $1 AND kind = 'system' LIMIT 1",
    )
    .bind(exam_id)
    .fetch_one(pool)
    .await
    .expect("fixture system");
    let chapter_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO curriculum_nodes (id, exam_id, kind, name, parent_id, display_order)
         VALUES ($1, $2, 'chapter', $3, $4, 99)",
    )
    .bind(chapter_id)
    .bind(exam_id)
    .bind(format!("Rights delivery fixture {chapter_id}"))
    .bind(parent_id)
    .execute(pool)
    .await
    .expect("isolated fixture chapter");
    chapter_id
}

async fn export_qti_at(
    app: &Router,
    token: &str,
    exam_id: Uuid,
    prefix: &str,
) -> (StatusCode, String) {
    call_text(
        app.clone(),
        admin_req(
            "GET",
            &format!("{prefix}/admin/qti/packages/{exam_id}"),
            Some(token),
            None,
        ),
    )
    .await
}

async fn export_qti(app: &Router, token: &str, exam_id: Uuid) -> (StatusCode, String) {
    export_qti_at(app, token, exam_id, "/api/v1").await
}

async fn assert_qti_denied_at(
    app: &Router,
    token: &str,
    exam_id: Uuid,
    prefix: &str,
    reason: &str,
) {
    let (status, body) = export_qti_at(app, token, exam_id, prefix).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{reason}: {body}");
    let body: Value = serde_json::from_str(&body).expect("QTI denial is JSON");
    assert_eq!(body["error"]["code"], "rights_unavailable", "{reason}");
}

async fn assert_qti_denied(app: &Router, token: &str, exam_id: Uuid, reason: &str) {
    assert_qti_denied_at(app, token, exam_id, "/api/v1", reason).await;
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
async fn learner_routes_stop_serving_question_content_after_rights_revoke_or_expire() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let learner = register_and_login(app.clone()).await;

    let parent_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM curriculum_nodes WHERE exam_id = $1 AND kind = 'system' LIMIT 1",
    )
    .bind(ids.exam_id)
    .fetch_one(&state.pool)
    .await
    .expect("fixture system");
    let chapter_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO curriculum_nodes (id, exam_id, kind, name, parent_id, display_order)
         VALUES ($1, $2, 'chapter', $3, $4, 99)",
    )
    .bind(chapter_id)
    .bind(ids.exam_id)
    .bind(format!("Rights fixture {}", Uuid::new_v4()))
    .bind(parent_id)
    .execute(&state.pool)
    .await
    .expect("isolated fixture chapter");

    let rights_ref = "QUESTION-RIGHTS-RUNTIME-REVOKE";
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
    let version_id = create_question(&app, &author, chapter_id, Some(rights_ref)).await;
    approve_question(&app, &author, &reviewer, version_id).await;
    let result = publish_result(&app, &reviewer, version_id).await;
    assert_eq!(result["status"], "published", "{result}");

    let expiring_ref = "QUESTION-RIGHTS-RUNTIME-EXPIRY";
    let expiring_rights_id = create_rights(
        &app,
        &author,
        expiring_ref,
        &["display", "derivatives"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    let expiring_version_id = create_question(&app, &author, chapter_id, Some(expiring_ref)).await;
    approve_question(&app, &author, &reviewer, expiring_version_id).await;
    let expiring_result = publish_result(&app, &reviewer, expiring_version_id).await;
    assert_eq!(expiring_result["status"], "published", "{expiring_result}");

    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&learner),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": chapter_id, "question_count": 2
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "active-rights session: {session}");
    let session_id = session["session_id"].as_str().expect("session id");
    let items = session["items"].as_array().expect("session items");
    assert_eq!(items.len(), 2, "{session}");
    let item_index = items
        .iter()
        .position(|item| item["question_version_id"] == serde_json::json!(version_id))
        .expect("revocable question item");
    let expiring_item_index = items
        .iter()
        .position(|item| item["question_version_id"] == serde_json::json!(expiring_version_id))
        .expect("expiring question item");

    let answer_key = "rights-revocation-answer-replay";
    let (status, answer) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{session_id}/answers"),
            Some(&learner),
            Some(serde_json::json!({
                "item_index": item_index,
                "chosen_index": 1,
                "idempotency_key": answer_key
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "active-rights answer: {answer}");

    let coach_key = "rights-revocation-coach-replay";
    let (status, coach) = call(
        app.clone(),
        coach_req(
            &learner,
            Some(version_id),
            "explain",
            "Explain the key point.",
            coach_key,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "active-rights Coach turn: {coach}");

    let (status, revoked) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/content-rights/{rights_id}/revoke"),
            Some(&author),
            Some(serde_json::json!({"reason": "Synthetic fixture grant ended"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "revoke rights: {revoked}");
    sqlx::query("UPDATE content_rights SET valid_to = CURRENT_DATE - 1 WHERE id = $1")
        .bind(expiring_rights_id)
        .execute(&state.pool)
        .await
        .expect("expire second grant");

    let (status, detail) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/practice/sessions/{session_id}"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "revoked session detail: {detail}"
    );
    assert_eq!(detail["error"]["code"], "rights_unavailable", "{detail}");
    assert!(!detail
        .to_string()
        .contains("Synthetic rights fixture vignette"));

    let (status, replay) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{session_id}/answers"),
            Some(&learner),
            Some(serde_json::json!({
                "item_index": item_index,
                "chosen_index": 1,
                "idempotency_key": answer_key
            })),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "revoked answer replay: {replay}"
    );
    assert_eq!(replay["error"]["code"], "rights_unavailable", "{replay}");

    let (status, sync) = call(
        app.clone(),
        request(
            "POST",
            "/v1/sync/events",
            Some(&learner),
            Some(serde_json::json!({
                "events": [{
                    "event_id": "revoked-question-answer-sync",
                    "kind": "answer",
                    "payload": {
                        "session_id": session_id,
                        "item_index": item_index,
                        "chosen_index": 1,
                        "idempotency_key": answer_key
                    }
                }]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "offline sync batch: {sync}");
    assert_eq!(
        sync["results"][0]["status"], "rejected",
        "revoked offline answer replay: {sync}"
    );
    assert_eq!(sync["results"][0]["code"], "rights_unavailable", "{sync}");

    let (status, hint) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/practice/sessions/{session_id}/items/{expiring_item_index}/hint"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "revoked hint: {hint}");
    assert_eq!(hint["error"]["code"], "rights_unavailable", "{hint}");

    let (status, coach_replay) = call(
        app.clone(),
        coach_req(
            &learner,
            Some(version_id),
            "explain",
            "Explain the key point.",
            coach_key,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "revoked Coach replay: {coach_replay}"
    );
    assert_eq!(
        coach_replay["error"]["code"], "rights_unavailable",
        "{coach_replay}"
    );

    let (status, history) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/coach/history?question_version_id={version_id}"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "revoked Coach history: {history}"
    );
    assert_eq!(history["error"]["code"], "rights_unavailable", "{history}");

    let (status, new_session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&learner),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": chapter_id, "question_count": 2
            })),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "new session: {new_session}"
    );
    assert_eq!(new_session["error"]["code"], "empty_pool", "{new_session}");

    let (status, submitted) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{session_id}/submit"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "aggregate submission survives revoke: {submitted}"
    );
    assert_eq!(submitted["total"], 2, "{submitted}");
    assert!(!submitted
        .to_string()
        .contains("Synthetic rights fixture vignette"));

    for action in ["retry", "practice_incorrect"] {
        let (status, result) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/practice/sessions/{session_id}/action"),
                Some(&learner),
                Some(serde_json::json!({"action": action})),
            ),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{action}: {result}"
        );
        assert!(
            matches!(
                result["error"]["code"].as_str(),
                Some("empty_pool" | "nothing_to_practice")
            ),
            "{action}: {result}"
        );
    }
}

#[tokio::test]
async fn question_display_rights_fail_closed_for_expiry_scope_and_seat_limits() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    seed::seed(&state.pool).await.expect("seed");
    let admin = register_and_login(app.clone()).await;

    let full_sources = [SOURCE_REF, SOURCE_REFS[1]];
    let full_media = [MEDIA_REFS[0]];
    let active_ref = "DISPLAY-RIGHTS-ACTIVE";
    create_rights(
        &app,
        &admin,
        active_ref,
        &["display"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &[],
        None,
        None,
    )
    .await;
    assert!(display_rights_active(&state.pool, Some(active_ref), &full_sources, &full_media).await);

    let expired_ref = "DISPLAY-RIGHTS-EXPIRED";
    create_rights(
        &app,
        &admin,
        expired_ref,
        &["display"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        Some("2020-01-01"),
        None,
    )
    .await;
    assert!(
        !display_rights_active(&state.pool, Some(expired_ref), &full_sources, &full_media).await
    );

    let audience_ref = "DISPLAY-RIGHTS-AUDIENCE";
    create_rights(
        &app,
        &admin,
        audience_ref,
        &["display"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["instructors"],
        None,
        None,
    )
    .await;
    assert!(
        !display_rights_active(&state.pool, Some(audience_ref), &full_sources, &full_media).await
    );

    let seat_ref = "DISPLAY-RIGHTS-SEATS";
    create_rights(
        &app,
        &admin,
        seat_ref,
        &["display"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        Some(50),
    )
    .await;
    assert!(!display_rights_active(&state.pool, Some(seat_ref), &full_sources, &full_media).await);

    let wrong_use_ref = "DISPLAY-RIGHTS-WRONG-USE";
    create_rights(
        &app,
        &admin,
        wrong_use_ref,
        &["derivatives"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    assert!(
        !display_rights_active(&state.pool, Some(wrong_use_ref), &full_sources, &full_media).await
    );

    let missing_source_ref = "DISPLAY-RIGHTS-MISSING-SOURCE";
    create_rights(
        &app,
        &admin,
        missing_source_ref,
        &["display"],
        &[SOURCE_REF, MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    assert!(
        !display_rights_active(
            &state.pool,
            Some(missing_source_ref),
            &full_sources,
            &full_media
        )
        .await
    );

    let missing_media_ref = "DISPLAY-RIGHTS-MISSING-MEDIA";
    create_rights(
        &app,
        &admin,
        missing_media_ref,
        &["display"],
        &[SOURCE_REF, SOURCE_REFS[1]],
        &["learners"],
        None,
        None,
    )
    .await;
    assert!(
        !display_rights_active(
            &state.pool,
            Some(missing_media_ref),
            &full_sources,
            &full_media
        )
        .await
    );

    assert!(!display_rights_active(&state.pool, None, &full_sources, &full_media).await);
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

#[tokio::test]
async fn assessment_launch_rechecks_reserved_question_display_rights() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let learner = register_and_login(app.clone()).await;
    let chapter_id = create_rights_chapter(&state.pool, ids.exam_id).await;

    let rights_ref = "ASSESSMENT-RIGHTS-REVOKE";
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
    let version_id = create_question(&app, &author, chapter_id, Some(rights_ref)).await;
    approve_question(&app, &author, &reviewer, version_id).await;
    assert_eq!(
        publish_result(&app, &reviewer, version_id).await["status"],
        "published"
    );

    let (status, spec) = call(
        app.clone(),
        admin_req(
            "POST",
            &format!("/v1/admin/exams/{}/specs", ids.exam_id),
            Some(&author),
            Some(serde_json::json!({
                "effective_from": chrono::Utc::now().date_naive().to_string(),
                "config": {
                    "blocks": 2,
                    "block_seconds": 3600,
                    "break_seconds": 600,
                    "grace_seconds": 30
                }
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create exam spec: {spec}");
    let spec_id = spec["spec_id"].as_str().expect("spec id");
    let (status, form) = call(
        app.clone(),
        admin_req(
            "POST",
            &format!("/v1/admin/exam-specs/{spec_id}/forms"),
            Some(&author),
            Some(serde_json::json!({
                "name": "Rights runtime form",
                "assessment_family": "pilot",
                "blueprint": {"chapters": []},
                "reserved": true,
                "ai_allowed": false,
                "question_ids": [version_id]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create reserved form: {form}");
    let form_id = form["form_id"].as_str().expect("form id");

    let (status, started) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/assessments/{form_id}/sessions"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "active assessment launch: {started}"
    );
    let session_id = started["session_id"].as_str().expect("session id");
    let (status, detail) = call(
        app.clone(),
        request(
            "GET",
            &format!("/v1/practice/sessions/{session_id}"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "active assessment detail: {detail}");
    assert_eq!(
        detail["items"][0]["question_version_id"],
        version_id.to_string()
    );

    let (status, revoked) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/content-rights/{rights_id}/revoke"),
            Some(&author),
            Some(serde_json::json!({"reason": "Assessment fixture license ended"})),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "revoke assessment rights: {revoked}"
    );
    let (status, denied) = call(
        app.clone(),
        request(
            "POST",
            &format!("/api/v1/assessments/{form_id}/sessions"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "revoked assessment launch: {denied}"
    );
    assert_eq!(denied["error"]["code"], "rights_unavailable", "{denied}");
}

#[tokio::test]
async fn mock_pool_excludes_revoked_expired_and_out_of_scope_questions() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let learner = register_and_login(app.clone()).await;
    let chapter_id = create_rights_chapter(&state.pool, ids.exam_id).await;
    let rights_refs = [
        "MOCK-RIGHTS-REVOKE",
        "MOCK-RIGHTS-EXPIRED",
        "MOCK-RIGHTS-AUDIENCE",
        "MOCK-RIGHTS-SEATS",
        "MOCK-RIGHTS-MEDIA-SCOPE",
        "MOCK-RIGHTS-SOURCE-SCOPE",
    ];
    let mut rights_ids = Vec::with_capacity(rights_refs.len());
    for rights_ref in rights_refs {
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
        let version_id = create_question(&app, &author, chapter_id, Some(rights_ref)).await;
        approve_question(&app, &author, &reviewer, version_id).await;
        assert_eq!(
            publish_result(&app, &reviewer, version_id).await["status"],
            "published"
        );
        rights_ids.push(rights_id);
    }

    let (status, mock) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/mocks",
            Some(&author),
            Some(serde_json::json!({
                "title": "Rights runtime mock",
                "exam_id": ids.exam_id,
                "mock_type": "mini",
                "blueprint": [{"chapter_id": chapter_id, "count": rights_refs.len()}],
                "pass_mark_percent": 50,
                "attempts_allowed": 2
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create mock: {mock}");
    let mock_id = mock["mock_id"].as_str().expect("mock id");
    let (status, started) = call(
        app.clone(),
        request(
            "POST",
            &format!("/api/v1/mocks/{mock_id}/start"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "active mock start: {started}");
    assert_eq!(started["question_count"], rights_refs.len());

    let (status, revoked) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/content-rights/{}/revoke", rights_ids[0]),
            Some(&author),
            Some(serde_json::json!({"reason": "Mock fixture license ended"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "revoke mock rights: {revoked}");
    sqlx::query("UPDATE content_rights SET valid_to = CURRENT_DATE - 1 WHERE id = $1")
        .bind(rights_ids[1])
        .execute(&state.pool)
        .await
        .expect("expire mock grant");
    sqlx::query("UPDATE content_rights SET audiences = $2 WHERE id = $1")
        .bind(rights_ids[2])
        .bind(serde_json::json!(["instructors"]))
        .execute(&state.pool)
        .await
        .expect("restrict mock audience");
    sqlx::query("UPDATE content_rights SET seat_limit = 25 WHERE id = $1")
        .bind(rights_ids[3])
        .execute(&state.pool)
        .await
        .expect("seat-limit mock grant");
    sqlx::query("UPDATE content_rights SET asset_refs = $2 WHERE id = $1")
        .bind(rights_ids[4])
        .bind(serde_json::json!([SOURCE_REF, SOURCE_REFS[1]]))
        .execute(&state.pool)
        .await
        .expect("remove mock media scope");
    sqlx::query("UPDATE content_rights SET asset_refs = $2 WHERE id = $1")
        .bind(rights_ids[5])
        .bind(serde_json::json!([SOURCE_REF, MEDIA_REFS[0]]))
        .execute(&state.pool)
        .await
        .expect("remove mock source scope");

    let (status, denied) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/mocks/{mock_id}/start"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "ineligible mock pool: {denied}"
    );
    assert_eq!(
        denied["error"]["code"], "insufficient_questions",
        "{denied}"
    );
}

#[tokio::test]
async fn guest_trial_does_not_return_a_question_after_its_rights_expire() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let synthetic_rights_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM content_rights WHERE ref_code = 'MEDICALOS-SYNTHETIC-SEED'",
    )
    .fetch_one(&state.pool)
    .await
    .expect("synthetic seed grant");
    let (status, revoked) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/content-rights/{synthetic_rights_id}/revoke"),
            Some(&author),
            Some(serde_json::json!({"reason": "Isolate guest rights fixture"})),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "revoke synthetic fixture grant: {revoked}"
    );

    let chapter_id = create_rights_chapter(&state.pool, ids.exam_id).await;
    let rights_ref = "GUEST-RIGHTS-EXPIRY";
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
    let version_id = create_question(&app, &author, chapter_id, Some(rights_ref)).await;
    approve_question(&app, &author, &reviewer, version_id).await;
    assert_eq!(
        publish_result(&app, &reviewer, version_id).await["status"],
        "published"
    );

    let guest_key = format!("guest-rights-{}", Uuid::new_v4());
    let (status, started) = call(
        app.clone(),
        request(
            "POST",
            "/guest/trial/start",
            None,
            Some(serde_json::json!({"guest_key": guest_key})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "guest trial start: {started}");
    let (status, question) = call(
        app.clone(),
        request(
            "POST",
            "/guest/trial/next-question",
            None,
            Some(serde_json::json!({"guest_key": guest_key})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "active guest content: {question}");

    sqlx::query("UPDATE content_rights SET valid_to = CURRENT_DATE - 1 WHERE id = $1")
        .bind(rights_id)
        .execute(&state.pool)
        .await
        .expect("expire guest grant");
    let (status, denied) = call(
        app.clone(),
        request(
            "POST",
            "/guest/trial/next-question",
            None,
            Some(serde_json::json!({"guest_key": guest_key})),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "expired guest pool: {denied}"
    );
    assert_eq!(denied["error"]["code"], "empty_pool", "{denied}");
}

#[tokio::test]
async fn content_rights_accept_explicit_distribution_use() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let author = register_and_login(app.clone()).await;

    create_rights(
        &app,
        &author,
        "QTI-DISTRIBUTION-API",
        &["display", "distribution"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
}

#[tokio::test]
async fn qti_export_requires_current_distribution_rights_for_every_question() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let chapter_id = create_rights_chapter(&state.pool, ids.exam_id).await;
    let rights_ref = "QTI-DISTRIBUTION-RIGHTS";
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
    let version_id = create_question(&app, &author, chapter_id, Some(rights_ref)).await;
    approve_question(&app, &author, &reviewer, version_id).await;
    assert_eq!(
        publish_result(&app, &reviewer, version_id).await["status"],
        "published"
    );

    assert_qti_denied(
        &app,
        &reviewer,
        ids.exam_id,
        "display permission cannot export an answer key",
    )
    .await;
    assert_qti_denied_at(
        &app,
        &reviewer,
        ids.exam_id,
        "/v1",
        "v1 alias display permission cannot export an answer key",
    )
    .await;

    sqlx::query("UPDATE content_rights SET permitted_uses = $2 WHERE id = $1")
        .bind(rights_id)
        .bind(serde_json::json!([
            "display",
            "derivatives",
            "distribution"
        ]))
        .execute(&state.pool)
        .await
        .expect("grant explicit distribution use");
    let (status, package) = export_qti(&app, &reviewer, ids.exam_id).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "active distribution grant: {package}"
    );
    assert!(package.contains(&version_id.to_string()), "{package}");
    assert!(package.contains("<correctResponse>"), "{package}");
    let (alias_status, alias_package) = export_qti_at(&app, &reviewer, ids.exam_id, "/v1").await;
    assert_eq!(
        alias_status,
        StatusCode::OK,
        "active distribution grant through v1 alias: {alias_package}"
    );
    assert!(alias_package.contains(&version_id.to_string()), "{alias_package}");
    assert!(alias_package.contains("<correctResponse>"), "{alias_package}");

    sqlx::query("UPDATE content_rights SET valid_to = CURRENT_DATE - 1 WHERE id = $1")
        .bind(rights_id)
        .execute(&state.pool)
        .await
        .expect("expire distribution grant");
    assert_qti_denied(&app, &reviewer, ids.exam_id, "expired distribution grant").await;

    sqlx::query("UPDATE content_rights SET valid_to = NULL, audiences = $2 WHERE id = $1")
        .bind(rights_id)
        .bind(serde_json::json!(["instructors"]))
        .execute(&state.pool)
        .await
        .expect("restrict distribution audience");
    assert_qti_denied(&app, &reviewer, ids.exam_id, "wrong distribution audience").await;

    sqlx::query("UPDATE content_rights SET audiences = $2, seat_limit = 25 WHERE id = $1")
        .bind(rights_id)
        .bind(serde_json::json!(["learners"]))
        .execute(&state.pool)
        .await
        .expect("set unsupported distribution seat limit");
    assert_qti_denied(
        &app,
        &reviewer,
        ids.exam_id,
        "seat-limited distribution grant",
    )
    .await;

    sqlx::query("UPDATE content_rights SET seat_limit = NULL, asset_refs = $2 WHERE id = $1")
        .bind(rights_id)
        .bind(serde_json::json!([SOURCE_REF, SOURCE_REFS[1]]))
        .execute(&state.pool)
        .await
        .expect("remove distribution media scope");
    assert_qti_denied(
        &app,
        &reviewer,
        ids.exam_id,
        "missing distribution media scope",
    )
    .await;

    sqlx::query("UPDATE content_rights SET asset_refs = $2 WHERE id = $1")
        .bind(rights_id)
        .bind(serde_json::json!([SOURCE_REF, MEDIA_REFS[0]]))
        .execute(&state.pool)
        .await
        .expect("remove distribution source scope");
    assert_qti_denied(
        &app,
        &reviewer,
        ids.exam_id,
        "missing distribution source scope",
    )
    .await;

    let (status, revoked) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/content-rights/{rights_id}/revoke"),
            Some(&author),
            Some(serde_json::json!({"reason": "QTI fixture license ended"})),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "revoke distribution grant: {revoked}"
    );
    assert_qti_denied(&app, &reviewer, ids.exam_id, "revoked distribution grant").await;
}
