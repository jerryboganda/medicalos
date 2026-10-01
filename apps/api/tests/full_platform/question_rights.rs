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
    assert!(
        alias_package.contains(&version_id.to_string()),
        "{alias_package}"
    );
    assert!(
        alias_package.contains("<correctResponse>"),
        "{alias_package}"
    );

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

#[tokio::test]
async fn qotd_routes_withhold_ineligible_content_and_keep_the_daily_pick_stable() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let learner = register_and_login(app.clone()).await;

    let (status, settings) = call(
        app.clone(),
        request(
            "PUT",
            "/v1/me/engagement/settings",
            Some(&learner),
            Some(serde_json::json!({"qotd_exam_id": ids.exam_id})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "QOTD settings: {settings}");

    let qotd_routes = ["/v1/qotd", "/v1/me/qotd", "/api/v1/qotd", "/api/v1/me/qotd"];
    let mut selected_id = None;
    for route in qotd_routes {
        let (status, qotd) = call(app.clone(), request("GET", route, Some(&learner), None)).await;
        assert_eq!(status, StatusCode::OK, "active QOTD {route}: {qotd}");
        assert_eq!(qotd["available"], true, "active QOTD {route}: {qotd}");
        let id = qotd["question_version_id"]
            .as_str()
            .expect("active QOTD version")
            .to_owned();
        assert!(qotd["vignette"].is_string(), "active QOTD {route}: {qotd}");
        assert!(qotd["options"].is_array(), "active QOTD {route}: {qotd}");
        if let Some(expected) = &selected_id {
            assert_eq!(&id, expected, "shared QOTD at {route}");
        } else {
            selected_id = Some(id);
        }
    }
    let selected_id: Uuid = selected_id
        .as_deref()
        .expect("daily QOTD")
        .parse()
        .expect("QOTD UUID");
    let rights_ref: String =
        sqlx::query_scalar("SELECT rights_ref FROM question_versions WHERE id = $1")
            .bind(selected_id)
            .fetch_one(&state.pool)
            .await
            .expect("selected question rights ref");
    let rights_id: Uuid = sqlx::query_scalar("SELECT id FROM content_rights WHERE ref_code = $1")
        .bind(&rights_ref)
        .fetch_one(&state.pool)
        .await
        .expect("selected question rights grant");
    let baseline_assets: Value =
        sqlx::query_scalar("SELECT asset_refs FROM content_rights WHERE id = $1")
            .bind(rights_id)
            .fetch_one(&state.pool)
            .await
            .expect("synthetic grant assets");

    for (state_name, mutation) in [
        (
            "expired",
            "UPDATE content_rights SET valid_to = CURRENT_DATE - 1 WHERE id = $1",
        ),
        (
            "wrong audience",
            "UPDATE content_rights SET audiences = '[\"instructors\"]'::jsonb WHERE id = $1",
        ),
        (
            "unsupported seat limit",
            "UPDATE content_rights SET seat_limit = 25 WHERE id = $1",
        ),
        (
            "incomplete asset scope",
            "UPDATE content_rights SET asset_refs = '[]'::jsonb WHERE id = $1",
        ),
    ] {
        sqlx::query(mutation)
            .bind(rights_id)
            .execute(&state.pool)
            .await
            .unwrap_or_else(|error| panic!("set {state_name} QOTD grant: {error}"));

        for route in qotd_routes {
            let (status, qotd) =
                call(app.clone(), request("GET", route, Some(&learner), None)).await;
            assert_eq!(status, StatusCode::OK, "{state_name} at {route}: {qotd}");
            assert_eq!(qotd["available"], false, "{state_name} at {route}: {qotd}");
            assert_eq!(qotd["answered"], false, "{state_name} at {route}: {qotd}");
            for field in ["question_version_id", "vignette", "options"] {
                assert!(
                    qotd.get(field).is_none(),
                    "{state_name} leaked {field}: {qotd}"
                );
            }
        }
        for route in ["/v1/me/engagement", "/api/v1/me/engagement"] {
            let (status, engagement) =
                call(app.clone(), request("GET", route, Some(&learner), None)).await;
            assert_eq!(
                status,
                StatusCode::OK,
                "{state_name} at {route}: {engagement}"
            );
            let qotd = &engagement["qotd"];
            assert_eq!(qotd["available"], false, "{state_name} at {route}: {qotd}");
            for field in ["question_version_id", "vignette", "options"] {
                assert!(
                    qotd.get(field).is_none(),
                    "{state_name} leaked {field}: {qotd}"
                );
            }
        }

        let (status, stale_answer) = call(
            app.clone(),
            request(
                "POST",
                "/v1/me/qotd/answers",
                Some(&learner),
                Some(serde_json::json!({
                    "question_version_id": selected_id,
                    "chosen_index": 0
                })),
            ),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "{state_name}: {stale_answer}"
        );
        assert_eq!(
            stale_answer["error"]["code"], "qotd_unavailable",
            "{state_name}"
        );
        let answer_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM qotd_answers WHERE question_version_id = $1 AND day = CURRENT_DATE",
        )
        .bind(selected_id)
        .fetch_one(&state.pool)
        .await
        .expect("count answers after stale submission");
        assert_eq!(answer_count, 0, "{state_name} stale answer was persisted");

        sqlx::query(
            "UPDATE content_rights
             SET revoked_at = NULL, valid_from = DATE '2020-01-01', valid_to = NULL,
                 audiences = '[\"learners\"]'::jsonb, seat_limit = NULL, asset_refs = $2
             WHERE id = $1",
        )
        .bind(rights_id)
        .bind(&baseline_assets)
        .execute(&state.pool)
        .await
        .unwrap_or_else(|error| panic!("restore QOTD rights after {state_name}: {error}"));
        let (status, restored) = call(
            app.clone(),
            request("GET", "/v1/qotd", Some(&learner), None),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "restore after {state_name}: {restored}"
        );
        assert_eq!(
            restored["available"], true,
            "restore after {state_name}: {restored}"
        );
        assert_eq!(restored["question_version_id"], selected_id.to_string());
    }

    // With no persisted daily pick, an ineligible pool must not create one.
    sqlx::query("UPDATE content_rights SET asset_refs = '[]'::jsonb WHERE id = $1")
        .bind(rights_id)
        .execute(&state.pool)
        .await
        .expect("remove QOTD source scope");
    sqlx::query("DELETE FROM qotd_daily_questions WHERE exam_id = $1 AND day = CURRENT_DATE")
        .bind(ids.exam_id)
        .execute(&state.pool)
        .await
        .expect("clear daily QOTD pick");
    let (status, unavailable) = call(
        app.clone(),
        request("GET", "/v1/qotd", Some(&learner), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "ineligible pool: {unavailable}");
    assert_eq!(
        unavailable["available"], false,
        "ineligible pool: {unavailable}"
    );
    assert!(unavailable.get("question_version_id").is_none());
    let daily_pick_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM qotd_daily_questions WHERE exam_id = $1 AND day = CURRENT_DATE",
    )
    .bind(ids.exam_id)
    .fetch_one(&state.pool)
    .await
    .expect("daily pick count");
    assert_eq!(
        daily_pick_count, 0,
        "ineligible question was persisted as QOTD"
    );
}

#[tokio::test]
async fn qotd_revocation_preserves_answer_evidence_without_question_content() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let first_learner = register_and_login(app.clone()).await;
    let second_learner = register_and_login(app.clone()).await;

    for learner in [&first_learner, &second_learner] {
        let (status, settings) = call(
            app.clone(),
            request(
                "PUT",
                "/v1/me/engagement/settings",
                Some(learner),
                Some(serde_json::json!({"qotd_exam_id": ids.exam_id})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "QOTD settings: {settings}");
    }

    let (status, first_pick) = call(
        app.clone(),
        request("GET", "/v1/qotd", Some(&first_learner), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "first QOTD: {first_pick}");
    assert_eq!(first_pick["available"], true, "first QOTD: {first_pick}");
    let question_id = first_pick["question_version_id"].clone();
    let (status, second_pick) = call(
        app.clone(),
        request("GET", "/api/v1/me/qotd", Some(&second_learner), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "second QOTD: {second_pick}");
    assert_eq!(second_pick["question_version_id"], question_id);

    for (learner, chosen_index) in [(&first_learner, 0), (&second_learner, 1)] {
        let (status, answer) = call(
            app.clone(),
            request(
                "POST",
                "/v1/me/qotd/answers",
                Some(learner),
                Some(serde_json::json!({
                    "question_version_id": question_id,
                    "chosen_index": chosen_index
                })),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "QOTD answer: {answer}");
    }

    let rights_ref: String =
        sqlx::query_scalar("SELECT rights_ref FROM question_versions WHERE id = $1")
            .bind(question_id.as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&state.pool)
            .await
            .expect("selected question rights ref");
    sqlx::query("UPDATE content_rights SET revoked_at = now() WHERE ref_code = $1")
        .bind(rights_ref)
        .execute(&state.pool)
        .await
        .expect("revoke QOTD display grant");

    for (learner, route) in [
        (&first_learner, "/v1/me/qotd"),
        (&second_learner, "/api/v1/me/engagement"),
    ] {
        let (status, response) =
            call(app.clone(), request("GET", route, Some(learner), None)).await;
        assert_eq!(status, StatusCode::OK, "answered QOTD {route}: {response}");
        let qotd = if route.ends_with("engagement") {
            &response["qotd"]
        } else {
            &response
        };
        assert_eq!(qotd["answered"], true, "answered QOTD {route}: {qotd}");
        assert_eq!(qotd["available"], false, "answered QOTD {route}: {qotd}");
        assert_eq!(qotd["community_total"], 2, "answered QOTD {route}: {qotd}");
        assert_eq!(
            qotd["community_split"].as_array().unwrap().len(),
            2,
            "{qotd}"
        );
        for field in ["question_version_id", "vignette", "options"] {
            assert!(
                qotd.get(field).is_none(),
                "revoked answer leaked {field}: {qotd}"
            );
        }
    }

    let stored_answers: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM qotd_answers WHERE question_version_id = $1 AND day = CURRENT_DATE",
    )
    .bind(question_id.as_str().unwrap().parse::<Uuid>().unwrap())
    .fetch_one(&state.pool)
    .await
    .expect("stored QOTD answer evidence");
    assert_eq!(stored_answers, 2, "rights changes retain answer evidence");
}

#[tokio::test]
async fn learner_records_withhold_source_linked_content_when_question_rights_change() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let learner = register_and_login(app.clone()).await;
    let question_id = ids.question_versions[0];

    async fn get_json(app: &Router, token: &str, path: &str) -> Value {
        let (status, body) = call(app.clone(), request("GET", path, Some(token), None)).await;
        assert_eq!(status, StatusCode::OK, "GET {path}: {body}");
        body
    }

    let (rights_ref, baseline_assets): (String, Value) = sqlx::query_as(
        "SELECT qv.rights_ref, cr.asset_refs
         FROM question_versions qv
         JOIN content_rights cr ON cr.ref_code = qv.rights_ref
         WHERE qv.id = $1",
    )
    .bind(question_id)
    .fetch_one(&state.pool)
    .await
    .expect("seeded question rights");
    let rights_id: Uuid = sqlx::query_scalar("SELECT id FROM content_rights WHERE ref_code = $1")
        .bind(&rights_ref)
        .fetch_one(&state.pool)
        .await
        .expect("rights grant id");
    let eligible: bool = sqlx::query_scalar(
        "SELECT question_display_rights_active(
            (SELECT rights_ref FROM question_versions WHERE id = $1),
            (SELECT source_ref FROM question_versions WHERE id = $1),
            (SELECT source_refs FROM question_versions WHERE id = $1),
            (SELECT media_refs FROM question_versions WHERE id = $1)
        )",
    )
    .bind(question_id)
    .fetch_one(&state.pool)
    .await
    .expect("current question display rights");
    assert!(eligible, "seeded question must begin displayable");

    let (status, linked) = call(
        app.clone(),
        request(
            "POST",
            "/v1/notes",
            Some(&learner),
            Some(serde_json::json!({
                "title": "SOURCE-LINKED-PRIVATE-TITLE",
                "body": "SOURCE-LINKED-PRIVATE-BODY",
                "source_question_version_id": question_id
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "linked note: {linked}");
    let linked_note_id: Uuid = linked["note_id"].as_str().unwrap().parse().unwrap();

    let (status, unrelated) = call(
        app.clone(),
        request(
            "POST",
            "/v1/notes",
            Some(&learner),
            Some(serde_json::json!({
                "title": "UNLINKED-PRIVATE-TITLE",
                "body": "UNLINKED-PRIVATE-BODY"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "unlinked note: {unrelated}");
    let unrelated_note_id: Uuid = unrelated["note_id"].as_str().unwrap().parse().unwrap();
    let learner_id: Uuid = sqlx::query_scalar("SELECT user_id FROM notes WHERE id = $1")
        .bind(linked_note_id)
        .fetch_one(&state.pool)
        .await
        .expect("learner note owner");

    for note_id in [linked_note_id, unrelated_note_id] {
        let (status, tagged) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/notes/{note_id}/concepts"),
                Some(&learner),
                Some(serde_json::json!({"concept": "negative-feedback"})),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "tag note concept: {tagged}");
    }
    let (status, linked_backlink) = call(
        app.clone(),
        request(
            "POST",
            "/v1/notes/link",
            Some(&learner),
            Some(serde_json::json!({
                "from_note_id": unrelated_note_id,
                "to_note_id": linked_note_id
            })),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "link learner notes: {linked_backlink}"
    );

    let (status, mark) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/questions/{question_id}/mark"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "mark question: {mark}");

    sqlx::query(
        "INSERT INTO retest_cards (user_id, question_version_id, passes, due)
         VALUES ($1, $2, 1, now() - INTERVAL '1 minute')",
    )
    .bind(learner_id)
    .bind(question_id)
    .execute(&state.pool)
    .await
    .expect("saved due retest");

    let (status, deck) = call(
        app.clone(),
        request(
            "POST",
            "/v1/decks",
            Some(&learner),
            Some(serde_json::json!({"name": "Rights fixture deck"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create deck: {deck}");
    let deck_id: Uuid = deck["deck_id"].as_str().unwrap().parse().unwrap();

    let (status, linked_card) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/decks/{deck_id}/cards"),
            Some(&learner),
            Some(serde_json::json!({
                "front": "SOURCE-LINKED-CARD-FRONT",
                "back": "SOURCE-LINKED-CARD-BACK"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create linked card: {linked_card}");
    let linked_card_id: Uuid = linked_card["card_id"].as_str().unwrap().parse().unwrap();
    let (status, unrelated_card) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/decks/{deck_id}/cards"),
            Some(&learner),
            Some(serde_json::json!({
                "front": "UNLINKED-CARD-FRONT",
                "back": "UNLINKED-CARD-BACK"
            })),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create unrelated card: {unrelated_card}"
    );
    let unrelated_card_id: Uuid = unrelated_card["card_id"].as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE cards SET source_question_version_id = $2 WHERE id = $1")
        .bind(linked_card_id)
        .bind(question_id)
        .execute(&state.pool)
        .await
        .expect("record linked card provenance");

    let prefix_paths = |prefix: &str| {
        [
            format!("{prefix}/me/retests"),
            format!("{prefix}/me/marks"),
            format!("{prefix}/notes"),
            format!("{prefix}/concepts/negative-feedback/notes"),
            format!("{prefix}/notes/export"),
            format!("{prefix}/reviews/queue"),
            format!("{prefix}/me/decks/export"),
            format!("{prefix}/me/export"),
        ]
    };
    for prefix in ["/v1", "/api/v1"] {
        for path in prefix_paths(prefix) {
            let response = get_json(&app, &learner, &path).await;
            match path.as_str() {
                "/v1/me/retests" | "/api/v1/me/retests" => assert!(
                    response["retests"].as_array().unwrap().iter().any(|item| {
                        item["card_version_id"] == question_id.to_string()
                            && item["question_version_id"] == question_id.to_string()
                    }),
                    "active retest missing at {path}: {response}"
                ),
                "/v1/me/marks" | "/api/v1/me/marks" => assert!(
                    response["marks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|item| { item["question_version_id"] == question_id.to_string() }),
                    "active mark missing at {path}: {response}"
                ),
                "/v1/notes" | "/api/v1/notes" => assert!(
                    response["notes"].as_array().unwrap().iter().any(|item| {
                        item["note_id"] == linked_note_id.to_string()
                            && item["body"] == "SOURCE-LINKED-PRIVATE-BODY"
                    }),
                    "active linked note missing at {path}: {response}"
                ),
                "/v1/concepts/negative-feedback/notes"
                | "/api/v1/concepts/negative-feedback/notes" => assert!(
                    response["notes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|item| { item["note_id"] == linked_note_id.to_string() }),
                    "active concept note missing at {path}: {response}"
                ),
                "/v1/notes/export" | "/api/v1/notes/export" => assert!(
                    response["notes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|item| { item["body"] == "SOURCE-LINKED-PRIVATE-BODY" }),
                    "active linked note missing from export at {path}: {response}"
                ),
                "/v1/me/export" | "/api/v1/me/export" => assert!(
                    response["notes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|item| { item["body"] == "SOURCE-LINKED-PRIVATE-BODY" }),
                    "active linked note missing from account export at {path}: {response}"
                ),
                "/v1/reviews/queue" | "/api/v1/reviews/queue" => {
                    assert!(
                        response.to_string().contains("SOURCE-LINKED-CARD-FRONT"),
                        "active linked card missing at {path}: {response}"
                    );
                }
                "/v1/me/decks/export" | "/api/v1/me/decks/export" => {
                    assert!(
                        response.to_string().contains("SOURCE-LINKED-CARD-FRONT"),
                        "active linked card missing from export at {path}: {response}"
                    );
                }
                _ => unreachable!("covered learner route"),
            }
        }
    }

    for (state_name, mutation) in [
        (
            "revoked",
            "UPDATE content_rights SET revoked_at = now() WHERE id = $1",
        ),
        (
            "expired",
            "UPDATE content_rights SET valid_to = CURRENT_DATE - 1 WHERE id = $1",
        ),
        (
            "wrong audience",
            "UPDATE content_rights SET audiences = '[\"instructors\"]'::jsonb WHERE id = $1",
        ),
        (
            "unsupported seat limit",
            "UPDATE content_rights SET seat_limit = 25 WHERE id = $1",
        ),
        (
            "incomplete asset scope",
            "UPDATE content_rights SET asset_refs = '[]'::jsonb WHERE id = $1",
        ),
    ] {
        sqlx::query(mutation)
            .bind(rights_id)
            .execute(&state.pool)
            .await
            .unwrap_or_else(|error| panic!("set {state_name} rights: {error}"));

        for prefix in ["/v1", "/api/v1"] {
            let paths = prefix_paths(prefix);
            let retests = get_json(&app, &learner, &paths[0]).await;
            assert!(
                !retests["retests"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| { item["card_version_id"] == question_id.to_string() }),
                "{state_name} leaked retest at {}: {retests}",
                paths[0]
            );

            let marks = get_json(&app, &learner, &paths[1]).await;
            assert!(
                !marks["marks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| { item["question_version_id"] == question_id.to_string() }),
                "{state_name} leaked mark at {}: {marks}",
                paths[1]
            );

            let notes = get_json(&app, &learner, &paths[2]).await;
            let linked_listed = notes["notes"].as_array().unwrap().iter().any(|item| {
                item["note_id"] == linked_note_id.to_string()
                    || item["title"] == "SOURCE-LINKED-PRIVATE-TITLE"
                    || item["body"] == "SOURCE-LINKED-PRIVATE-BODY"
            });
            assert!(
                !linked_listed,
                "{state_name} leaked linked note at {}: {notes}",
                paths[2]
            );
            let unrelated = notes["notes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["note_id"] == unrelated_note_id.to_string())
                .expect("unlinked note stays visible");
            assert_eq!(unrelated["body"], "UNLINKED-PRIVATE-BODY");
            assert!(
                !unrelated["backlinks"]
                    .to_string()
                    .contains("SOURCE-LINKED-PRIVATE-TITLE"),
                "{state_name} leaked hidden backlink at {}: {notes}",
                paths[2]
            );

            let concept_notes = get_json(&app, &learner, &paths[3]).await;
            assert!(
                !concept_notes["notes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| { item["note_id"] == linked_note_id.to_string() }),
                "{state_name} leaked concept note at {}: {concept_notes}",
                paths[3]
            );
            assert!(
                concept_notes.to_string().contains("UNLINKED-PRIVATE-BODY"),
                "unlinked concept note missing at {}: {concept_notes}",
                paths[3]
            );

            let note_export = get_json(&app, &learner, &paths[4]).await;
            assert!(
                !note_export.to_string().contains("SOURCE-LINKED-PRIVATE"),
                "{state_name} leaked note export at {}: {note_export}",
                paths[4]
            );
            assert!(
                note_export.to_string().contains("UNLINKED-PRIVATE-BODY"),
                "unlinked note missing from export at {}: {note_export}",
                paths[4]
            );

            let account_export = get_json(&app, &learner, &paths[7]).await;
            assert!(
                !account_export.to_string().contains("SOURCE-LINKED-PRIVATE"),
                "{state_name} leaked account export at {}: {account_export}",
                paths[7]
            );
            assert!(
                account_export.to_string().contains("UNLINKED-PRIVATE-BODY"),
                "unlinked note missing from account export at {}: {account_export}",
                paths[7]
            );
            let queue = get_json(&app, &learner, &paths[5]).await;
            assert!(
                !queue.to_string().contains("SOURCE-LINKED-CARD"),
                "{state_name} leaked review card at {}: {queue}",
                paths[5]
            );
            assert!(
                queue.to_string().contains("UNLINKED-CARD-FRONT"),
                "unlinked card missing at {}: {queue}",
                paths[5]
            );

            let deck_export = get_json(&app, &learner, &paths[6]).await;
            assert!(
                !deck_export.to_string().contains("SOURCE-LINKED-CARD"),
                "{state_name} leaked exported card at {}: {deck_export}",
                paths[6]
            );
            assert!(
                deck_export.to_string().contains("UNLINKED-CARD-FRONT"),
                "unlinked card missing from export at {}: {deck_export}",
                paths[6]
            );

            let retained: (i64, i64, i64, i64, i64) = sqlx::query_as(
                "SELECT
                    (SELECT COUNT(*) FROM question_marks WHERE user_id = $1 AND question_version_id = $2),
                    (SELECT COUNT(*) FROM notes WHERE id = $3 AND user_id = $1),
                    (SELECT COUNT(*) FROM retest_cards WHERE user_id = $1 AND question_version_id = $2),
                    (SELECT COUNT(*) FROM cards WHERE id = $4 AND user_id = $1
                        AND source_question_version_id = $2),
                    (SELECT COUNT(*) FROM cards WHERE id = $5 AND user_id = $1)",
            )
            .bind(learner_id)
            .bind(question_id)
            .bind(linked_note_id)
            .bind(linked_card_id)
            .bind(unrelated_card_id)
            .fetch_one(&state.pool)
            .await
            .expect("learner records remain stored");
            assert_eq!(
                retained,
                (1, 1, 1, 1, 1),
                "{state_name} mutated stored learner records"
            );
        }

        sqlx::query(
            "UPDATE content_rights
             SET revoked_at = NULL, valid_from = DATE '2020-01-01', valid_to = NULL,
                 audiences = '[\"learners\"]'::jsonb, seat_limit = NULL, asset_refs = $2
             WHERE id = $1",
        )
        .bind(rights_id)
        .bind(&baseline_assets)
        .execute(&state.pool)
        .await
        .unwrap_or_else(|error| panic!("restore rights after {state_name}: {error}"));

        for prefix in ["/v1", "/api/v1"] {
            let paths = prefix_paths(prefix);
            let notes = get_json(&app, &learner, &paths[2]).await;
            assert!(
                notes["notes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| { item["note_id"] == linked_note_id.to_string() }),
                "restored linked note missing at {}: {notes}",
                paths[2]
            );
            let retests = get_json(&app, &learner, &paths[0]).await;
            assert!(
                retests["retests"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| { item["card_version_id"] == question_id.to_string() }),
                "restored retest missing at {}: {retests}",
                paths[0]
            );
            let marks = get_json(&app, &learner, &paths[1]).await;
            assert!(
                marks["marks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| { item["question_version_id"] == question_id.to_string() }),
                "restored mark missing at {}: {marks}",
                paths[1]
            );
            let account_export = get_json(&app, &learner, &paths[7]).await;
            assert!(
                account_export
                    .to_string()
                    .contains("SOURCE-LINKED-PRIVATE-BODY"),
                "restored linked note missing from account export at {}: {account_export}",
                paths[7]
            );
            let queue = get_json(&app, &learner, &paths[5]).await;
            assert!(
                queue.to_string().contains("SOURCE-LINKED-CARD-FRONT"),
                "restored linked card missing at {}: {queue}",
                paths[5]
            );
        }
    }

    let _ = unrelated_card_id;
}

#[tokio::test]
async fn pack_resource_download_rechecks_rights_after_a_tutoring_answer() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let learner = register_and_login(app.clone()).await;
    let chapter_id = create_rights_chapter(&state.pool, ids.exam_id).await;
    let rights_id = create_rights(
        &app,
        &author,
        "PACK-RIGHTS-LIVE",
        &["display", "derivatives", "offline"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    let question_id = create_question(&app, &author, chapter_id, Some("PACK-RIGHTS-LIVE")).await;
    approve_question(&app, &author, &reviewer, question_id).await;
    publish_result(&app, &reviewer, question_id).await;
    sqlx::query("UPDATE users SET tier = 'paid' WHERE tier = 'free'")
        .execute(&state.pool)
        .await
        .expect("enable paid pack fixture");

    let device_id = "rights-device";
    let (status, lease) = call(
        app.clone(),
        request(
            "POST",
            "/v1/packs/lease",
            Some(&learner),
            Some(serde_json::json!({
                "exam_id": ids.exam_id,
                "device_id": device_id,
                "chapters": [chapter_id]
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create pack lease: {lease}");

    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&learner),
            Some(serde_json::json!({
                "preset": "tutor",
                "chapter_id": chapter_id,
                "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "tutor session: {session}");
    let session_id = session["session_id"].as_str().unwrap();
    let (status, answer) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{session_id}/answers"),
            Some(&learner),
            Some(serde_json::json!({
                "item_index": 0,
                "chosen_index": 0,
                "idempotency_key": "rights-pack-tutor-answer"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "tutor answer: {answer}");
    assert_eq!(answer["tutoring_cards"].as_array().unwrap().len(), 5);

    let manifest_url = format!(
        "/v2/packs/{}/manifest?chapters={}&device_id={device_id}",
        ids.exam_id, chapter_id
    );
    let (status, manifest) = call(
        app.clone(),
        request("GET", &manifest_url, Some(&learner), None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "active pack manifest: {manifest}");
    assert!(manifest["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["question_version_id"] == question_id.to_string()));

    let resource_request = serde_json::json!({
        "device_id": device_id,
        "chapters": [chapter_id],
        "question_version_ids": [question_id]
    });
    let (status, resources) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v2/packs/{}/resources", ids.exam_id),
            Some(&learner),
            Some(resource_request.clone()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "active resources: {resources}");
    assert_eq!(
        resources["resources"][0]["tutoring_cards"]
            .as_array()
            .unwrap()
            .len(),
        5
    );

    let (status, revoked) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/content-rights/{rights_id}/revoke"),
            Some(&author),
            Some(serde_json::json!({"reason": "Synthetic pack fixture revocation"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "revoke pack grant: {revoked}");

    let (status, revoked_manifest) = call(
        app.clone(),
        request("GET", &manifest_url, Some(&learner), None),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "revoked pack manifest: {revoked_manifest}"
    );
    assert!(!revoked_manifest["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["question_version_id"] == question_id.to_string()));

    let (status, denied) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v2/packs/{}/resources", ids.exam_id),
            Some(&learner),
            Some(resource_request),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "revoked pack download: {denied}"
    );
    assert_eq!(denied["error"]["code"], "rights_unavailable");
    let (status, pregen) = call(
        app.clone(),
        admin_req(
            "POST",
            &format!("/v1/questions/versions/{question_id}/pregen-tutoring"),
            Some(&author),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "revoked tutoring generation: {pregen}"
    );
    assert_eq!(pregen["error"]["code"], "rights_unavailable");
    for content in [
        "Synthetic rights fixture vignette.",
        "Synthetic first rationale.",
        "Synthetic rights fixtures stay within scope.",
    ] {
        assert!(
            !denied.to_string().contains(content),
            "revoked question content leaked in pack response: {denied}"
        );
    }
}

#[tokio::test]
async fn question_linked_insights_keep_only_aggregate_evidence_after_rights_change() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let learner = register_and_login(app.clone()).await;
    let chapter_id = create_rights_chapter(&state.pool, ids.exam_id).await;
    let rights_id = create_rights(
        &app,
        &author,
        "INSIGHTS-RIGHTS-LIVE",
        &["display", "derivatives"],
        &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
        &["learners"],
        None,
        None,
    )
    .await;
    let question_id = create_question(&app, &author, chapter_id, Some("INSIGHTS-RIGHTS-LIVE")).await;
    approve_question(&app, &author, &reviewer, question_id).await;
    publish_result(&app, &reviewer, question_id).await;

    for attempt in 0..2 {
        let (status, session) = call(
            app.clone(),
            request(
                "POST",
                "/v1/practice/sessions",
                Some(&learner),
                Some(serde_json::json!({
                    "preset": "tutor",
                    "chapter_id": chapter_id,
                    "question_count": 1
                })),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "practice session: {session}");
        let session_id = session["session_id"].as_str().unwrap();
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
        assert_eq!(status, StatusCode::OK, "practice detail: {detail}");
        assert_eq!(
            detail["items"][0]["question_version_id"],
            question_id.to_string()
        );
        let (status, answer) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/practice/sessions/{session_id}/answers"),
                Some(&learner),
                Some(serde_json::json!({
                    "item_index": 0,
                    "chosen_index": 1,
                    "idempotency_key": format!("rights-insight-{attempt}")
                })),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "record wrong answer: {answer}");
        assert_eq!(answer["correct"], false);
    }

    let (status, revoked) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/content-rights/{rights_id}/revoke"),
            Some(&author),
            Some(serde_json::json!({"reason": "Synthetic insights fixture revocation"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "revoke insights grant: {revoked}");

    for prefix in ["/v1", "/api/v1"] {
        let paths = [
            format!("{prefix}/me/mistake-hypotheses"),
            format!("{prefix}/me/heatmap"),
            format!("{prefix}/me/trends?days=30&chapter_id={chapter_id}"),
        ];
        let mut responses = Vec::new();
        for path in paths {
            let (status, response) =
                call(app.clone(), request("GET", &path, Some(&learner), None)).await;
            assert_eq!(status, StatusCode::OK, "GET {path}: {response}");
            let serialized = response.to_string();
            assert!(
                !serialized.contains(&question_id.to_string()),
                "question identifier leaked at {path}: {response}"
            );
            for content in [
                "Synthetic rights fixture vignette.",
                "Which synthetic answer applies?",
                "Synthetic first rationale.",
                "Synthetic rights fixtures stay within scope.",
            ] {
                assert!(
                    !serialized.contains(content),
                    "question content leaked at {path}: {response}"
                );
            }
            responses.push(response);
        }

        let hypothesis = responses[0]["hypotheses"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["chapter_id"] == chapter_id.to_string())
            .expect("aggregate repeated-miss hypothesis remains visible");
        assert_eq!(hypothesis["misses"], 2);

        let chapter = responses[1]["systems"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|system| system["chapters"].as_array().unwrap())
            .find(|item| item["chapter_id"] == chapter_id.to_string())
            .expect("chapter mastery aggregate remains visible");
        assert_eq!(
            chapter["chapter_name"],
            format!("Rights delivery fixture {chapter_id}")
        );

        let trend = responses[2]["chapters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["chapter_id"] == chapter_id.to_string())
            .expect("chapter accuracy trend remains visible");
        let buckets = trend["buckets"].as_array().unwrap();
        assert_eq!(
            buckets
                .iter()
                .map(|bucket| bucket["answered"].as_i64().unwrap())
                .sum::<i64>(),
            2,
            "the revoked question's real attempts remain in aggregate evidence"
        );
        assert!(
            buckets.iter().all(|bucket| bucket["accuracy"] == 0),
            "both retained attempts were wrong: {trend}"
        );
    }
}

#[tokio::test]
async fn competition_attempts_recheck_rights_on_resume_answer_and_idempotent_replay() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let learner = register_and_login(app.clone()).await;
    let chapter_id = create_rights_chapter(&state.pool, ids.exam_id).await;
    let rights_ref = "COMPETITION-RIGHTS-LIVE";
    let rights_id = create_rights(
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
    let mut question_ids = Vec::new();
    for _ in 0..3 {
        let question_id = create_question(&app, &author, chapter_id, Some(rights_ref)).await;
        approve_question(&app, &author, &reviewer, question_id).await;
        assert_eq!(
            publish_result(&app, &reviewer, question_id).await["status"],
            "published"
        );
        question_ids.push(question_id);
    }

    let now = chrono::Utc::now();
    let starts_at = (now - chrono::Duration::minutes(1)).to_rfc3339();
    let ends_at = (now + chrono::Duration::hours(2)).to_rfc3339();
    let create_competition = |title: &str| {
        admin_req(
            "POST",
            "/v1/competitions",
            Some(&author),
            Some(serde_json::json!({
                "title": title,
                "exam_id": ids.exam_id,
                "question_ids": &question_ids,
                "starts_at": starts_at,
                "ends_at": ends_at
            })),
        )
    };
    for (state_name, mutation) in [
        (
            "revoked",
            "UPDATE content_rights SET revoked_at = clock_timestamp() WHERE id = $1",
        ),
        (
            "expired",
            "UPDATE content_rights SET valid_to = CURRENT_DATE - 1 WHERE id = $1",
        ),
        (
            "wrong audience",
            "UPDATE content_rights SET audiences = '[\"instructors\"]'::jsonb WHERE id = $1",
        ),
        (
            "seat limit",
            "UPDATE content_rights SET seat_limit = 25 WHERE id = $1",
        ),
        (
            "incomplete scope",
            "UPDATE content_rights SET asset_refs = '[]'::jsonb WHERE id = $1",
        ),
    ] {
        sqlx::query(mutation)
            .bind(rights_id)
            .execute(&state.pool)
            .await
            .unwrap_or_else(|error| panic!("set {state_name} competition grant: {error}"));
        let (status, denied) = call(
            app.clone(),
            create_competition(&format!("Ineligible {state_name}")),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{state_name} competition questions: {denied}"
        );
        assert_eq!(
            denied["error"]["code"], "invalid_competition_questions",
            "{state_name}: {denied}"
        );
        sqlx::query(
            "UPDATE content_rights SET revoked_at = NULL, valid_from = DATE '2020-01-01',
                 valid_to = NULL, audiences = '[\"learners\"]'::jsonb,
                 seat_limit = NULL, asset_refs = $2 WHERE id = $1",
        )
        .bind(rights_id)
        .bind(serde_json::json!([
            SOURCE_REF,
            SOURCE_REFS[1],
            MEDIA_REFS[0]
        ]))
        .execute(&state.pool)
        .await
        .unwrap_or_else(|error| panic!("restore competition rights after {state_name}: {error}"));
    }
    let (status, competition) = call(
        app.clone(),
        create_competition("Rights-aware competition fixture"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create competition: {competition}");
    let competition_id: Uuid = competition["competition_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let (status, profile) = call(
        app.clone(),
        request(
            "POST",
            "/v1/community/profile",
            Some(&learner),
            Some(serde_json::json!({ "handle": "rights-competition-learner" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "community profile: {profile}");

    sqlx::query("UPDATE content_rights SET revoked_at = clock_timestamp() WHERE id = $1")
        .bind(rights_id)
        .execute(&state.pool)
        .await
        .expect("revoke rights before the first attempt");
    let (status, denied_start) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/competitions/{competition_id}/entry"),
            Some(&learner),
            Some(serde_json::json!({ "handle": "rights-competition-learner" })),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "do not start from revoked competition questions: {denied_start}"
    );
    assert_eq!(denied_start["error"]["code"], "question_unavailable");
    let attempt_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM competition_attempts WHERE competition_id = $1")
            .bind(competition_id)
            .fetch_one(&state.pool)
            .await
            .expect("count attempts after denied start");
    assert_eq!(attempt_count, 0, "denied start persisted an attempt");
    sqlx::query(
        "UPDATE content_rights SET revoked_at = NULL, valid_from = DATE '2020-01-01',
             valid_to = NULL, audiences = '[\"learners\"]'::jsonb,
             seat_limit = NULL, asset_refs = $2 WHERE id = $1",
    )
    .bind(rights_id)
    .bind(serde_json::json!([
        SOURCE_REF,
        SOURCE_REFS[1],
        MEDIA_REFS[0]
    ]))
    .execute(&state.pool)
    .await
    .expect("restore rights after denied first attempt");

    let (status, first_step) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/competitions/{competition_id}/entry"),
            Some(&learner),
            Some(serde_json::json!({ "handle": "rights-competition-learner" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "start attempt: {first_step}");
    let first_question_id = first_step["question"]["question_version_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    let answer_body = serde_json::json!({
        "question_version_id": first_question_id,
        "chosen_index": 0,
        "idempotency_key": Uuid::new_v4()
    });
    let (status, next_step) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/competitions/{competition_id}/entry/answer"),
            Some(&learner),
            Some(answer_body.clone()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "answer first question: {next_step}");
    let cached_question = next_step["question"].clone();
    let next_question_id = cached_question["question_version_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();

    for (state_name, mutation) in [
        (
            "revoked",
            "UPDATE content_rights SET revoked_at = clock_timestamp() WHERE id = $1",
        ),
        (
            "expired",
            "UPDATE content_rights SET valid_to = CURRENT_DATE - 1 WHERE id = $1",
        ),
        (
            "wrong audience",
            "UPDATE content_rights SET audiences = '[\"instructors\"]'::jsonb WHERE id = $1",
        ),
        (
            "seat limit",
            "UPDATE content_rights SET seat_limit = 25 WHERE id = $1",
        ),
        (
            "incomplete scope",
            "UPDATE content_rights SET asset_refs = '[]'::jsonb WHERE id = $1",
        ),
    ] {
        sqlx::query(mutation)
            .bind(rights_id)
            .execute(&state.pool)
            .await
            .unwrap_or_else(|error| panic!("set {state_name} competition grant: {error}"));

        let (replay_status, replay) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/competitions/{competition_id}/entry/answer"),
                Some(&learner),
                Some(answer_body.clone()),
            ),
        )
        .await;
        assert_eq!(
            replay_status,
            StatusCode::CONFLICT,
            "{state_name} idempotent replay: {replay}"
        );
        assert_eq!(
            replay["error"]["code"], "question_unavailable",
            "{state_name}: {replay}"
        );
        assert!(
            replay.get("question").is_none()
                && !replay
                    .to_string()
                    .contains("Synthetic rights fixture vignette"),
            "{state_name} stale replay leaked question content: {replay}"
        );

        let (resume_status, resume) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/competitions/{competition_id}/entry"),
                Some(&learner),
                Some(serde_json::json!({ "handle": "rights-competition-learner" })),
            ),
        )
        .await;
        assert_eq!(
            resume_status,
            StatusCode::CONFLICT,
            "{state_name} stale resume: {resume}"
        );
        assert_eq!(
            resume["error"]["code"], "question_unavailable",
            "{state_name}: {resume}"
        );

        let (answer_status, stale_answer) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/competitions/{competition_id}/entry/answer"),
                Some(&learner),
                Some(serde_json::json!({
                    "question_version_id": next_question_id,
                    "chosen_index": 0,
                    "idempotency_key": Uuid::new_v4()
                })),
            ),
        )
        .await;
        assert_eq!(
            answer_status,
            StatusCode::CONFLICT,
            "{state_name} stale answer: {stale_answer}"
        );
        assert_eq!(
            stale_answer["error"]["code"], "question_unavailable",
            "{state_name}: {stale_answer}"
        );
        let saved_answers: Value = sqlx::query_scalar(
            "SELECT answers FROM competition_attempts WHERE competition_id = $1",
        )
        .bind(competition_id)
        .fetch_one(&state.pool)
        .await
        .expect("persisted competition answer");
        assert_eq!(
            saved_answers.as_array().unwrap().len(),
            1,
            "{state_name} stale answer mutated attempt"
        );

        sqlx::query(
            "UPDATE content_rights SET revoked_at = NULL, valid_from = DATE '2020-01-01',
                 valid_to = NULL, audiences = '[\"learners\"]'::jsonb,
                 seat_limit = NULL, asset_refs = $2 WHERE id = $1",
        )
        .bind(rights_id)
        .bind(serde_json::json!([
            SOURCE_REF,
            SOURCE_REFS[1],
            MEDIA_REFS[0]
        ]))
        .execute(&state.pool)
        .await
        .unwrap_or_else(|error| panic!("restore competition rights after {state_name}: {error}"));
        let (resume_status, resumed) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/competitions/{competition_id}/entry"),
                Some(&learner),
                Some(serde_json::json!({ "handle": "rights-competition-learner" })),
            ),
        )
        .await;
        assert_eq!(resume_status, StatusCode::OK, "restored resume: {resumed}");
        assert_eq!(
            resumed["question"], cached_question,
            "restored question changed after {state_name}"
        );
    }

    let (resume_status, mut step) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/competitions/{competition_id}/entry"),
            Some(&learner),
            Some(serde_json::json!({ "handle": "rights-competition-learner" })),
        ),
    )
    .await;
    assert_eq!(resume_status, StatusCode::OK, "restored resume: {step}");
    assert_eq!(
        step["question"], cached_question,
        "restored question changed"
    );
    for _ in 0..3 {
        if step["submitted"] == true {
            break;
        }
        let question_id = step["question"]["question_version_id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap();
        let (status, next) = call(
            app.clone(),
            request(
                "POST",
                &format!("/v1/competitions/{competition_id}/entry/answer"),
                Some(&learner),
                Some(serde_json::json!({
                    "question_version_id": question_id,
                    "chosen_index": 0,
                    "idempotency_key": Uuid::new_v4()
                })),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "finish attempt: {next}");
        step = next;
    }
    assert_eq!(step["submitted"], true, "attempt did not submit: {step}");

    sqlx::query("UPDATE content_rights SET revoked_at = clock_timestamp() WHERE id = $1")
        .bind(rights_id)
        .execute(&state.pool)
        .await
        .expect("revoke rights after submission");
    let (status, leaderboard) = call(
        app,
        request(
            "GET",
            &format!("/v1/competitions/{competition_id}/leaderboard"),
            Some(&learner),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "preserved leaderboard: {leaderboard}"
    );
    assert!(
        leaderboard["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["handle"] == "rights-competition-learner" && entry["score"].as_f64().is_some()
            }),
        "submitted score was lost: {leaderboard}"
    );
    for content_field in ["question_version_id", "vignette", "lead_in"] {
        assert!(
            leaderboard.get(content_field).is_none()
                && !leaderboard.to_string().contains(content_field),
            "leaderboard includes question content field {content_field}: {leaderboard}"
        );
    }
}

#[tokio::test]
async fn duel_acceptance_requires_a_full_current_pool_and_commits_once() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let (opponent_id, opponent) = register(app.clone(), "rights-duel-opponent".into()).await;
    let reviewer = register_and_login(app.clone()).await;
    let chapter_id = create_rights_chapter(&state.pool, ids.exam_id).await;

    let mut rights_ids = Vec::new();
    for index in 0..3 {
        let rights_ref = format!("COMP-DUEL-RIGHTS-{index}");
        rights_ids.push(
            create_rights(
                &app,
                &author,
                &rights_ref,
                &["display"],
                &[SOURCE_REF, SOURCE_REFS[1], MEDIA_REFS[0]],
                &["learners"],
                None,
                None,
            )
            .await,
        );
        let question_id =
            create_question(&app, &author, chapter_id, Some(rights_ref.as_str())).await;
        approve_question(&app, &author, &reviewer, question_id).await;
        assert_eq!(
            publish_result(&app, &reviewer, question_id).await["status"],
            "published"
        );
    }

    for (token, handle) in [
        (&author, "rights-duel-author"),
        (&opponent, "rights-duel-opponent"),
    ] {
        let (status, profile) = call(
            app.clone(),
            request(
                "POST",
                "/v1/community/profile",
                Some(token),
                Some(serde_json::json!({ "handle": handle })),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "community profile: {profile}");
    }
    let (status, duel) = call(
        app.clone(),
        request(
            "POST",
            "/v1/community/duels",
            Some(&author),
            Some(serde_json::json!({
                "opponent": opponent_id,
                "exam_id": ids.exam_id,
                "chapter_id": chapter_id,
                "question_count": 3
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create rights-aware duel: {duel}");
    let duel_id: Uuid = duel["duel_id"].as_str().unwrap().parse().unwrap();

    let (status, revoked) = call(
        app.clone(),
        admin_req(
            "PATCH",
            &format!("/v1/admin/content-rights/{}/revoke", rights_ids[0]),
            Some(&author),
            Some(serde_json::json!({ "reason": "Synthetic duel fixture grant ended" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "revoke duel grant: {revoked}");
    let (status, denied) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/community/duels/{duel_id}/accept"),
            Some(&opponent),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "partial rights-eligible duel must not start: {denied}"
    );
    assert_eq!(denied["error"]["code"], "empty_pool", "{denied}");
    let session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM practice_sessions WHERE preset = 'duel'")
            .fetch_one(&state.pool)
            .await
            .expect("count sessions after denied duel");
    assert_eq!(session_count, 0, "denied duel created partial sessions");
    let duel_status: String = sqlx::query_scalar("SELECT status FROM duels WHERE id = $1")
        .bind(duel_id)
        .fetch_one(&state.pool)
        .await
        .expect("pending duel state");
    assert_eq!(duel_status, "pending");

    sqlx::query("UPDATE content_rights SET revoked_at = NULL WHERE id = $1")
        .bind(rights_ids[0])
        .execute(&state.pool)
        .await
        .expect("restore duel fixture grant");

    let placeholder_session_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO practice_sessions (id, user_id, preset, chapter_id)
         VALUES ($1, $2, 'duel', $3)",
    )
    .bind(placeholder_session_id)
    .bind(opponent_id)
    .bind(chapter_id)
    .execute(&state.pool)
    .await
    .expect("create duplicate duel-session fixture");
    sqlx::query(
        "INSERT INTO duel_sessions (duel_id, user_id, session_id)
         VALUES ($1, $2, $3)",
    )
    .bind(duel_id)
    .bind(opponent_id)
    .bind(placeholder_session_id)
    .execute(&state.pool)
    .await
    .expect("reserve opponent duel slot");
    let (status, failed_accept) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/community/duels/{duel_id}/accept"),
            Some(&opponent),
            None,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "duplicate duel slot should reject the whole acceptance: {failed_accept}"
    );
    let persisted_sessions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM practice_sessions WHERE preset = 'duel'")
            .fetch_one(&state.pool)
            .await
            .expect("count sessions after failed duel write");
    let persisted_duel_sessions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM duel_sessions WHERE duel_id = $1")
            .bind(duel_id)
            .fetch_one(&state.pool)
            .await
            .expect("count duel session links after failed write");
    let persisted_items: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session_items item
         JOIN duel_sessions duel_session ON duel_session.session_id = item.session_id
         WHERE duel_session.duel_id = $1",
    )
    .bind(duel_id)
    .fetch_one(&state.pool)
    .await
    .expect("count duel items after failed write");
    assert_eq!(
        persisted_sessions, 1,
        "failed acceptance left partial sessions"
    );
    assert_eq!(
        persisted_duel_sessions, 1,
        "failed acceptance left a second link"
    );
    assert_eq!(
        persisted_items, 0,
        "failed acceptance left partial question items"
    );
    sqlx::query("DELETE FROM duel_sessions WHERE duel_id = $1 AND user_id = $2")
        .bind(duel_id)
        .bind(opponent_id)
        .execute(&state.pool)
        .await
        .expect("remove duplicate duel-session fixture");
    sqlx::query("DELETE FROM practice_sessions WHERE id = $1")
        .bind(placeholder_session_id)
        .execute(&state.pool)
        .await
        .expect("remove duplicate practice-session fixture");

    let mut hold_duel = state.pool.begin().await.expect("begin duel lock fixture");
    sqlx::query("SELECT id FROM duels WHERE id = $1 FOR UPDATE")
        .bind(duel_id)
        .fetch_one(&mut *hold_duel)
        .await
        .expect("lock pending duel");
    let first_app = app.clone();
    let first_token = opponent.clone();
    let first_path = format!("/v1/community/duels/{duel_id}/accept");
    let first = tokio::spawn(async move {
        call(
            first_app,
            request("POST", &first_path, Some(&first_token), None),
        )
        .await
    });
    let second_app = app.clone();
    let second_token = opponent.clone();
    let second_path = format!("/v1/community/duels/{duel_id}/accept");
    let second = tokio::spawn(async move {
        call(
            second_app,
            request("POST", &second_path, Some(&second_token), None),
        )
        .await
    });
    wait_for_blocked_statements(&state.pool, "FROM duels", 2).await;
    hold_duel.commit().await.expect("release duel lock fixture");

    let (first_status, first_body) = first.await.expect("first accept task");
    let (second_status, second_body) = second.await.expect("second accept task");
    assert!(
        (first_status == StatusCode::OK && second_status == StatusCode::CONFLICT)
            || (second_status == StatusCode::OK && first_status == StatusCode::CONFLICT),
        "exactly one concurrent acceptance must win: {first_status} {first_body}; {second_status} {second_body}"
    );
    let accepted = if first_status == StatusCode::OK {
        first_body
    } else {
        second_body
    };
    assert_eq!(accepted["question_count"], 3, "{accepted}");
    let session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM practice_sessions WHERE preset = 'duel'")
            .fetch_one(&state.pool)
            .await
            .expect("count accepted duel sessions");
    assert_eq!(
        session_count, 2,
        "acceptance must create exactly two sessions"
    );
    let item_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session_items item
         JOIN duel_sessions duel_session ON duel_session.session_id = item.session_id
         WHERE duel_session.duel_id = $1",
    )
    .bind(duel_id)
    .fetch_one(&state.pool)
    .await
    .expect("count accepted duel questions");
    assert_eq!(
        item_count, 6,
        "each duel session must contain all three questions"
    );
}

#[tokio::test]
async fn recurring_competitions_do_not_materialize_ineligible_question_pools() {
    let _guard = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let author = register_and_login(app.clone()).await;
    let reviewer = register_and_login(app.clone()).await;
    let learner = register_and_login(app.clone()).await;
    let chapter_id = create_rights_chapter(&state.pool, ids.exam_id).await;
    let rights_ref = "COMPETITION-RECURRING-RIGHTS";
    let rights_id = create_rights(
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
    let mut question_ids = Vec::new();
    for _ in 0..3 {
        let question_id = create_question(&app, &author, chapter_id, Some(rights_ref)).await;
        approve_question(&app, &author, &reviewer, question_id).await;
        assert_eq!(
            publish_result(&app, &reviewer, question_id).await["status"],
            "published"
        );
        question_ids.push(question_id);
    }

    let starts_at = chrono::Utc::now() - chrono::Duration::minutes(1);
    let ends_at = starts_at + chrono::Duration::hours(4);
    let (status, competition) = call(
        app.clone(),
        admin_req(
            "POST",
            "/v1/competitions",
            Some(&author),
            Some(serde_json::json!({
                "title": "Recurring rights fixture",
                "exam_id": ids.exam_id,
                "question_ids": question_ids,
                "starts_at": starts_at.to_rfc3339(),
                "ends_at": ends_at.to_rfc3339(),
                "cadence": "weekly",
                "question_count": 3
            })),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create recurring event: {competition}"
    );
    let series_id = competition["series_id"].as_str().unwrap().to_owned();

    sqlx::query("UPDATE content_rights SET revoked_at = clock_timestamp() WHERE id = $1")
        .bind(rights_id)
        .execute(&state.pool)
        .await
        .expect("revoke recurring pool rights");
    let (status, before_restore) = call(
        app.clone(),
        request("GET", "/v1/competitions", Some(&learner), None),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "list ineligible recurrence: {before_restore}"
    );
    let occurrence_count = before_restore["competitions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["series_id"] == series_id)
        .count();
    assert_eq!(
        occurrence_count, 1,
        "ineligible recurring questions produced events: {before_restore}"
    );

    sqlx::query(
        "UPDATE content_rights SET revoked_at = NULL, valid_from = DATE '2020-01-01',
             valid_to = NULL, audiences = '[\"learners\"]'::jsonb,
             seat_limit = NULL, asset_refs = $2 WHERE id = $1",
    )
    .bind(rights_id)
    .bind(serde_json::json!([
        SOURCE_REF,
        SOURCE_REFS[1],
        MEDIA_REFS[0]
    ]))
    .execute(&state.pool)
    .await
    .expect("restore recurring pool rights");
    sqlx::query("UPDATE competition_series SET next_start_at = now() WHERE id = $1")
        .bind(series_id.parse::<Uuid>().unwrap())
        .execute(&state.pool)
        .await
        .expect("make next occurrence due in fixture");
    let (status, after_restore) = call(
        app,
        request("GET", "/v1/competitions", Some(&learner), None),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "restored recurring pool: {after_restore}"
    );
    let restored_occurrence_count = after_restore["competitions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["series_id"] == series_id)
        .count();
    assert!(
        restored_occurrence_count > occurrence_count,
        "eligible recurring pool did not resume: {after_restore}"
    );
}
