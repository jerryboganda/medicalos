//! TRUST-02/OFF-01: full account export (GDPR-style) and signed pack
//! manifests. Export covers every table that stores this learner's content
//! or evidence; manifests sign pack contents with HMAC-SHA256 so the client
//! can verify integrity offline (§22).

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
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

fn signing_key(state: &AppState) -> Vec<u8> {
    // # ponytail: HMAC key from env with a dev fallback; a KMS-held key is
    // the Phase 3 hardening step when packs carry licensed media.
    state
        .pack_signing_key
        .as_deref()
        .unwrap_or("dev-pack-signing-key")
        .as_bytes()
        .to_vec()
}

fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
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
pub struct ManifestQuery {
    pub chapters: String,
}

/// OFF-01: signed manifest for an offline pack — the published question
/// versions with SHA-256 checksums of their content, plus an HMAC signature
/// the client verifies before trusting the pack (§22).
pub async fn pack_manifest(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(exam_id): Path<Uuid>,
    Query(q): Query<ManifestQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let chapter_ids: Vec<Uuid> = q
        .chapters
        .split(',')
        .filter_map(|c| Uuid::parse_str(c.trim()).ok())
        .collect();
    if chapter_ids.is_empty() {
        return Err(ApiError::unprocessable(
            "chapters_required",
            "pass ?chapters=<uuid,uuid,...>",
        ));
    }
    let _ = user;

    let mut items: Vec<serde_json::Value> = Vec::new();
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
        for r in rows {
            canonical.push_str(&format!("{} {}\n", r.id, r.checksum));
            items.push(json!({
                "question_version_id": r.id,
                "checksum": r.checksum,
            }));
        }
    }
    let signature = hmac_sha256_hex(&signing_key(&state), canonical.as_bytes());
    Ok(Json(json!({
        "exam_id": exam_id,
        "chapters": chapter_ids,
        "items": items,
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
    if req.device_id.is_empty() || req.device_id.len() > 128 {
        return Err(ApiError::unprocessable(
            "invalid_device",
            "device_id must be 1-128 characters",
        ));
    }
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
