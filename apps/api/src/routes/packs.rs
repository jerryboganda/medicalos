//! TRUST-02/OFF-01: full account export (GDPR-style) and signed pack
//! manifests. Export covers every table that stores this learner's content
//! or evidence; manifests sign pack contents with HMAC-SHA256 so the client
//! can verify integrity offline (§22).

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use sha2::{Digest, Sha256};

pub async fn export_account(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let profile = sqlx::query!(
        "SELECT email, created_at, tier FROM users WHERE id = $1",
        user.user_id
    )
    .fetch_one(&state.pool)
    .await?;
    let attempts = sqlx::query!(
        r#"SELECT question_version_id, chosen_index, correct, confidence, assisted, created_at
           FROM attempts WHERE user_id = $1 ORDER BY created_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let notes = sqlx::query!(
        "SELECT title, body, created_at FROM notes WHERE user_id = $1 ORDER BY created_at",
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let reviews = sqlx::query!(
        r#"SELECT card_id, rating, reviewed_at FROM review_events
           WHERE user_id = $1 ORDER BY reviewed_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let portfolio = sqlx::query!(
        r#"SELECT kind, title, detail, occurred_on FROM portfolio_entries
           WHERE user_id = $1 ORDER BY created_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "exported_at": chrono::Utc::now(),
        "account": {
            "email": profile.email,
            "tier": profile.tier,
            "created_at": profile.created_at,
        },
        "attempts": attempts.into_iter().map(|r| json!({
            "question_version_id": r.question_version_id,
            "chosen_index": r.chosen_index,
            "correct": r.correct,
            "confidence": r.confidence,
            "assisted": r.assisted,
            "created_at": r.created_at,
        })).collect::<Vec<_>>(),
        "notes": notes.into_iter().map(|r| json!({
            "title": r.title, "body": r.body, "created_at": r.created_at,
        })).collect::<Vec<_>>(),
        "card_reviews": reviews.into_iter().map(|r| json!({
            "card_id": r.card_id, "rating": r.rating, "reviewed_at": r.reviewed_at,
        })).collect::<Vec<_>>(),
        "portfolio": portfolio.into_iter().map(|r| json!({
            "kind": r.kind, "title": r.title, "detail": r.detail,
            "occurred_on": r.occurred_on,
        })).collect::<Vec<_>>(),
    })))
}

// ---- OFF-01: signed pack manifests -------------------------------------------

pub(crate) fn signing_key(state: &AppState) -> ApiResult<&[u8]> {
    configured_signing_key(state.pack_signing_key.as_deref())
}

fn configured_signing_key(key: Option<&str>) -> ApiResult<&[u8]> {
    key.filter(|value| value.trim().len() >= 32)
        .map(str::as_bytes)
        .ok_or_else(ApiError::internal)
}

pub(crate) fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    // RFC 2104 HMAC-SHA256 over sha2 — no extra dependency.
    const BLOCK: usize = 64;
    let mut k = key.to_vec();
    if k.len() > BLOCK {
        k = Sha256::digest(&k).to_vec();
    }
    k.resize(BLOCK, 0);
    let mut inner = Sha256::new();
    for b in k.iter() {
        inner.update([b ^ 0x36]);
    }
    inner.update(message);
    let inner_hash = inner.finalize();
    let mut outer = Sha256::new();
    for b in k.iter() {
        outer.update([b ^ 0x5c]);
    }
    outer.update(inner_hash);
    outer
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Deserialize)]
pub struct LegacyManifestQuery {
    pub chapters: String,
}

/// Preserve the v1 metadata-only contract for installed clients. It does not
/// return question content or tutoring cards; lease-scoped v3 manifests live
/// on the versioned v2 route.
pub async fn legacy_pack_manifest(
    State(state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(exam_id): Path<Uuid>,
    Query(q): Query<LegacyManifestQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let chapter_ids: Vec<Uuid> = q
        .chapters
        .split(',')
        .filter_map(|chapter| Uuid::parse_str(chapter.trim()).ok())
        .collect();
    if chapter_ids.is_empty() {
        return Err(ApiError::unprocessable(
            "chapters_required",
            "pass ?chapters=<uuid,uuid,...>",
        ));
    }

    let mut items = Vec::new();
    let mut canonical = String::new();
    for chapter_id in &chapter_ids {
        let rows = sqlx::query!(
            r#"SELECT id, encode(sha256((vignette || lead_in)::bytea), 'hex') AS "checksum!"
               FROM question_versions
               WHERE chapter_id = $1 AND status = 'published'
               ORDER BY id"#,
            chapter_id
        )
        .fetch_all(&state.pool)
        .await?;
        for row in rows {
            canonical.push_str(&format!("{} {}\n", row.id, row.checksum));
            items.push(json!({
                "question_version_id": row.id,
                "checksum": row.checksum,
            }));
        }
    }
    let signature = hmac_sha256_hex(signing_key(&state)?, canonical.as_bytes());
    Ok(Json(json!({
        "exam_id": exam_id,
        "chapters": chapter_ids,
        "items": items,
        "signature": signature,
        "algorithm": "hmac-sha256",
    })))
}

#[derive(Deserialize)]
pub struct ManifestQuery {
    pub chapters: String,
    pub device_id: String,
}

fn parse_chapter_ids(raw: &str) -> ApiResult<Vec<Uuid>> {
    let parts: Vec<_> = raw.split(',').map(str::trim).collect();
    if parts.is_empty() || parts.len() > 50 || parts.iter().any(|part| part.is_empty()) {
        return Err(ApiError::unprocessable(
            "invalid_chapters",
            "pass 1-50 unique chapter UUIDs as ?chapters=<uuid,uuid,...>",
        ));
    }
    let mut seen = std::collections::HashSet::with_capacity(parts.len());
    let mut chapters = Vec::with_capacity(parts.len());
    for part in parts {
        let chapter = Uuid::parse_str(part).map_err(|_| {
            ApiError::unprocessable("invalid_chapters", "chapter IDs must be valid UUIDs")
        })?;
        if !seen.insert(chapter) {
            return Err(ApiError::unprocessable(
                "invalid_chapters",
                "chapter IDs must be unique",
            ));
        }
        chapters.push(chapter);
    }
    Ok(chapters)
}

fn validate_device_id(device_id: &str) -> ApiResult<()> {
    if device_id.is_empty() || device_id.len() > 128 {
        return Err(ApiError::unprocessable(
            "invalid_device",
            "device_id must be 1-128 characters",
        ));
    }
    Ok(())
}

async fn validate_chapters(pool: &PgPool, exam_id: Uuid, chapters: &[Uuid]) -> ApiResult<()> {
    let valid_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM curriculum_nodes
         WHERE exam_id = $1 AND id = ANY($2) AND kind = 'chapter'",
    )
    .bind(exam_id)
    .bind(chapters.to_vec())
    .fetch_one(pool)
    .await?;
    if valid_count != chapters.len() as i64 {
        return Err(ApiError::unprocessable(
            "invalid_chapters",
            "every chapter must belong to the requested exam",
        ));
    }
    Ok(())
}

pub(crate) fn item_checksum(
    question_checksum: &str,
    tutoring_cards: &[crate::routes::program::TutoringCard],
) -> ApiResult<String> {
    let tutoring_bytes = serde_json::to_vec(tutoring_cards).map_err(|_| ApiError::internal())?;
    let mut content_hash = Sha256::new();
    content_hash.update(question_checksum.as_bytes());
    content_hash.update([0]);
    content_hash.update(tutoring_bytes);
    Ok(content_hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(crate) fn manifest_canonical(
    exam_id: Uuid,
    device_id: &str,
    chapters: &[Uuid],
    items: &[(Uuid, String)],
) -> ApiResult<String> {
    let device_id = serde_json::to_string(device_id).map_err(|_| ApiError::internal())?;
    let chapters = chapters
        .iter()
        .map(Uuid::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let mut canonical = format!(
        "medical-os-pack-manifest-v3\nexam {exam_id}\ndevice {device_id}\nchapters {chapters}\n"
    );
    for (id, checksum) in items {
        canonical.push_str(&format!("{id} {checksum}\n"));
    }
    Ok(canonical)
}

/// OFF-01: signed manifest v3 binds exam, device and chapter scope as well as
/// the question and tutoring-card contents (§22).
pub async fn pack_manifest(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(exam_id): Path<Uuid>,
    Query(q): Query<ManifestQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_device_id(&q.device_id)?;
    let chapter_ids = parse_chapter_ids(&q.chapters)?;
    validate_chapters(&state.pool, exam_id, &chapter_ids).await?;
    let chapter_json = serde_json::to_value(&chapter_ids).map_err(|_| ApiError::internal())?;
    let has_lease: bool = sqlx::query_scalar(
        "SELECT EXISTS(
             SELECT 1 FROM pack_leases
             WHERE user_id = $1 AND exam_id = $2 AND device_id = $3
               AND expires_at > now() AND chapters @> $4
         )",
    )
    .bind(user.user_id)
    .bind(exam_id)
    .bind(&q.device_id)
    .bind(chapter_json)
    .fetch_one(&state.pool)
    .await?;
    if !has_lease {
        return Err(ApiError::forbidden(
            "pack_lease_required",
            "an active device lease covering every requested chapter is required",
        ));
    }

    let mut items: Vec<serde_json::Value> = Vec::new();
    let mut canonical_items = Vec::new();
    for chapter_id in &chapter_ids {
        let rows = sqlx::query!(
            r#"SELECT id, encode(sha256((vignette || lead_in)::bytea), 'hex') AS "checksum!"
               FROM question_versions
               WHERE chapter_id = $1 AND status = 'published'
               ORDER BY id"#,
            chapter_id
        )
        .fetch_all(&state.pool)
        .await?;
        for r in rows {
            let answered_in_tutor: bool = sqlx::query_scalar(
                r#"SELECT EXISTS(
                       SELECT 1 FROM attempts a
                       JOIN practice_sessions s ON s.id = a.session_id
                       WHERE a.user_id = $1 AND a.question_version_id = $2
                         AND s.user_id = $1
                         AND s.preset = 'tutor'
                   )"#,
            )
            .bind(user.user_id)
            .bind(r.id)
            .fetch_one(&state.pool)
            .await?;
            let tutoring_cards = if answered_in_tutor {
                crate::routes::program::ensure_pregen(&state.pool, r.id).await?
            } else {
                Vec::new()
            };
            let checksum = item_checksum(&r.checksum, &tutoring_cards)?;
            canonical_items.push((r.id, checksum.clone()));
            items.push(json!({
                "question_version_id": r.id,
                "checksum": checksum,
                "tutoring_cards": tutoring_cards,
            }));
        }
    }
    let canonical = manifest_canonical(exam_id, &q.device_id, &chapter_ids, &canonical_items)?;
    let signature = hmac_sha256_hex(signing_key(&state)?, canonical.as_bytes());
    Ok(Json(json!({
        "exam_id": exam_id,
        "device_id": q.device_id,
        "chapters": chapter_ids,
        "items": items,
        "manifest_version": 3,
        "canonical_format": "medical-os-pack-manifest-v3",
        "item_checksum_algorithm": "sha256(question_checksum || 0x00 || serde_json(tutoring_cards))",
        "signature": signature,
        "algorithm": "hmac-sha256",
    })))
}

// ---- OFF-04 / PROT-02 / §22: pack leases ------------------------------------

#[derive(Deserialize)]
pub struct LeaseReq {
    pub exam_id: Uuid,
    /// Opaque per-device identifier (install-scoped, never a user id).
    pub device_id: String,
    pub chapters: Vec<Uuid>,
}

/// POST /v1/packs/lease — grant or renew a 14-day offline lease bound to one
/// device. Entitlement-gated (CORE-03): free-tier learners get an honest
/// refusal with the upgrade payload, never a degraded pack. The pack key is
/// disclosed only in this response — the client encrypts the local pack
/// with it, so another device holding copied files cannot open them (§22).
pub async fn create_lease(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<LeaseReq>,
) -> ApiResult<Json<serde_json::Value>> {
    validate_device_id(&req.device_id)?;
    let tier = sqlx::query!("SELECT tier FROM users WHERE id = $1", user.user_id)
        .fetch_one(&state.pool)
        .await?
        .tier;
    if tier == "free" {
        return Err(ApiError::forbidden_with_details(
            "offline_pack_entitlement",
            "Offline packs are part of the paid plan.",
            serde_json::json!({
                "entitlement": { "required_tier": "paid", "current_tier": tier }
            }),
        ));
    }
    if req.chapters.is_empty() || req.chapters.len() > 50 {
        return Err(ApiError::unprocessable(
            "invalid_chapters",
            "1-50 chapters per pack lease",
        ));
    }
    let mut seen = std::collections::HashSet::with_capacity(req.chapters.len());
    if req.chapters.iter().any(|chapter| !seen.insert(*chapter)) {
        return Err(ApiError::unprocessable(
            "invalid_chapters",
            "chapter IDs must be unique",
        ));
    }
    validate_chapters(&state.pool, req.exam_id, &req.chapters).await?;

    // Per-device pack key: random 32 bytes, rotated on renewal.
    use rand::RngCore;
    let mut key_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key_bytes);
    let pack_key: String = key_bytes.iter().map(|b| format!("{b:02x}")).collect();
    let expires = chrono::Utc::now() + chrono::Duration::days(14);

    let row = sqlx::query!(
        r#"INSERT INTO pack_leases (id, user_id, device_id, exam_id, chapters, pack_key, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT (user_id, device_id, exam_id) DO UPDATE SET
             chapters = $5, pack_key = $6, expires_at = $7, updated_at = now()
           RETURNING id, expires_at"#,
        Uuid::new_v4(),
        user.user_id,
        req.device_id,
        req.exam_id,
        serde_json::to_value(&req.chapters).map_err(|_| ApiError::internal())?,
        pack_key,
        expires
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({
        "lease_id": row.id,
        "expires_at": row.expires_at,
        "pack_key": pack_key,
        "algorithm": "xchacha20-poly1305 (client-side; key shown once per renewal)",
    })))
}

/// GET /v1/me/packs — active leases with freshness disclosure (OFF-04):
/// content_as_of is when the server issued/refreshed this lease. Question
/// versions do not carry publication timestamps, so claiming a newer content
/// timestamp would fabricate precision.
pub async fn list_leases(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT pl.id, pl.exam_id, pl.device_id, pl.chapters, pl.expires_at,
                  pl.created_at AS "content_as_of!"
           FROM pack_leases pl
           WHERE pl.user_id = $1 AND pl.expires_at > now()
           ORDER BY pl.expires_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let leases: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "lease_id": r.id,
                "exam_id": r.exam_id,
                "device_id": r.device_id,
                "chapters": r.chapters,
                "expires_at": r.expires_at,
                "content_as_of": r.content_as_of,
            })
        })
        .collect();
    Ok(Json(json!({ "leases": leases })))
}

/// DELETE /v1/packs/lease/{lease_id} — revoke. The stored key becomes
/// useless at the next manifest verification (PROT-02).
pub async fn revoke_lease(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(lease_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let deleted = sqlx::query!(
        "DELETE FROM pack_leases WHERE id = $1 AND user_id = $2",
        lease_id,
        user.user_id
    )
    .execute(&state.pool)
    .await?;
    if deleted.rows_affected() == 0 {
        return Err(ApiError::not_found("lease_not_found"));
    }
    Ok(Json(json!({ "revoked": true })))
}

#[cfg(test)]
mod signing_key_tests {
    use super::configured_signing_key;

    #[test]
    fn signing_key_fails_closed_when_missing_or_weak() {
        assert!(configured_signing_key(None).is_err());
        assert!(configured_signing_key(Some("")).is_err());
        assert!(configured_signing_key(Some("short")).is_err());
        assert!(configured_signing_key(Some("                                ")).is_err());
    }

    #[test]
    fn signing_key_accepts_at_least_32_non_whitespace_bytes() {
        let key = "0123456789abcdef0123456789abcdef";
        assert_eq!(configured_signing_key(Some(key)).unwrap(), key.as_bytes());
    }
}
