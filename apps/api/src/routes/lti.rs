//! INST-06: LTI 1.3 tool side (1EdTech Security Framework). A campus LMS
//! (platform) launches learners into this tool: the platform's browser hits
//! `login` (third-party initiated login), we redirect to the platform's OIDC
//! auth endpoint, it form-posts the signed `id_token` back to `launch`, and
//! we verify it against the platform's JWKS and issue the shared app
//! session. Deep-linking responses the platform verifies against OUR JWKS
//! are signed with `LTI_TOOL_PRIVATE_KEY`. Browser form posts receive a
//! same-origin HTML session handoff; API callers continue to receive JSON.

use axum::extract::{Path, Query, State};
use axum::http::header::{self, HeaderValue};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Form, Json};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use url::{Host, Url};
use uuid::Uuid;

use crate::auth::issue_session_with;
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, LtiJwksFuture, LtiJwksTransport};

const LTI_VERSION: &str = "1.3.0";
const MESSAGE_RESOURCE_LINK: &str = "LtiResourceLinkRequest";
const MESSAGE_DEEP_LINK: &str = "LtiDeepLinkingRequest";
const STATE_TTL_MINUTES: i32 = 10;
const DEEP_LINK_TTL_MINUTES: i32 = 30;
const TOOL_KID: &str = "medicalos-lti-1";
const JWKS_FETCH_TIMEOUT: Duration = Duration::from_secs(10);
const JWKS_MAX_BYTES: usize = 1024 * 1024;

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
    headers: HeaderMap,
    Form(params): Form<LaunchParams>,
) -> ApiResult<Response> {
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
        state.lti_jwks_transport.as_ref(),
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

    let response = json!({
        "token": token,
        "user_id": user_id,
        "message_type": message_type,
        "target_link_uri": row.target_link_uri,
        "resource_link": payload.get(lti_claim("resource_link").as_str()),
        "roles": payload.get(lti_claim("roles").as_str()),
        "deep_linking_settings": deep_link_settings,
    });
    if accepts_html(
        headers
            .get(header::ACCEPT)
            .and_then(|value| value.to_str().ok()),
    ) {
        handoff_response(&response)
    } else {
        Ok(Json(response).into_response())
    }
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
    Ok(Json(json!({
        "keys": [{
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "kid": TOOL_KID,
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
    validate_deep_link_items(&req.items)?;
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
    state
        .lti_jwks_transport
        .validate_key_set_url(req.key_set_url.trim())?;
    for (value, code) in [
        (&req.issuer, "invalid_issuer"),
        (&req.client_id, "invalid_client_id"),
        (&req.deployment_id, "invalid_deployment_id"),
        (&req.auth_login_url, "invalid_auth_login_url"),
    ] {
        let value = value.trim();
        let is_url = code.ends_with("login_url") || code == "invalid_issuer";
        let ok = !value.is_empty()
            && value.len() <= 500
            && (!is_url || value.starts_with("https://") || value.starts_with("http://"));
        if !ok {
            return Err(ApiError::unprocessable(code, format!("{code} rejected")));
        }
    }
    let platform_id = Uuid::new_v4();
    let registered = sqlx::query!(
        "INSERT INTO lti_platforms (id, institution_id, issuer, client_id, deployment_id,
                                    auth_login_url, key_set_url, display_name)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (issuer, client_id, deployment_id) DO UPDATE SET
            auth_login_url = $6, key_set_url = $7, display_name = $8
         WHERE lti_platforms.institution_id = EXCLUDED.institution_id
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
    .fetch_optional(&state.pool)
    .await?;
    let platform_id = registered.map(|row| row.id).ok_or_else(|| {
        ApiError::conflict(
            "lti_platform_conflict",
            "an LTI platform with these identifiers is already registered",
        )
    })?;
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
    transport: &dyn LtiJwksTransport,
    _pem: &str,
    id_token: &str,
    issuer: &str,
    client_id: &str,
    key_set_url: &str,
) -> ApiResult<Value> {
    let jwks = transport.fetch_jwks(key_set_url).await?;

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

/// Production transport. Registration and fetching require HTTPS; fetch
/// resolves once, rejects every non-public answer, pins those addresses,
/// disables proxies and redirects, and caps the streamed response body.
pub struct GuardedHttpsJwksTransport;

impl LtiJwksTransport for GuardedHttpsJwksTransport {
    fn validate_key_set_url(&self, raw: &str) -> ApiResult<()> {
        parse_jwks_url(raw).map(|_| ())
    }

    fn fetch_jwks<'a>(&'a self, raw: &'a str) -> LtiJwksFuture<'a> {
        Box::pin(async move {
            tokio::time::timeout(JWKS_FETCH_TIMEOUT, async {
                let url = parse_jwks_url(raw)?;
                let (domain, addresses) = resolve_public_jwks_host(&url).await?;
                let mut builder = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .no_proxy();
                if let Some(domain) = domain {
                    builder = builder.resolve_to_addrs(&domain, &addresses);
                }
                let client = builder.build().map_err(|_| ApiError::internal())?;
                let mut response = client
                    .get(url)
                    .send()
                    .await
                    .map_err(|_| key_set_unreachable())?;
                if !response.status().is_success() {
                    return Err(key_set_unreachable());
                }
                if response
                    .content_length()
                    .is_some_and(|length| length > JWKS_MAX_BYTES as u64)
                {
                    return Err(key_set_too_large());
                }

                let capacity = response
                    .content_length()
                    .unwrap_or_default()
                    .min(JWKS_MAX_BYTES as u64) as usize;
                let mut body = Vec::with_capacity(capacity);
                while let Some(chunk) = response.chunk().await.map_err(|_| key_set_unreachable())? {
                    if body.len().saturating_add(chunk.len()) > JWKS_MAX_BYTES {
                        return Err(key_set_too_large());
                    }
                    body.extend_from_slice(&chunk);
                }
                serde_json::from_slice(&body).map_err(|_| {
                    ApiError::unprocessable(
                        "invalid_key_set",
                        "the platform's JWKS is not valid JSON",
                    )
                })
            })
            .await
            .map_err(|_| key_set_unreachable())?
        })
    }
}

fn parse_jwks_url(raw: &str) -> ApiResult<Url> {
    let invalid = || {
        ApiError::unprocessable(
            "invalid_key_set_url",
            "key_set_url must use HTTPS and identify a public host",
        )
    };
    if raw.is_empty() || raw.len() > 500 {
        return Err(invalid());
    }
    let url = Url::parse(raw).map_err(|_| invalid())?;
    if url.scheme() != "https"
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    if let Some(address) = match url.host() {
        Some(Host::Ipv4(address)) => Some(IpAddr::V4(address)),
        Some(Host::Ipv6(address)) => Some(IpAddr::V6(address)),
        Some(Host::Domain(_)) | None => None,
    } {
        if !is_public_ip(address) {
            return Err(invalid());
        }
    }
    Ok(url)
}

async fn resolve_public_jwks_host(url: &Url) -> ApiResult<(Option<String>, Vec<SocketAddr>)> {
    let port = url.port_or_known_default().ok_or_else(|| {
        ApiError::unprocessable("invalid_key_set_url", "key_set_url must use HTTPS")
    })?;
    let (domain, addresses) = match url.host().ok_or_else(|| {
        ApiError::unprocessable(
            "invalid_key_set_url",
            "key_set_url must identify a public host",
        )
    })? {
        Host::Domain(domain) => {
            let addresses = tokio::net::lookup_host((domain, port))
                .await
                .map_err(|_| key_set_unreachable())?
                .collect::<Vec<_>>();
            (Some(domain.to_owned()), addresses)
        }
        Host::Ipv4(address) => (None, vec![SocketAddr::new(address.into(), port)]),
        Host::Ipv6(address) => (None, vec![SocketAddr::new(address.into(), port)]),
    };
    if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(address.ip())) {
        return Err(ApiError::unprocessable(
            "invalid_key_set_url",
            "key_set_url must resolve only to public addresses",
        ));
    }
    Ok((domain, addresses))
}

fn is_public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            let octets = address.octets();
            !address.is_private()
                && !address.is_loopback()
                && !address.is_link_local()
                && !address.is_unspecified()
                && !address.is_multicast()
                && !address.is_broadcast()
                && !address.is_documentation()
                && octets[0] != 0
                && !(octets[0] == 100 && (64..=127).contains(&octets[1]))
                // IANA's IPv4 registry (https://www.iana.org/assignments/iana-ipv4-special-registry)
                // marks 192.0.0.0/24 special-purpose, with .9 and .10 globally
                // reachable. Don't exclude neighboring 192.0.1.0/24 through
                // 192.0.255.0/24.
                && !(octets[0] == 192
                    && ((octets[1] == 0
                        && octets[2] == 0
                        && !matches!(octets[3], 9 | 10))
                        || (octets[1] == 88 && octets[2] == 99)))
                && !(octets[0] == 198 && matches!(octets[1], 18 | 19))
                && octets[0] < 224
        }
        IpAddr::V6(address) => {
            let segments = address.segments();
            segments[0] & 0xe000 == 0x2000
                && !(segments[0] == 0x2001 && segments[1] <= 0x01ff)
                && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
                && segments[0] != 0x2002
                && !(segments[0] == 0x3fff && segments[1] <= 0x0fff)
        }
    }
}

fn key_set_unreachable() -> ApiError {
    ApiError::unprocessable(
        "key_set_unreachable",
        "the platform's JWKS could not be fetched",
    )
}

fn key_set_too_large() -> ApiError {
    ApiError::unprocessable(
        "key_set_too_large",
        "the platform's JWKS exceeds the size limit",
    )
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

/// Percent-encode for query values (RFC 3986 unreserved set passes).
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

fn form_encode(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn accepts_html(accept: Option<&str>) -> bool {
    let mut html_quality = None;
    let mut json_quality = None;
    let mut application_quality = None;
    let mut wildcard_quality = None;

    for media_range in accept.unwrap_or_default().split(',') {
        let mut parts = media_range.split(';');
        let media_type = parts.next().unwrap_or_default().trim();
        let quality = parts
            .filter_map(|parameter| parameter.trim().split_once('='))
            .find_map(|(name, value)| {
                name.trim().eq_ignore_ascii_case("q").then(|| {
                    value
                        .trim()
                        .parse::<f32>()
                        .ok()
                        .filter(|quality| quality.is_finite() && (0.0..=1.0).contains(quality))
                        .unwrap_or(0.0)
                })
            })
            .unwrap_or(1.0);
        if media_type.eq_ignore_ascii_case("text/html") {
            html_quality = Some(quality);
        } else if media_type.eq_ignore_ascii_case("application/json") {
            json_quality = Some(quality);
        } else if media_type.eq_ignore_ascii_case("application/*") {
            application_quality = Some(quality);
        } else if media_type == "*/*" {
            wildcard_quality = Some(quality);
        }
    }

    let Some(html_quality) = html_quality else {
        return false;
    };
    let json_quality = json_quality
        .or(application_quality)
        .or(wildcard_quality)
        .unwrap_or(0.0);
    html_quality > 0.0 && html_quality >= json_quality
}

fn handoff_response(payload: &Value) -> ApiResult<Response> {
    let token = payload
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::internal)?;
    let message_type = payload
        .get("message_type")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::internal)?;
    let next = if message_type == MESSAGE_DEEP_LINK {
        "/lti/deep-links"
    } else {
        "/lti/handoff"
    };
    let context = json!({
        "message_type": message_type,
        "resource_link": payload.get("resource_link"),
        "deep_linking_settings": payload.get("deep_linking_settings"),
    });
    let bootstrap = script_safe_json(&json!({
        "token": token,
        "context": context,
        "next": next,
    }))?;
    let nonce = random_token();
    let body = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Opening Medical OS</title></head><body><p role=\"status\">Opening Medical OS…</p><script nonce=\"{nonce}\">const launch={bootstrap};try{{localStorage.setItem('mlos_token',launch.token);try{{sessionStorage.setItem('mlos_lti_launch',JSON.stringify(launch.context))}}catch{{}}window.location.replace(launch.next)}}catch{{document.body.textContent='Medical OS could not save this session in browser storage. Allow site storage, then launch again from your LMS.'}}</script></body></html>"
    );
    let policy = format!(
        "default-src 'none'; script-src 'nonce-{nonce}'; style-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors https:"
    );
    let mut response = Response::new(axum::body::Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_str(&policy).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

fn script_safe_json(value: &Value) -> ApiResult<String> {
    let encoded = serde_json::to_string(value).map_err(|_| ApiError::internal())?;
    Ok(encoded
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}

fn validate_deep_link_items(items: &[Value]) -> ApiResult<()> {
    if items.len() > 50 {
        return Err(ApiError::unprocessable(
            "invalid_items",
            "deep linking supports at most 50 content items",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod browser_handoff_tests {
    use super::*;
    use axum::http::header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE};

    #[test]
    fn negotiates_html_only_for_an_html_navigation() {
        assert!(accepts_html(Some(
            "text/html,application/xhtml+xml,*/*;q=0.8"
        )));
        assert!(!accepts_html(Some("application/json, text/html;q=0.2")));
        assert!(!accepts_html(Some("text/html;q=invalid")));
        assert!(!accepts_html(Some("text/html;q=0.2,*/*;q=0.8")));
        assert!(!accepts_html(Some("application/json")));
        assert!(!accepts_html(Some("text/html;q=0")));
        assert!(!accepts_html(None));
    }

    #[tokio::test]
    async fn handoff_document_keeps_token_out_of_navigation_and_escapes_claims() {
        let payload = json!({
            "token": "browser-session-secret",
            "message_type": MESSAGE_RESOURCE_LINK,
            "resource_link": { "title": "</script><img src=x onerror=alert(1)>" },
            "deep_linking_settings": null
        });
        let response = handoff_response(&payload).expect("valid launch context builds a handoff");

        assert_eq!(response.headers()[CONTENT_TYPE], "text/html; charset=utf-8");
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        let policy = response.headers()[CONTENT_SECURITY_POLICY]
            .to_str()
            .expect("CSP header is text")
            .to_owned();
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("handoff body is readable")
            .to_bytes();
        let body = String::from_utf8(body.to_vec()).expect("handoff document is UTF-8");

        assert!(body.contains("browser-session-secret"));
        assert!(body.contains("localStorage.setItem"));
        assert!(body.contains("location.replace"));
        assert!(body.contains("/lti/handoff"));
        assert!(!body.contains("/lti/handoff?token="));
        assert!(!body.contains("</script><img"));
        let nonce = body
            .split("<script nonce=\"")
            .nth(1)
            .and_then(|part| part.split('\"').next())
            .expect("script has a CSP nonce");
        assert!(policy.contains(&format!("'nonce-{nonce}'")));
        assert!(policy.contains("frame-ancestors https:"));
    }

    #[tokio::test]
    async fn deep_link_launch_handoff_opens_picker_with_platform_settings() {
        let response = handoff_response(&json!({
            "token": "browser-session-secret",
            "message_type": MESSAGE_DEEP_LINK,
            "deep_linking_settings": {
                "deep_link_return_url": "https://lms.example.test/return",
                "accept_types": ["ltiResourceLink"],
                "data": "opaque-platform-state"
            }
        }))
        .expect("valid deep-link launch builds a handoff");
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("handoff body is readable")
            .to_bytes();
        let body = String::from_utf8(body.to_vec()).expect("handoff document is UTF-8");

        assert!(body.contains("/lti/deep-links"));
        assert!(body.contains("https://lms.example.test/return"));
        assert!(body.contains("opaque-platform-state"));
    }

    #[test]
    fn deep_link_response_allows_cancel_but_caps_content_items() {
        assert!(validate_deep_link_items(&[]).is_ok());
        assert!(validate_deep_link_items(&vec![json!({}); 50]).is_ok());
        assert!(validate_deep_link_items(&vec![json!({}); 51]).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::{is_public_ip, parse_jwks_url};
    use std::net::IpAddr;
    use std::str::FromStr;

    #[test]
    fn jwks_url_requires_https_without_credentials_or_private_ip_literals() {
        for url in [
            "http://lms.example.test/jwks",
            "https://127.0.0.1/jwks",
            "https://10.1.2.3/jwks",
            "https://169.254.10.20/jwks",
            "https://[::1]/jwks",
            "https://[fd00::1]/jwks",
            "https://[::ffff:127.0.0.1]/jwks",
            "https://user:password@lms.example.test/jwks",
            "https://lms.example.test/jwks#fragment",
        ] {
            assert!(parse_jwks_url(url).is_err(), "accepted unsafe URL {url}");
        }
        assert!(parse_jwks_url("https://lms.example.test/jwks").is_ok());
    }

    #[test]
    fn dns_answers_must_be_public_unicast_addresses() {
        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "192.0.2.1",
            "192.0.0.1",
            "192.0.0.8",
            "192.0.0.170",
            "192.168.1.1",
            "198.18.0.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "2001:db8::1",
            "2002::1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
        ] {
            let address = IpAddr::from_str(address).expect("valid test IP");
            assert!(!is_public_ip(address), "accepted non-public IP {address}");
        }
        for address in [
            "8.8.8.8",
            "192.0.0.9",
            "192.0.0.10",
            "192.0.1.1",
            "192.0.255.255",
            "2606:4700:4700::1111",
        ] {
            let address = IpAddr::from_str(address).expect("valid test IP");
            assert!(is_public_ip(address), "rejected public IP {address}");
        }
    }
}
