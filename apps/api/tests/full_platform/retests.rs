use super::*;

pub(super) async fn submitted_practice(
    app: &Router,
    token: &str,
    chapter_id: Uuid,
    chosen_index: Option<i16>,
    confidence: &str,
    assisted: bool,
) -> (Uuid, Uuid) {
    let (status, session) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(token),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": chapter_id, "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let sid = session["session_id"].as_str().unwrap().parse().unwrap();
    let vid = session["items"][0]["question_version_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let (status, answer) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/answers"),
            Some(token),
            Some(serde_json::json!({
                "item_index": 0, "chosen_index": chosen_index,
                "confidence": confidence, "assisted": assisted,
                "idempotency_key": format!("retest-answer-{sid}")
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, submission) = call(
        app.clone(),
        request(
            "POST",
            &format!("/v1/practice/sessions/{sid}/submit"),
            Some(token),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{submission}");
    (sid, vid)
}

#[tokio::test]
async fn retest_refuses_correctness_claim_without_submitted_answer_evidence() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let learner = register_and_login(app.clone()).await;
    let (status, result) = call(
        app,
        request(
            "POST",
            "/v1/me/retests/result",
            Some(&learner),
            Some(serde_json::json!({
                "question_version_id": ids.question_versions[0],
                "correct": true,
                "idempotency_key": "forged-retest-pass"
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
}

fn result_request(token: &str, card: Uuid, sid: Uuid, key: &str) -> Request<Body> {
    request(
        "POST",
        "/v1/me/retests/result",
        Some(token),
        Some(serde_json::json!({
            "question_version_id": card, "session_id": sid, "item_index": 0,
            "correct": true, "idempotency_key": key
        })),
    )
}

#[tokio::test]
async fn retest_grades_stored_answers_and_replays_one_durable_receipt() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let learner = register_and_login(app.clone()).await;
    // The synthetic one-question chapter's key is A. A forged correct flag
    // alongside a submitted B must never manufacture a successful pass.
    let (wrong_sid, card) =
        submitted_practice(&app, &learner, ids.chapter3, Some(1), "sure", false).await;
    let (status, wrong) = call(
        app.clone(),
        result_request(&learner, card, wrong_sid, "wrong-evidence"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{wrong}");
    assert_eq!(wrong["correct"], false, "{wrong}");
    assert_eq!(wrong["rating"], "again", "{wrong}");
    assert_eq!(wrong["passes"], 0, "{wrong}");

    let (good_sid, _) =
        submitted_practice(&app, &learner, ids.chapter3, Some(0), "sure", false).await;
    let (status, good) = call(
        app.clone(),
        result_request(&learner, card, good_sid, "good-evidence"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{good}");
    assert_eq!(good["correct"], true, "{good}");
    assert_eq!(good["rating"], "good", "{good}");
    assert_eq!(good["passes"], 1, "{good}");
    let (status, replay) = call(
        app.clone(),
        result_request(&learner, card, good_sid, "good-evidence"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["already_recorded"], true);
    assert_eq!(replay["due"], good["due"]);
    assert_eq!(replay["passes"], good["passes"]);

    let (status, duplicate) = call(
        app.clone(),
        result_request(&learner, card, good_sid, "different-key"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{duplicate}");
    assert_eq!(duplicate["error"]["code"], "retest_attempt_used");
    let (fresh_sid, _) =
        submitted_practice(&app, &learner, ids.chapter3, Some(0), "sure", false).await;
    let (status, collision) = call(
        app,
        result_request(&learner, card, fresh_sid, "good-evidence"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{collision}");
    assert_eq!(collision["error"]["code"], "retest_key_conflict");
}

#[tokio::test]
async fn retest_uncertain_assisted_and_skipped_answers_do_not_advance_passes() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let learner = register_and_login(app.clone()).await;
    for (chosen, confidence, assisted, rating, correct) in [
        (Some(0), "unsure", false, "hard", true),
        (Some(0), "sure", true, "again", true),
        (None, "sure", false, "again", false),
    ] {
        let (sid, card) =
            submitted_practice(&app, &learner, ids.chapter3, chosen, confidence, assisted).await;
        let (status, result) = call(
            app.clone(),
            result_request(&learner, card, sid, &format!("case-{sid}")),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{result}");
        assert_eq!(result["correct"], correct, "{result}");
        assert_eq!(result["rating"], rating, "{result}");
        assert_eq!(result["passes"], 0, "{result}");
    }
}

#[tokio::test]
async fn retest_refuses_foreign_unrelated_unsubmitted_and_obsolete_evidence() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let learner = register_and_login(app.clone()).await;
    let outsider = register_and_login(app.clone()).await;
    let (old_sid, card) =
        submitted_practice(&app, &learner, ids.chapter3, Some(0), "sure", false).await;
    let (status, foreign) = call(
        app.clone(),
        result_request(&outsider, card, old_sid, "foreign"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{foreign}");
    let (status, unrelated) = call(
        app.clone(),
        result_request(&learner, ids.question_versions[0], old_sid, "unrelated"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{unrelated}");

    let (status, open) = call(
        app.clone(),
        request(
            "POST",
            "/v1/practice/sessions",
            Some(&learner),
            Some(serde_json::json!({
                "preset": "tutor", "chapter_id": ids.chapter3, "question_count": 1
            })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{open}");
    let open_sid = open["session_id"].as_str().unwrap().parse().unwrap();
    let (status, unfinished) = call(
        app.clone(),
        result_request(&learner, card, open_sid, "unfinished"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{unfinished}");
    assert_eq!(unfinished["error"]["code"], "retest_session_not_submitted");

    let _ = submitted_practice(&app, &learner, ids.chapter3, Some(1), "sure", false).await;
    let (status, obsolete) = call(
        app,
        result_request(&learner, card, old_sid, "old-good-answer"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{obsolete}");
    assert_eq!(obsolete["error"]["code"], "retest_evidence_obsolete");
}

#[tokio::test]
async fn concurrent_retest_keys_cannot_count_one_answer_twice() {
    let _g = LOCK.lock().await;
    let state = setup().await;
    let app = router(state.clone());
    let ids = seed::seed(&state.pool).await.expect("seed");
    let learner = register_and_login(app.clone()).await;
    let (sid, card) =
        submitted_practice(&app, &learner, ids.chapter3, Some(0), "sure", false).await;
    let (left, right) = tokio::join!(
        call(
            app.clone(),
            result_request(&learner, card, sid, "race-left")
        ),
        call(
            app.clone(),
            result_request(&learner, card, sid, "race-right")
        ),
    );
    let (winner, loser) = if left.0 == StatusCode::OK {
        (left, right)
    } else {
        (right, left)
    };
    assert_eq!(winner.0, StatusCode::OK, "{winner:?}");
    assert_eq!(winner.1["passes"], 1, "{winner:?}");
    assert_eq!(loser.0, StatusCode::CONFLICT, "{loser:?}");
    assert_eq!(loser.1["error"]["code"], "retest_attempt_used");
}
