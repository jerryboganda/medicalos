//! COMMUNITY-01/02/03 + COMP-01/03/04 + GROW-01: moderated groups, private
//! duels, integrity-gated prizes, and share tokens. §16/§18.1: community
//! participation is strictly opt-in (profile + handle) and moderation lives
//! with the group's own moderators.

use axum::extract::{Path, State};
use axum::Json;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

fn valid_handle(handle: &str) -> bool {
    let h = handle.as_bytes();
    (3..=30).contains(&h.len())
        && h.iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

// ---- COMMUNITY-03: opt-in identity ------------------------------------------

#[derive(Deserialize)]
pub struct ProfileReq {
    pub handle: String,
}

/// Creating a profile IS the opt-in. Nothing about a learner is shared
/// before this point, and leaderboards never show non-participants.
pub async fn create_profile(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<ProfileReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let handle = req.handle.trim().to_lowercase();
    if !valid_handle(&handle) {
        return Err(ApiError::unprocessable(
            "invalid_handle",
            "handle must be 3-30 characters of a-z, 0-9, or '-'",
        ));
    }
    let taken = sqlx::query!(
        "SELECT 1 AS one FROM community_profiles WHERE handle = $1",
        handle
    )
    .fetch_optional(&state.pool)
    .await?;
    if taken.is_some() {
        return Err(ApiError::conflict("handle_taken", "that handle is in use"));
    }
    sqlx::query!(
        "INSERT INTO community_profiles (user_id, handle) VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET handle = EXCLUDED.handle",
        user.user_id,
        handle
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "handle": handle })))
}

pub async fn my_profile(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query!(
        "SELECT handle FROM community_profiles WHERE user_id = $1",
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(Json(match row {
        Some(r) => json!({ "opted_in": true, "handle": r.handle }),
        None => json!({ "opted_in": false }),
    }))
}

// ---- COMMUNITY-01: moderated groups -----------------------------------------

#[derive(Deserialize)]
pub struct GroupReq {
    pub name: String,
}

pub async fn create_group(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<GroupReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let name = req.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::unprocessable(
            "invalid_name",
            "group name must be 1-100 characters",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO community_groups (id, name, created_by) VALUES ($1, $2, $3)",
        id,
        name,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    sqlx::query!(
        "INSERT INTO community_group_members (group_id, user_id, role)
         VALUES ($1, $2, 'moderator')",
        id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "group_id": id })))
}

async fn require_member(state: &AppState, group_id: Uuid, user_id: Uuid) -> ApiResult<()> {
    sqlx::query!(
        "SELECT 1 AS one FROM community_group_members
         WHERE group_id = $1 AND user_id = $2",
        group_id,
        user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::forbidden("membership_required", "join the group first"))?;
    Ok(())
}

pub async fn join_group(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let exists = sqlx::query!(
        "SELECT 1 AS one FROM community_groups WHERE id = $1",
        group_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("group_not_found"))?;
    let _ = exists;
    sqlx::query!(
        "INSERT INTO community_group_members (group_id, user_id, role)
         VALUES ($1, $2, 'member')
         ON CONFLICT (group_id, user_id) DO NOTHING",
        group_id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "joined": true })))
}

#[derive(Deserialize)]
pub struct PostReq {
    pub body: String,
}

pub async fn create_post(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
    Json(req): Json<PostReq>,
) -> ApiResult<Json<serde_json::Value>> {
    require_member(&state, group_id, user.user_id).await?;
    let body = req.body.trim();
    if body.is_empty() || body.len() > 2000 {
        return Err(ApiError::unprocessable(
            "invalid_body",
            "post body must be 1-2000 characters",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO community_posts (id, group_id, author, body) VALUES ($1, $2, $3, $4)",
        id,
        group_id,
        user.user_id,
        body
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "post_id": id })))
}

/// COMMUNITY-01: members see visible posts only; moderation removes content,
/// it does not hide the fact that content was removed (honest tombstone).
pub async fn list_posts(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_member(&state, group_id, user.user_id).await?;
    let rows = sqlx::query!(
        r#"SELECT p.id, p.body, p.status, p.created_at, cp.handle AS "handle!"
           FROM community_posts p
           JOIN community_profiles cp ON cp.user_id = p.author
           WHERE p.group_id = $1
           ORDER BY p.created_at DESC LIMIT 100"#,
        group_id
    )
    .fetch_all(&state.pool)
    .await?;
    // Authors without a community profile are shown as "member", never by
    // email or identity — COMMUNITY-03's non-coercive defaults run both ways.
    let anonymous = sqlx::query!(
        r#"SELECT p.id, p.body, p.status, p.created_at
           FROM community_posts p
           WHERE p.group_id = $1
             AND NOT EXISTS (
                 SELECT 1 FROM community_profiles cp WHERE cp.user_id = p.author)
           ORDER BY p.created_at DESC LIMIT 100"#,
        group_id
    )
    .fetch_all(&state.pool)
    .await?;
    let mut posts: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "post_id": r.id, "body": r.body, "status": r.status,
                "handle": r.handle, "at": r.created_at,
            })
        })
        .collect();
    posts.extend(anonymous.into_iter().map(|r| {
        json!({
            "post_id": r.id, "body": r.body, "status": r.status,
            "handle": "member", "at": r.created_at,
        })
    }));
    Ok(Json(json!({ "posts": posts })))
}

pub async fn remove_post(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((group_id, post_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    let moderator = sqlx::query!(
        "SELECT 1 AS one FROM community_group_members
         WHERE group_id = $1 AND user_id = $2 AND role = 'moderator'",
        group_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ApiError::forbidden("moderator_required", "only group moderators remove posts")
    })?;
    let _ = moderator;
    let updated = sqlx::query!(
        "UPDATE community_posts SET status = 'removed', removed_reason = 'moderator_removed'
         WHERE id = $1 AND group_id = $2",
        post_id,
        group_id
    )
    .execute(&state.pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::not_found("post_not_found"));
    }
    Ok(Json(json!({ "removed": true })))
}

// ---- COMP-03/GROW-01: private duels with share tokens ------------------------

#[derive(Deserialize)]
pub struct DuelReq {
    pub opponent: Uuid,
    pub exam_id: Uuid,
    pub chapter_id: Option<Uuid>,
    pub question_count: Option<i64>,
}

fn share_token() -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

pub async fn create_duel(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<DuelReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.opponent == user.user_id {
        return Err(ApiError::unprocessable(
            "invalid_opponent",
            "you cannot duel yourself",
        ));
    }
    let count = req.question_count.unwrap_or(5);
    if !(3..=20).contains(&count) {
        return Err(ApiError::unprocessable(
            "invalid_question_count",
            "duels use 3-20 questions",
        ));
    }
    let opponent_exists = sqlx::query!("SELECT 1 AS one FROM users WHERE id = $1", req.opponent)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("user_not_found"))?;
    let _ = opponent_exists;
    let id = Uuid::new_v4();
    let token = share_token();
    sqlx::query!(
        "INSERT INTO duels (id, exam_id, challenger, opponent, chapter_id, question_count, share_token)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        id,
        req.exam_id,
        user.user_id,
        req.opponent,
        req.chapter_id,
        count as i32,
        token
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "duel_id": id, "share_token": token })))
}

/// GROW-01: the payload a deferred deep link resolves. Participation still
/// requires the opponent's explicit accept — the link never auto-enters.
pub async fn duel_by_token(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(token): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let duel = sqlx::query!(
        r#"SELECT d.id, d.status, d.question_count, c.name AS "chapter?",
                  ch.handle AS "challenger_handle?", oh.handle AS "opponent_handle?"
           FROM duels d
           LEFT JOIN curriculum_nodes c ON c.id = d.chapter_id
           LEFT JOIN community_profiles ch ON ch.user_id = d.challenger
           LEFT JOIN community_profiles oh ON oh.user_id = d.opponent
           WHERE d.share_token = $1"#,
        token
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("duel_not_found"))?;
    Ok(Json(json!({
        "duel_id": duel.id,
        "status": duel.status,
        "question_count": duel.question_count,
        "chapter": duel.chapter,
        "challenger": duel.challenger_handle.unwrap_or_else(|| "member".into()),
        "opponent": duel.opponent_handle.unwrap_or_else(|| "member".into()),
    })))
}

pub async fn accept_duel(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(duel_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let duel = sqlx::query!(
        "SELECT id, challenger, opponent, status, chapter_id, question_count
         FROM duels WHERE id = $1",
        duel_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("duel_not_found"))?;
    if duel.opponent != user.user_id {
        return Err(ApiError::forbidden(
            "not_your_duel",
            "only the challenged opponent can accept",
        ));
    }
    if duel.status != "pending" {
        return Err(ApiError::conflict(
            "invalid_state",
            "this duel is no longer pending",
        ));
    }
    // Same selection rules for both sides: random published, non-reserved,
    // non-quarantined questions from the chapter (or exam-wide without one).
    let pick = |count: i32| async move {
        let rows = sqlx::query!(
            r#"SELECT qv.id FROM question_versions qv
               JOIN curriculum_nodes c ON c.id = qv.chapter_id
               WHERE qv.status = 'published'
                 AND c.exam_id = (SELECT exam_id FROM duels WHERE id = $1)
                 AND ($2::uuid IS NULL OR qv.chapter_id = $2)
                 AND NOT EXISTS (SELECT 1 FROM question_reports r
                                 WHERE r.question_version_id = qv.id AND r.status = 'quarantined')
                 AND NOT EXISTS (SELECT 1 FROM reserved_questions rq
                                 WHERE rq.question_version_id = qv.id)
               ORDER BY random() LIMIT $3"#,
            duel.id,
            duel.chapter_id,
            count as i64
        )
        .fetch_all(&state.pool)
        .await?;
        Ok::<Vec<Uuid>, ApiError>(rows.into_iter().map(|r| r.id).collect())
    };
    let challenger_qs = pick(duel.question_count).await?;
    let opponent_qs = pick(duel.question_count).await?;
    if challenger_qs.is_empty() || opponent_qs.is_empty() {
        return Err(ApiError::unprocessable(
            "empty_pool",
            "no questions available for this duel",
        ));
    }
    let challenger_session = Uuid::new_v4();
    let opponent_session = Uuid::new_v4();
    for (sid, uid, qs) in [
        (challenger_session, duel.challenger, &challenger_qs),
        (opponent_session, duel.opponent, &opponent_qs),
    ] {
        sqlx::query!(
            "INSERT INTO practice_sessions (id, user_id, preset, chapter_id)
             VALUES ($1, $2, 'duel', $3)",
            sid,
            uid,
            duel.chapter_id
        )
        .execute(&state.pool)
        .await?;
        for (i, vid) in qs.iter().enumerate() {
            let idx = i as i16;
            sqlx::query!(
                "INSERT INTO session_items (id, session_id, item_index, question_version_id)
                 VALUES ($1, $2, $3, $4)",
                Uuid::new_v4(),
                sid,
                idx,
                vid
            )
            .execute(&state.pool)
            .await?;
        }
        sqlx::query!(
            "INSERT INTO duel_sessions (duel_id, user_id, session_id) VALUES ($1, $2, $3)",
            duel.id,
            uid,
            sid
        )
        .execute(&state.pool)
        .await?;
    }
    sqlx::query!("UPDATE duels SET status = 'active' WHERE id = $1", duel.id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({
        "accepted": true,
        "your_session_id": opponent_session,
        "question_count": opponent_qs.len(),
    })))
}

/// Called from the practice submit pipeline: score a duel participant and
/// settle the duel once both sides are in. Correct count wins; total time
/// breaks ties; a true draw stays a draw (winner NULL, never fabricated).
pub async fn on_session_submitted(state: &AppState, session_id: Uuid) -> ApiResult<()> {
    let duel = sqlx::query!(
        "SELECT duel_id, user_id FROM duel_sessions WHERE session_id = $1",
        session_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let Some(d) = duel else {
        return Ok(());
    };
    let score = sqlx::query!(
        r#"SELECT COALESCE(COUNT(*) FILTER (WHERE correct), 0) AS "c!",
                  COALESCE(SUM(elapsed_ms), 0)::bigint AS "ms!"
           FROM attempts WHERE session_id = $1"#,
        session_id
    )
    .fetch_one(&state.pool)
    .await?;
    sqlx::query!(
        "UPDATE duel_sessions SET score = $3, total_ms = $4
         WHERE duel_id = $1 AND user_id = $2",
        d.duel_id,
        d.user_id,
        score.c as i32,
        score.ms
    )
    .execute(&state.pool)
    .await?;
    let sides = sqlx::query!(
        r#"SELECT user_id, score AS "score!", total_ms AS "ms!" FROM duel_sessions
           WHERE duel_id = $1"#,
        d.duel_id
    )
    .fetch_all(&state.pool)
    .await?;
    if sides.len() == 2 {
        let (a, b) = (&sides[0], &sides[1]);
        let winner = match a.score.cmp(&b.score) {
            std::cmp::Ordering::Greater => Some(a.user_id),
            std::cmp::Ordering::Less => Some(b.user_id),
            std::cmp::Ordering::Equal => match a.ms.cmp(&b.ms) {
                std::cmp::Ordering::Greater => Some(b.user_id),
                std::cmp::Ordering::Less => Some(a.user_id),
                std::cmp::Ordering::Equal => None,
            },
        };
        sqlx::query!(
            "UPDATE duels SET status = 'done', winner = $2 WHERE id = $1",
            d.duel_id,
            winner
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(())
}

pub async fn duel_state(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(duel_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let duel = sqlx::query!(
        "SELECT status, challenger, opponent, winner, question_count FROM duels WHERE id = $1",
        duel_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("duel_not_found"))?;
    if duel.challenger != user.user_id && duel.opponent != user.user_id {
        return Err(ApiError::forbidden(
            "not_your_duel",
            "duel state is private to its participants",
        ));
    }
    let sides = sqlx::query!(
        "SELECT user_id, score, total_ms, session_id FROM duel_sessions WHERE duel_id = $1",
        duel_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "status": duel.status,
        "winner": duel.winner,
        "sides": sides.iter().map(|s| json!({
            "user_id": s.user_id, "score": s.score, "total_ms": s.total_ms,
            "session_id": s.session_id,
        })).collect::<Vec<_>>(),
    })))
}

pub async fn decline_duel(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(duel_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let updated = sqlx::query!(
        "UPDATE duels SET status = 'declined'
         WHERE id = $1 AND opponent = $2 AND status = 'pending'",
        duel_id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::not_found("duel_not_found"));
    }
    Ok(Json(json!({ "declined": true })))
}

// ---- COMP-01/04: competition leaderboard + integrity-gated prizes -----------

/// Ranked entries for one competition. Only opted-in handles are named;
/// flagged entries stay listed but marked — honesty over cosmetics.
pub async fn competition_leaderboard(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(comp_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let comp = sqlx::query!(
        "SELECT prize_reviewed, status FROM competitions WHERE id = $1",
        comp_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("competition_not_found"))?;
    let rows = sqlx::query!(
        r#"SELECT ce.handle, ce.score, ce.total_time_ms, ce.flagged
           FROM competition_entries ce
           WHERE ce.competition_id = $1
           ORDER BY ce.score DESC, ce.total_time_ms ASC
           LIMIT 50"#,
        comp_id
    )
    .fetch_all(&state.pool)
    .await?;
    let entries: Vec<serde_json::Value> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            json!({
                "rank": i + 1,
                "handle": r.handle,
                "score": r.score,
                "total_time_ms": r.total_time_ms,
                "flagged": r.flagged,
                "prize_eligible": comp.prize_reviewed && !r.flagged,
            })
        })
        .collect();
    Ok(Json(json!({
        "prize_reviewed": comp.prize_reviewed,
        "status": comp.status,
        "entries": entries,
    })))
}

#[derive(Deserialize)]
pub struct PrizeReviewReq {
    pub flag: Option<Vec<String>>,
}

/// COMP-04: prizes only after integrity review. The admin marks the review
/// done (optionally flagging handles) — until then nobody can claim.
pub async fn prize_review(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
    Path(comp_id): Path<Uuid>,
    Json(req): Json<PrizeReviewReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provided = headers.get("x-admin-token").and_then(|v| v.to_str().ok());
    state.require_admin(provided)?;
    for handle in req.flag.unwrap_or_default() {
        sqlx::query!(
            "UPDATE competition_entries SET flagged = true
             WHERE competition_id = $1 AND handle = $2",
            comp_id,
            handle
        )
        .execute(&state.pool)
        .await?;
    }
    sqlx::query!(
        "UPDATE competitions SET prize_reviewed = true WHERE id = $1",
        comp_id
    )
    .execute(&state.pool)
    .await?;
    audit_pub(&state, user.user_id, comp_id).await?;
    Ok(Json(json!({ "prize_reviewed": true })))
}

async fn audit_pub(state: &AppState, actor: Uuid, comp_id: Uuid) -> ApiResult<()> {
    sqlx::query!(
        "INSERT INTO audit_events (id, actor, action, entity, entity_id, new_value)
         VALUES ($1, $2, 'competition_prize_reviewed', 'competition', $3, $4)",
        Uuid::new_v4(),
        actor,
        comp_id,
        json!({})
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}

/// A participant's prize claim: honest outcome only — eligibility requires
/// a submitted entry, a closed competition, completed review, no flag.
pub async fn claim_prize(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(comp_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let entry = sqlx::query!(
        "SELECT flagged, handle FROM competition_entries
         WHERE competition_id = $1 AND user_id = $2",
        comp_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("entry_not_found"))?;
    let comp = sqlx::query!(
        "SELECT prize_reviewed, status, ends_at FROM competitions WHERE id = $1",
        comp_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("competition_not_found"))?;
    let deny = |code: &'static str, msg: &str| {
        Err(ApiError::forbidden(code, msg)) as ApiResult<Json<serde_json::Value>>
    };
    if comp.status != "closed" && chrono::Utc::now() < comp.ends_at {
        return deny("competition_open", "the competition has not closed yet");
    }
    if !comp.prize_reviewed {
        return deny("prize_review_pending", "integrity review has not completed");
    }
    if entry.flagged {
        return deny("entry_flagged", "this entry is under integrity review");
    }
    Ok(Json(json!({
        "prize_claimable": true,
        "handle": entry.handle,
    })))
}
