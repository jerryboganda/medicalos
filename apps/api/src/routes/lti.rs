//! INST-06: LTI 1.3 tool side (1EdTech Security Framework). A campus LMS
//! (platform) launches learners into this tool: the platform's browser hits
//! `login` (third-party initiated login), we redirect to the platform's OIDC
//! auth endpoint, it form-posts the signed `id_token` back to `launch`, and
//! we verify it against the platform's JWKS and issue the shared app
//! session. Deep-linking responses the platform verifies against OUR JWKS
//! are signed with `LTI_TOOL_PRIVATE_KEY`. No UI here — launches answer
//! JSON; a browser handoff page is a UI-gated follow-up.

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Form, Json};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::issue_session_with;
use crate::authz::Permission;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const LTI_VERSION: &str = "1.3.0";
const MESSAGE_RESOURCE_LINK: &str = "LtiResourceLinkRequest";
const MESSAGE_DEEP_LINK: &str = "LtiDeepLinkingRequest";
const STATE_TTL_MINUTES: i32 = 10;
const DEEP_LINK_TTL_MINUTES: i32 = 30;
const TOOL_KID: &str = "medicalos-lti-1";

fn lti_claim(name: &str) -> String {
    format!("https://purl.imsglobal.org/spec/lti/claim/{name}")
}

/// LTI 1.3 is off until the operator provisions the tool's signing key.
fn tool_key(state: &AppState) -> ApiResult<&str> {
    state
        .lti_tool_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| ApiError::not_found("lti_unavailable"))
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Deserialize)]
pub struct LoginParams {
    pub iss: String,
    pub login_hint: String,
    #[serde(default)]
    pub target_link_uri: Option<String>,
    #[serde(default)]
    pub lti_message_hint: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub lti_deployment_id: Option<String>,
}

async fn login_initiation(state: State<Arc<AppState>>, params: LoginParams) -> ApiResult<Response> {
    tool_key(&state)?;
    let platform = sqlx::query!(
        r#"SELECT id, auth_login_url, client_id FROM lti_platforms
           WHERE issuer = $1 AND ($2::text IS NULL OR client_id = $2)
           ORDER BY created_at LIMIT 1"#,
        params.iss,
        params.client_id.as_deref().filter(|c| !c.trim().is_empty()),
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ApiError::unprocessable(
            "unknown_platform",
            "issuer is not a registered LTI platform",
        )
    })?;

    let state_value = random_token();
    let nonce = random_token();
    sqlx::query!(
        "INSERT INTO lti_launch_states (state, nonce, platform_id, target_link_uri, message_type, expires_at)
         VALUES ($1, $2, $3, $4, $5, now() + make_interval(mins => $6))",
        state_value,
        nonce,
        platform.id,
        params.target_link_uri,
        MESSAGE_RESOURCE_LINK,
        STATE_TTL_MINUTES,
    )
    .execute(&state.pool)
    .await?;

    let redirect_uri = format!(
        "{}/v1/lti/launch",
        state.public_api_base_url.trim_end_matches('/')
    );
    let query = form_encode(&[
        ("scope", "openid"),
        ("response_type", "id_token"),
        ("response_mode", "form_post"),
        ("prompt", "none"),
        ("client_id", &platform.client_id),
        ("redirect_uri", &redirect_uri),
        ("login_hint", &params.login_hint),
        ("state", &state_value),
        ("nonce", &nonce),
        (
            "lti_message_hint",
            params.lti_message_hint.as_deref().unwrap_or(""),
        ),
    ]);
    Ok(Redirect::to(&format!(
        "{}?{query}",
        platform.auth_login_url.trim_end_matches('?')
    ))
    .into_response())
}

pub async fn login_get(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LoginParams>,
) -> ApiResult<Response> {
    login_initiation(State(state), params).await
}

pub async fn login_post(
    State(state): State<Arc<AppState>>,
    Form(params): Form<LoginParams>,
) -> ApiResult<Response> {
    login_initiation(State(state), params).await
}

#[derive(Deserialize)]
pub struct LaunchParams {
    pub id_token: String,
    pub state: String,
}

/// Resource-link launch: consume the one-use state, verify the platform's
/// id_token against its JWKS, link the LMS identity once, issue a session.
pub async fn launch(
    State(state): State<Arc<AppState>>,
    Form(params): Form<LaunchParams>,
) -> ApiResult<Json<Value>> {
    let pem = tool_key(&state)?;

    // Single-use: delete-first makes replay impossible even under races.
    let row = sqlx::query!(
        "DELETE FROM lti_launch_states WHERE state = $1 AND expires_at > now()
         RETURNING nonce, platform_id, target_link_uri, message_type",
        params.state,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;

    let platform = sqlx::query!(
        "SELECT id, institution_id, issuer, client_id, deployment_id, key_set_url
         FROM lti_platforms WHERE id = $1",
        row.platform_id,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;

    let payload = verify_platform_token(
        pem,
        &params.id_token,
        &platform.issuer,
        &platform.client_id,
        &platform.key_set_url,
    )
    .await?;

    // The nonce echo must match the one we stored at login initiation.
    if payload["nonce"].as_str() != Some(row.nonce.as_str()) {
        return Err(ApiError::unauthorized());
    }
    // The message type is only known from the id_token; both first-class
    // messages may arrive over one login initiation.
    let message_type = payload[&lti_claim("message_type")]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if message_type != MESSAGE_RESOURCE_LINK && message_type != MESSAGE_DEEP_LINK {
        return Err(ApiError::unprocessable(
            "unsupported_message_type",
            "only resource-link and deep-linking launches are supported",
        ));
    }
    let _ = &row;
    if payload[&lti_claim("version")].as_str() != Some(LTI_VERSION) {
        return Err(ApiError::unprocessable(
            "unsupported_version",
            "only LTI 1.3.0 launches are supported",
        ));
    }
    if payload[&lti_claim("deployment_id")].as_str() != Some(platform.deployment_id.as_str()) {
        return Err(ApiError::unprocessable(
            "deployment_mismatch",
            "launch deployment id is not registered",
        ));
    }

    let subject = payload["sub"]
        .as_str()
        .ok_or_else(ApiError::unauthorized)?
        .to_owned();
    let user_id = link_identity(
        &state,
        platform.id,
        platform.institution_id,
        &subject,
        &payload,
    )
    .await?;
    let token = issue_session_with(&state.pool, user_id, &[], false).await?;

    // Deep-linking requests park the platform's settings against the staff
    // user; the signed response is built later by POST /v1/lti/deep-links.
    let deep_link_settings = payload
        .get(lti_claim("deep_linking_settings").as_str())
        .cloned();
    if message_type == MESSAGE_DEEP_LINK {
        sqlx::query!(
            "INSERT INTO lti_deep_link_pends (user_id, platform_id, settings, expires_at)
             VALUES ($1, $2, $3, now() + make_interval(mins => $4))
             ON CONFLICT (user_id) DO UPDATE SET
                platform_id = $2, settings = $3, expires_at = now() + make_interval(mins => $4)",
            user_id,
            platform.id,
            deep_link_settings,
            DEEP_LINK_TTL_MINUTES,
        )
        .execute(&state.pool)
        .await?;
    }

    Ok(Json(json!({
        "token": token,
        "user_id": user_id,
        "message_type": message_type,
        "target_link_uri": row.target_link_uri,
        "resource_link": payload.get(lti_claim("resource_link").as_str()),
        "roles": payload.get(lti_claim("roles").as_str()),
        "deep_linking_settings": deep_link_settings,
    })))
}

/// Tool JWKS: the public half the platform uses to verify our deep-linking
/// responses. kid is deterministic (short SHA-256 of the modulus).
pub async fn jwks(State(state): State<Arc<AppState>>) -> ApiResult<impl IntoResponse> {
    let pem = tool_key(&state)?;
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::traits::PublicKeyParts;
    let key = rsa::RsaPrivateKey::from_pkcs8_pem(pem).map_err(|_| ApiError::internal())?;
    let public = key.to_public_key();
    let n = public.n().to_bytes_be();
    let e = public.e().to_bytes_be();
    let kid = format!("{:x}", Sha256::digest(&n))[..16].to_string();
    Ok(Json(json!({
        "keys": [{
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "kid": kid,
            "n": URL_SAFE_NO_PAD.encode(n),
            "e": URL_SAFE_NO_PAD.encode(e),
        }]
    })))
}

#[derive(Deserialize)]
pub struct DeepLinkReq {
    /// Content items to hand back to the platform, each an ltiResourceLink.
    pub items: Vec<Value>,
}

/// Sign the LtiDeepLinkingResponse over the submitted content items,
/// addressed to the return URL the platform gave at launch (never taken
/// from the request).
pub async fn deep_links(
    State(state): State<Arc<AppState>>,
    user: crate::auth::AuthUser,
    Json(req): Json<DeepLinkReq>,
) -> ApiResult<Json<Value>> {
    let pem = tool_key(&state)?;
    if req.items.is_empty() || req.items.len() > 50 {
        return Err(ApiError::unprocessable(
            "invalid_items",
            "deep linking needs 1-50 content items",
        ));
    }
    let pend = sqlx::query!(
        "DELETE FROM lti_deep_link_pends WHERE user_id = $1 AND expires_at > now()
         RETURNING platform_id, settings",
        user.user_id,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ApiError::unprocessable(
            "no_deep_link_request",
            "launch an LtiDeepLinkingRequest first",
        )
    })?;
    let platform = sqlx::query!(
        "SELECT issuer, client_id FROM lti_platforms WHERE id = $1",
        pend.platform_id,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::internal)?;

    let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(pem.as_bytes())
        .map_err(|_| ApiError::internal())?;

    let settings = &pend.settings;
    let return_url = settings["deep_link_return_url"].as_str().ok_or_else(|| {
        ApiError::unprocessable(
            "invalid_deep_link_settings",
            "the launch did not carry a deep_link_return_url",
        )
    })?;
    let accepted = settings["accept_types"].as_array().map(|types| {
        types
            .iter()
            .filter_map(Value::as_str)
            .any(|t| t == "ltiResourceLink")
    });
    if accepted != Some(true) {
        return Err(ApiError::unprocessable(
            "unsupported_deep_link_type",
            "the platform did not accept ltiResourceLink content",
        ));
    }
    let items: Vec<Value> = req
        .items
        .iter()
        .map(|item| {
            let mut with_type = item.clone();
            if with_type.get("type").is_none() {
                with_type["type"] = json!("ltiResourceLink");
            }
            with_type
        })
        .collect();

    let claims = json!({
        "iss": platform.client_id,
        "aud": platform.client_id,
        "exp": (Utc::now() + chrono::Duration::minutes(5)).timestamp(),
        "iat": Utc::now().timestamp(),
        "nonce": random_token(),
        &lti_claim("message_type"): "LtiDeepLinkingResponse",
        &lti_claim("version"): LTI_VERSION,
        &lti_claim("data"): settings.get("data").cloned().unwrap_or(Value::Null),
        &lti_claim("content_items"): items,
    });
    let header = jsonwebtoken::Header {
        kid: Some(TOOL_KID.to_string()),
        ..jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256)
    };
    let jwt =
        jsonwebtoken::encode(&header, &claims, &encoding_key).map_err(|_| ApiError::internal())?;

    Ok(Json(json!({
        "post_url": return_url,
        "jwt": jwt,
        "platform_issuer": platform.issuer,
        "deployed_at": Utc::now().to_rfc3339(),
    })))
}

/// Register a campus LMS as an LTI 1.3 platform for one institution.
#[derive(Deserialize)]
pub struct RegisterPlatformReq {
    pub issuer: String,
    pub client_id: String,
    pub deployment_id: String,
    pub auth_login_url: String,
    pub key_set_url: String,
    #[serde(default = "default_display_name")]
    pub display_name: String,
}

fn default_display_name() -> String {
    "LMS".into()
}

#[derive(serde::Serialize)]
pub struct RegisteredPlatform {
    pub platform_id: Uuid,
    pub institution_id: Uuid,
    pub issuer: String,
    pub client_id: String,
    pub deployment_id: String,
}

pub async fn register_platform(
    State(state): State<Arc<AppState>>,
    user: crate::auth::AuthUser,
    Path(institution_id): Path<Uuid>,
    Json(req): Json<RegisterPlatformReq>,
) -> ApiResult<Json<RegisteredPlatform>> {
    // The institution surface's own pattern (as add_member): the creator /
    // institution admin role decides, checked directly against membership.
    let staff = sqlx::query!(
        "SELECT 1 AS one FROM institution_members
         WHERE institution_id = $1 AND user_id = $2 AND role = 'admin'",
        institution_id,
        user.user_id
    )
    .fetch_optional(&state.pool)
    .await?;
    if staff.is_none() {
        return Err(ApiError::forbidden(
            "admin_required",
            "LTI platforms are registered by the institution's admins",
        ));
    }
    for (value, code) in [
        (&req.issuer, "invalid_issuer"),
        (&req.client_id, "invalid_client_id"),
        (&req.deployment_id, "invalid_deployment_id"),
        (&req.auth_login_url, "invalid_auth_login_url"),
        (&req.key_set_url, "invalid_key_set_url"),
    ] {
        let ok = !value.trim().is_empty()
            && value.len() <= 500
            && (code.starts_with("invalid_auth")
                || code.starts_with("invalid_key")
                || value.starts_with("https://"));
        if !ok {
            return Err(ApiError::unprocessable(code, format!("{code} rejected")));
        }
    }
    let platform_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO lti_platforms (id, institution_id, issuer, client_id, deployment_id,
                                    auth_login_url, key_set_url, display_name)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (issuer, client_id, deployment_id) DO UPDATE SET
            institution_id = $2, auth_login_url = $6, key_set_url = $7, display_name = $8
         RETURNING id",
        platform_id,
        institution_id,
        req.issuer.trim(),
        req.client_id.trim(),
        req.deployment_id.trim(),
        req.auth_login_url.trim(),
        req.key_set_url.trim(),
        req.display_name.trim(),
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(RegisteredPlatform {
        platform_id,
        institution_id,
        issuer: req.issuer,
        client_id: req.client_id,
        deployment_id: req.deployment_id,
    }))
}

// ---- internals ---------------------------------------------------------------

async fn verify_platform_token(
    _pem: &str,
    id_token: &str,
    issuer: &str,
    client_id: &str,
    key_set_url: &str,
) -> ApiResult<Value> {
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| ApiError::internal())?
        .get(key_set_url)
        .send()
        .await
        .map_err(|_| {
            ApiError::unprocessable(
                "key_set_unreachable",
                "the platform's JWKS could not be fetched",
            )
        })?;
    let jwks: jsonwebtoken::jwk::JwkSet = response.json().await.map_err(|_| {
        ApiError::unprocessable("invalid_key_set", "the platform's JWKS is not valid JSON")
    })?;

    let header = jsonwebtoken::decode_header(id_token).map_err(|_| ApiError::unauthorized())?;
    if header.alg != jsonwebtoken::Algorithm::RS256 {
        return Err(ApiError::unauthorized());
    }
    let kid = header.kid.as_deref();
    let jwk = jwks
        .keys
        .iter()
        .find(|jwk| kid.is_some() && jwk.common.key_id.as_deref() == kid)
        .or_else(|| jwks.keys.first())
        .ok_or_else(ApiError::unauthorized)?;
    let decoding_key =
        jsonwebtoken::DecodingKey::from_jwk(jwk).map_err(|_| ApiError::unauthorized())?;
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(&[issuer]);
    validation.set_audience(&[client_id]);
    let token_data = jsonwebtoken::decode::<Value>(id_token, &decoding_key, &validation)
        .map_err(|_| ApiError::unauthorized())?;
    Ok(token_data.claims)
}

/// Link (once) the LMS subject to an app account: the linked row wins, then
/// a verified-email match that is a member of the platform's institution.
/// Never creates accounts and never matches unverified addresses.
async fn link_identity(
    state: &AppState,
    platform_id: Uuid,
    institution_id: Uuid,
    subject: &str,
    payload: &Value,
) -> ApiResult<Uuid> {
    let linked = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM lti_identities WHERE platform_id = $1 AND subject = $2",
    )
    .bind(platform_id)
    .bind(subject)
    .fetch_optional(&state.pool)
    .await?;
    if let Some(user_id) = linked {
        return Ok(user_id);
    }
    let email = payload["email"]
        .as_str()
        .map(str::to_lowercase)
        .filter(|_| payload["email_verified"].as_bool() == Some(true))
        .ok_or_else(|| {
            ApiError::forbidden(
                "launch_unlinked",
                "this LMS account has no verified email and is not linked to a medicalos account",
            )
        })?;
    let member = sqlx::query_scalar::<_, Uuid>(
        "SELECT u.id FROM users u
         JOIN institution_members im ON im.user_id = u.id AND im.institution_id = $2
         WHERE u.email = $1 AND u.deleted_at IS NULL",
    )
    .bind(&email)
    .bind(institution_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ApiError::forbidden(
            "launch_unlinked",
            "no medicalos account with that verified email is a member of the launching institution",
        )
    })?;
    sqlx::query!(
        "INSERT INTO lti_identities (platform_id, subject, user_id) VALUES ($1, $2, $3)
         ON CONFLICT (platform_id, subject) DO NOTHING",
        platform_id,
        subject,
        member,
    )
    .execute(&state.pool)
    .await?;
    Ok(member)
}

fn form_encode(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", URL_SAFE_NO_PAD.encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}
