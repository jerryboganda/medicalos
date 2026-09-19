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
        "attempts": attempts,
        "notes": notes,
        "card_reviews": reviews,
        "portfolio": portfolio,
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
            r#"SELECT id, encode(sha256((vignette || lead_in)::bytea), 'hex') AS checksum
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
    let signature = hmac_sha256_hex(signing_key(&state), canonical.as_bytes());
    Ok(Json(json!({
        "exam_id": exam_id,
        "chapters": chapter_ids,
        "items": items,
        "signature": signature,
        "algorithm": "hmac-sha256",
    })))
}
