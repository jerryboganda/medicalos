//! TRUST-02/OFF-01: full account export (GDPR-style) and signed pack
//! manifests. Export covers every table that stores this learner's content
//! or evidence. Legacy manifests use HMAC; lease-bound browser packs use
//! publicly verifiable Ed25519 signatures (§22).

use axum::extract::{Path, Query, State};
use axum::Json;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use sha2::{Digest, Sha256};

const MAX_BROWSER_PACK_QUESTIONS: i64 = 500;

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackManifestItem.ts",
        rename = "PackManifestItem"
    )
)]
pub struct PackManifestItem {
    pub question_version_id: Uuid,
    pub checksum: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "packs/PackManifest.ts", rename = "PackManifest")
)]
pub struct PackManifest {
    pub exam_id: Uuid,
    pub device_id: String,
    pub chapters: Vec<Uuid>,
    pub items: Vec<PackManifestItem>,
    pub manifest_version: u8,
    pub canonical_format: String,
    pub item_checksum_algorithm: String,
    pub verification_key: String,
    pub key_id: String,
    pub signature: String,
    pub algorithm: String,
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackQuestionResource.ts",
        rename = "PackQuestionResource"
    )
)]
pub struct PackQuestionResource {
    pub question_version_id: Uuid,
    pub vignette: String,
    pub lead_in: String,
    pub difficulty: String,
    pub options: Vec<crate::seed::QuestionOption>,
    pub correct_index: i16,
    pub key_learning_point: String,
    pub exam_tip: Option<String>,
    pub source_ref: String,
    pub tutoring_cards: Vec<crate::routes::program::TutoringCard>,
    pub checksum: String,
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackResourcesRequest.ts",
        rename = "PackResourcesRequest"
    )
)]
#[serde(deny_unknown_fields)]
pub struct PackResourcesReq {
    pub device_id: String,
    pub chapters: Vec<Uuid>,
    pub question_version_ids: Vec<Uuid>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackResourcesResponse.ts",
        rename = "PackResourcesResponse"
    )
)]
pub struct PackResourcesResponse {
    pub resources: Vec<PackQuestionResource>,
    pub receipt: PackDownloadReceipt,
}

/// OFF-01: a signed, server-recorded attestation that this device received
/// rights-verified content for exactly these checksums under an active lease.
#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackDownloadReceipt.ts",
        rename = "PackDownloadReceipt"
    )
)]
pub struct PackDownloadReceipt {
    pub device_id: String,
    pub exam_id: Uuid,
    /// RFC3339 — this exact string is part of the signed payload.
    pub issued_at: String,
    pub checksums: Vec<String>,
    pub signature: String,
}

/// The exact bytes an auditor or the browser re-covers from the receipt fields
/// before checking the Ed25519 signature. Shared by the handler and tests so
/// the canonical form cannot drift between signer and verifier.
pub fn pack_download_receipt_message(
    device_id: &str,
    exam_id: Uuid,
    issued_at: &str,
    checksums: &[String],
) -> String {
    let payload = json!({
        "checksums": checksums,
        "device_id": device_id,
        "exam_id": exam_id,
        "issued_at": issued_at,
    });
    let mut canonical = String::new();
    canonical_value(&payload, &mut canonical);
    canonical
}

#[derive(Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackLeaseRequest.ts",
        rename = "PackLeaseRequest"
    )
)]
#[serde(deny_unknown_fields)]
pub struct LeaseReq {
    pub exam_id: Uuid,
    /// Opaque per-device identifier (install-scoped, never a user id).
    pub device_id: String,
    pub chapters: Vec<Uuid>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackLeaseResponse.ts",
        rename = "PackLeaseResponse"
    )
)]
pub struct PackLeaseResponse {
    pub lease_id: Uuid,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub expires_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub content_as_of: chrono::DateTime<chrono::Utc>,
    pub pack_key: String,
    pub algorithm: String,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackLeaseSummary.ts",
        rename = "PackLeaseSummary"
    )
)]
pub struct PackLeaseSummary {
    pub lease_id: Uuid,
    pub exam_id: Uuid,
    pub device_id: String,
    pub chapters: Vec<Uuid>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub expires_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    pub content_as_of: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackLeaseListResponse.ts",
        rename = "PackLeaseListResponse"
    )
)]
pub struct PackLeaseListResponse {
    pub leases: Vec<PackLeaseSummary>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/PackLeaseRevokedResponse.ts",
        rename = "PackLeaseRevokedResponse"
    )
)]
pub struct PackLeaseRevokedResponse {
    pub revoked: bool,
}

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

pub(crate) fn ed25519_signing_key(state: &AppState) -> ApiResult<SigningKey> {
    let secret = signing_key(state)?;
    let mut derivation = Sha256::new();
    derivation.update(b"medical-os-offline-pack-ed25519-seed-v1\0");
    derivation.update(secret);
    let seed: [u8; 32] = derivation.finalize().into();
    Ok(SigningKey::from_bytes(&seed))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A small cross-language canonical form: object keys sort by UTF-8 bytes,
/// strings encode as a byte length plus hex, and values carry explicit tags.
/// This avoids JSON serializer escaping and property-order differences.
fn canonical_value(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push_str("z;"),
        Value::Bool(false) => output.push_str("f;"),
        Value::Bool(true) => output.push_str("t;"),
        Value::Number(number) => {
            output.push('n');
            output.push_str(&number.to_string());
            output.push(';');
        }
        Value::String(string) => {
            output.push('s');
            output.push_str(&string.len().to_string());
            output.push(':');
            output.push_str(&hex(string.as_bytes()));
            output.push(';');
        }
        Value::Array(values) => {
            output.push('a');
            output.push_str(&values.len().to_string());
            output.push(':');
            for value in values {
                canonical_value(value, output);
            }
        }
        Value::Object(values) => {
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_by(|(left, _), (right, _)| left.as_bytes().cmp(right.as_bytes()));
            output.push('o');
            output.push_str(&entries.len().to_string());
            output.push(':');
            for (key, value) in entries {
                canonical_value(&Value::String(key.clone()), output);
                canonical_value(value, output);
            }
        }
    }
}

fn resource_checksum(content: &Value) -> String {
    let mut canonical = String::new();
    canonical_value(content, &mut canonical);
    hex(&Sha256::digest(canonical.as_bytes()))
}

#[derive(Deserialize)]
pub struct LegacyManifestQuery {
    pub chapters: String,
}

/// Preserve the v1 metadata-only contract for installed clients. It does not
/// return question content or tutoring cards; lease-scoped v4 manifests live
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
        "medical-os-pack-manifest-v4\nexam {exam_id}\ndevice {device_id}\nchapters {chapters}\n"
    );
    for (id, checksum) in items {
        canonical.push_str(&format!("{id} {checksum}\n"));
    }
    Ok(canonical)
}

pub(crate) async fn answered_tutor_question_ids(
    pool: &PgPool,
    user_id: Uuid,
    question_version_ids: &[Uuid],
) -> ApiResult<std::collections::HashSet<Uuid>> {
    if question_version_ids.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let rows = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT DISTINCT a.question_version_id
           FROM attempts a
           JOIN practice_sessions s ON s.id = a.session_id
           WHERE a.user_id = $1 AND s.user_id = $1 AND s.preset = 'tutor'
             AND a.question_version_id = ANY($2)"#,
    )
    .bind(user_id)
    .bind(question_version_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

pub(crate) async fn tutoring_cards_for_questions(
    pool: &PgPool,
    question_version_ids: &[Uuid],
) -> ApiResult<std::collections::HashMap<Uuid, Vec<crate::routes::program::TutoringCard>>> {
    if question_version_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let rows = sqlx::query!(
        r#"SELECT p.question_version_id, p.prompt_type, p.content, q.source_ref
           FROM pregen_tutoring p
           JOIN question_versions q ON q.id = p.question_version_id
           WHERE p.question_version_id = ANY($1) AND q.status = 'published'
             AND NOT EXISTS (
                 SELECT 1 FROM reserved_questions rq
                 JOIN assessment_forms f ON f.id = rq.form_id
                 WHERE rq.question_version_id = q.id AND f.ai_allowed = FALSE
             )
           ORDER BY p.question_version_id, p.prompt_type"#,
        question_version_ids
    )
    .fetch_all(pool)
    .await?;

    let mut cards = std::collections::HashMap::new();
    for row in rows {
        cards
            .entry(row.question_version_id)
            .or_insert_with(Vec::new)
            .push(crate::routes::program::TutoringCard {
                prompt_type: row.prompt_type,
                content: row.content,
                source_ref: row.source_ref,
            });
    }
    for question_version_id in question_version_ids {
        if cards.get(question_version_id).map(Vec::len) != Some(5) {
            cards.insert(
                *question_version_id,
                crate::routes::program::ensure_pregen(pool, *question_version_id).await?,
            );
        }
    }
    Ok(cards)
}

pub(crate) struct PackQuestionData {
    pub(crate) question_version_id: Uuid,
    pub(crate) vignette: String,
    pub(crate) lead_in: String,
    pub(crate) difficulty: String,
    pub(crate) options: Value,
    pub(crate) correct_index: i16,
    pub(crate) key_learning_point: String,
    pub(crate) exam_tip: Option<String>,
    pub(crate) source_ref: String,
    pub(crate) tutoring_cards: Vec<crate::routes::program::TutoringCard>,
}

pub(crate) fn build_pack_resource(question: PackQuestionData) -> ApiResult<PackQuestionResource> {
    let PackQuestionData {
        question_version_id,
        vignette,
        lead_in,
        difficulty,
        options,
        correct_index,
        key_learning_point,
        exam_tip,
        source_ref,
        tutoring_cards,
    } = question;
    let options: Vec<crate::seed::QuestionOption> =
        serde_json::from_value(options).map_err(|_| ApiError::internal())?;
    let mut content = json!({
        "question_version_id": question_version_id,
        "vignette": vignette,
        "lead_in": lead_in,
        "difficulty": difficulty,
        "options": options,
        "correct_index": correct_index,
        "key_learning_point": key_learning_point,
        "exam_tip": exam_tip,
        "source_ref": source_ref,
        "tutoring_cards": tutoring_cards,
    });
    let checksum = resource_checksum(&content);
    content
        .as_object_mut()
        .ok_or_else(ApiError::internal)?
        .insert("checksum".into(), Value::String(checksum));
    serde_json::from_value(content).map_err(|_| ApiError::internal())
}

/// OFF-01: signs every byte-relevant field in each prepared practice resource.
pub async fn pack_manifest(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(exam_id): Path<Uuid>,
    Query(q): Query<ManifestQuery>,
) -> ApiResult<Json<PackManifest>> {
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

    let published_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM question_versions
         WHERE chapter_id = ANY($1) AND status = 'published'",
    )
    .bind(&chapter_ids)
    .fetch_one(&state.pool)
    .await?;
    if published_count > MAX_BROWSER_PACK_QUESTIONS {
        return Err(ApiError::unprocessable(
            "pack_too_large",
            "browser packs are limited to 500 published questions; choose fewer chapters",
        ));
    }

    let mut items: Vec<PackManifestItem> = Vec::new();
    let mut canonical_items = Vec::new();
    let rows = sqlx::query!(
        r#"SELECT id, vignette, lead_in, difficulty, options, correct_index,
                  key_learning_point, exam_tip, source_ref
           FROM question_versions
           WHERE chapter_id = ANY($1) AND status = 'published'
           ORDER BY chapter_id, id"#,
        &chapter_ids
    )
    .fetch_all(&state.pool)
    .await?;
    let question_ids: Vec<_> = rows.iter().map(|row| row.id).collect();
    let answered = answered_tutor_question_ids(&state.pool, user.user_id, &question_ids).await?;
    let answered_ids: Vec<_> = answered.iter().copied().collect();
    let mut tutoring_cards = tutoring_cards_for_questions(&state.pool, &answered_ids).await?;
    for r in rows {
        let resource = build_pack_resource(PackQuestionData {
            question_version_id: r.id,
            vignette: r.vignette,
            lead_in: r.lead_in,
            difficulty: r.difficulty,
            options: r.options,
            correct_index: r.correct_index,
            key_learning_point: r.key_learning_point,
            exam_tip: r.exam_tip,
            source_ref: r.source_ref,
            tutoring_cards: tutoring_cards.remove(&r.id).unwrap_or_default(),
        })?;
        let checksum = resource.checksum.clone();
        canonical_items.push((r.id, checksum.clone()));
        items.push(PackManifestItem {
            question_version_id: r.id,
            checksum,
        });
    }
    let canonical = manifest_canonical(exam_id, &q.device_id, &chapter_ids, &canonical_items)?;
    let signer = ed25519_signing_key(&state)?;
    let public_key = signer.verifying_key().to_bytes();
    let key_id = hex(&Sha256::digest(public_key)[..8]);
    let signature = signer.sign(canonical.as_bytes()).to_bytes();
    Ok(Json(PackManifest {
        exam_id,
        device_id: q.device_id,
        chapters: chapter_ids,
        items,
        manifest_version: 4,
        canonical_format: "medical-os-pack-manifest-v4".into(),
        item_checksum_algorithm: "sha256(medical-os-resource-canonical-v1)".into(),
        verification_key: hex(&public_key),
        key_id,
        signature: hex(&signature),
        algorithm: "ed25519".into(),
    }))
}

/// Return one bounded resource batch covered by an active device lease.
/// The client compares every checksum with the previously verified manifest.
pub async fn pack_resources(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(exam_id): Path<Uuid>,
    Json(req): Json<PackResourcesReq>,
) -> ApiResult<Json<PackResourcesResponse>> {
    validate_device_id(&req.device_id)?;
    if req.chapters.is_empty() || req.chapters.len() > 50 {
        return Err(ApiError::unprocessable(
            "invalid_chapters",
            "pass 1-50 chapter IDs",
        ));
    }
    let mut seen_chapters = std::collections::HashSet::with_capacity(req.chapters.len());
    if req
        .chapters
        .iter()
        .any(|chapter| !seen_chapters.insert(*chapter))
    {
        return Err(ApiError::unprocessable(
            "invalid_chapters",
            "chapter IDs must be unique",
        ));
    }
    validate_chapters(&state.pool, exam_id, &req.chapters).await?;
    if req.question_version_ids.is_empty() || req.question_version_ids.len() > 50 {
        return Err(ApiError::unprocessable(
            "invalid_pack_resources",
            "pass 1-50 unique question version IDs per batch",
        ));
    }
    let mut seen_questions =
        std::collections::HashSet::with_capacity(req.question_version_ids.len());
    if req
        .question_version_ids
        .iter()
        .any(|question| !seen_questions.insert(*question))
    {
        return Err(ApiError::unprocessable(
            "invalid_pack_resources",
            "question version IDs must be unique",
        ));
    }
    let chapter_json = serde_json::to_value(&req.chapters).map_err(|_| ApiError::internal())?;
    let has_lease: bool = sqlx::query_scalar(
        "SELECT EXISTS(
             SELECT 1 FROM pack_leases
             WHERE user_id = $1 AND exam_id = $2 AND device_id = $3
               AND expires_at > now() AND chapters @> $4
         )",
    )
    .bind(user.user_id)
    .bind(exam_id)
    .bind(&req.device_id)
    .bind(chapter_json)
    .fetch_one(&state.pool)
    .await?;
    if !has_lease {
        return Err(ApiError::forbidden(
            "pack_lease_required",
            "an active device lease covering every requested chapter is required",
        ));
    }

    let rows = sqlx::query!(
        r#"SELECT qv.id, qv.vignette, qv.lead_in, qv.difficulty, qv.options,
                  qv.correct_index, qv.key_learning_point, qv.exam_tip, qv.source_ref
           FROM question_versions qv
           JOIN curriculum_nodes c ON c.id = qv.chapter_id
           WHERE qv.id = ANY($1) AND qv.chapter_id = ANY($2)
             AND c.exam_id = $3 AND qv.status = 'published'
           ORDER BY qv.id"#,
        &req.question_version_ids,
        &req.chapters,
        exam_id
    )
    .fetch_all(&state.pool)
    .await?;
    if rows.len() != req.question_version_ids.len() {
        return Err(ApiError::unprocessable(
            "invalid_pack_resources",
            "every requested question must be published in the leased exam chapters",
        ));
    }

    let answered =
        answered_tutor_question_ids(&state.pool, user.user_id, &req.question_version_ids).await?;
    let answered_ids: Vec<_> = answered.iter().copied().collect();
    let mut tutoring_cards = tutoring_cards_for_questions(&state.pool, &answered_ids).await?;
    let mut resources = Vec::with_capacity(rows.len());
    for row in rows {
        resources.push(build_pack_resource(PackQuestionData {
            question_version_id: row.id,
            vignette: row.vignette,
            lead_in: row.lead_in,
            difficulty: row.difficulty,
            options: row.options,
            correct_index: row.correct_index,
            key_learning_point: row.key_learning_point,
            exam_tip: row.exam_tip,
            source_ref: row.source_ref,
            tutoring_cards: tutoring_cards.remove(&row.id).unwrap_or_default(),
        })?);
    }
    // OFF-01: issue and record a verified download receipt for this batch.
    let checksums: Vec<String> = resources.iter().map(|r| r.checksum.clone()).collect();
    let issued_at = chrono::Utc::now();
    let issued_at_rfc3339 = issued_at.to_rfc3339();
    let message =
        pack_download_receipt_message(&req.device_id, exam_id, &issued_at_rfc3339, &checksums);
    let signature = hex(&ed25519_signing_key(&state)?
        .sign(message.as_bytes())
        .to_bytes());
    sqlx::query(
        "INSERT INTO pack_download_receipts
           (id, user_id, exam_id, device_id, checksums, signature, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(user.user_id)
    .bind(exam_id)
    .bind(&req.device_id)
    .bind(json!(checksums))
    .bind(&signature)
    .bind(issued_at)
    .execute(&state.pool)
    .await?;
    let receipt = PackDownloadReceipt {
        device_id: req.device_id.clone(),
        exam_id,
        issued_at: issued_at_rfc3339,
        checksums,
        signature,
    };
    Ok(Json(PackResourcesResponse { resources, receipt }))
}

// ---- OFF-04 / PROT-02 / §22: pack leases ------------------------------------

/// POST /v1/packs/lease — grant or renew the configured offline lease bound to one
/// device. Entitlement-gated (CORE-03): free-tier learners get an honest
/// refusal with the upgrade payload, never a degraded pack. Browser clients
/// import the one-time key as a non-extractable AES-GCM key in IndexedDB.
pub async fn create_lease(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<LeaseReq>,
) -> ApiResult<Json<PackLeaseResponse>> {
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
    let lease_days = crate::routes::settings::current_bounded_i64(
        &state.pool,
        "offline_lease_days",
        crate::routes::settings::DEFAULT_OFFLINE_LEASE_DAYS,
        1,
        30,
    )
    .await?;
    let expires = chrono::Utc::now() + chrono::Duration::days(lease_days);

    let row = sqlx::query!(
        r#"INSERT INTO pack_leases (id, user_id, device_id, exam_id, chapters, pack_key, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT (user_id, device_id, exam_id) DO UPDATE SET
             chapters = $5, pack_key = $6, expires_at = $7, updated_at = now()
           RETURNING id, expires_at, updated_at"#,
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

    Ok(Json(PackLeaseResponse {
        lease_id: row.id,
        expires_at: row.expires_at,
        content_as_of: row.updated_at,
        pack_key,
        algorithm: "AES-GCM-256 (browser-local storage; key shown once per renewal)".into(),
    }))
}

/// GET /v1/me/packs — active leases with freshness disclosure (OFF-04):
/// content_as_of is when the server issued/refreshed this lease. Question
/// versions do not carry publication timestamps, so claiming a newer content
/// timestamp would fabricate precision.
pub async fn list_leases(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<PackLeaseListResponse>> {
    let rows = sqlx::query!(
        r#"SELECT pl.id, pl.exam_id, pl.device_id, pl.chapters, pl.expires_at,
                  pl.updated_at AS "content_as_of!"
           FROM pack_leases pl
           WHERE pl.user_id = $1 AND pl.expires_at > now()
           ORDER BY pl.expires_at"#,
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;
    let mut leases = Vec::with_capacity(rows.len());
    for row in rows {
        let chapters = serde_json::from_value(row.chapters).map_err(|_| ApiError::internal())?;
        leases.push(PackLeaseSummary {
            lease_id: row.id,
            exam_id: row.exam_id,
            device_id: row.device_id,
            chapters,
            expires_at: row.expires_at,
            content_as_of: row.content_as_of,
        });
    }
    Ok(Json(PackLeaseListResponse { leases }))
}

/// DELETE /v1/packs/lease/{lease_id} — revoke. The stored key becomes
/// useless at the next manifest verification (PROT-02).
pub async fn revoke_lease(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(lease_id): Path<Uuid>,
) -> ApiResult<Json<PackLeaseRevokedResponse>> {
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
    Ok(Json(PackLeaseRevokedResponse { revoked: true }))
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
