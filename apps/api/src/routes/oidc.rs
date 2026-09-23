//! INST-03: institution-scoped OIDC Authorization Code login.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Redirect;
use axum::Json;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::reqwest;
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce as OidcNonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, TokenResponse,
};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use url::{Host, Url};
use uuid::Uuid;

use crate::auth::{issue_session, new_session_token, sha256_hex, AuthUser};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ConfigureOidcReq {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    #[serde(default)]
    pub clear_client_secret: bool,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct CompleteOidcReq {
    pub ticket: String,
}

#[derive(Deserialize)]
pub struct OidcCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

fn require_admin(state: &AppState, headers: &HeaderMap) -> ApiResult<()> {
    state.require_admin(
        headers
            .get("x-admin-token")
            .and_then(|value| value.to_str().ok()),
    )
}

fn secure_or_loopback(url: &Url) -> bool {
    match url.scheme() {
        "https" => true,
        "http" => match url.host() {
            Some(Host::Ipv4(address)) => address.is_loopback(),
            Some(Host::Ipv6(address)) => address.is_loopback(),
            Some(Host::Domain("localhost")) => true,
            _ => false,
        },
        _ => false,
    }
}

fn validate_issuer(issuer: &str) -> ApiResult<()> {
    let url = Url::parse(issuer).map_err(|_| {
        ApiError::unprocessable("invalid_oidc_issuer", "issuer must be a valid HTTPS URL")
    })?;
    if issuer.len() > 2048
        || !secure_or_loopback(&url)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        return Err(ApiError::unprocessable(
            "invalid_oidc_issuer",
            "issuer must be an HTTPS URL (HTTP is allowed only on loopback) without credentials, query, or fragment",
        ));
    }
    IssuerUrl::new(issuer.to_owned()).map_err(|_| {
        ApiError::unprocessable("invalid_oidc_issuer", "issuer URL is not valid for OIDC")
    })?;
    Ok(())
}

fn callback_uri(state: &AppState, institution_id: Uuid) -> ApiResult<String> {
    let mut url = Url::parse(&state.public_api_base_url).map_err(|_| ApiError::internal())?;
    if !secure_or_loopback(&url)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
        || url.path().contains('%')
    {
        return Err(ApiError::internal());
    }
    let path = format!(
        "{}/v1/auth/oidc/callback/{institution_id}",
        url.path().trim_end_matches('/')
    );
    url.set_path(&path);
    Ok(url.to_string())
}

fn frontend_callback_uri(state: &AppState, ticket: Option<&str>) -> ApiResult<String> {
    let mut url = Url::parse(&state.public_app_url).map_err(|_| ApiError::internal())?;
    if !secure_or_loopback(&url)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        return Err(ApiError::internal());
    }
    url.set_path("/login/sso/callback");
    if let Some(ticket) = ticket {
        url.set_fragment(Some(&format!("ticket={ticket}")));
    } else {
        url.set_query(Some("error=sso_failed"));
    }
    Ok(url.to_string())
}

fn oidc_http_client() -> ApiResult<reqwest::Client> {
    reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|_| ApiError::internal())
}

fn credential_cipher(key: &str) -> ApiResult<ChaCha20Poly1305> {
    if key.len() < 32 {
        return Err(ApiError::unprocessable(
            "oidc_encryption_unavailable",
            "OIDC_CREDENTIAL_KEY must contain at least 32 bytes",
        ));
    }
    let mut input = b"medical-os-oidc-client-secret\0".to_vec();
    input.extend_from_slice(key.as_bytes());
    let derived = Sha256::digest(input);
    ChaCha20Poly1305::new_from_slice(&derived).map_err(|_| ApiError::internal())
}

fn secret_aad(institution_id: Uuid, issuer: &str) -> Vec<u8> {
    format!("{institution_id}:{issuer}").into_bytes()
}

fn encrypt_secret(
    key: &str,
    institution_id: Uuid,
    issuer: &str,
    secret: &str,
) -> ApiResult<Vec<u8>> {
    let cipher = credential_cipher(key)?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: secret.as_bytes(),
                aad: &secret_aad(institution_id, issuer),
            },
        )
        .map_err(|_| ApiError::internal())?;
    let mut stored = nonce_bytes.to_vec();
    stored.extend_from_slice(&encrypted);
    Ok(stored)
}

fn decrypt_secret(
    key: &str,
    institution_id: Uuid,
    issuer: &str,
    encrypted: &[u8],
) -> ApiResult<String> {
    if encrypted.len() < 28 {
        return Err(ApiError::internal());
    }
    let cipher = credential_cipher(key).map_err(|_| ApiError::internal())?;
    let decrypted = cipher
        .decrypt(
            Nonce::from_slice(&encrypted[..12]),
            Payload {
                msg: &encrypted[12..],
                aad: &secret_aad(institution_id, issuer),
            },
        )
        .map_err(|_| ApiError::internal())?;
    String::from_utf8(decrypted).map_err(|_| ApiError::internal())
}

fn provider_view(
    issuer: String,
    client_id: String,
    enabled: bool,
    secret_configured: bool,
) -> Value {
    json!({
        "issuer": issuer,
        "client_id": client_id,
        "enabled": enabled,
        "client_secret_configured": secret_configured
    })
}

pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    Path(institution_id): Path<Uuid>,
    headers: HeaderMap,
    _user: AuthUser,
) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let row = sqlx::query(
        "SELECT issuer, client_id, enabled, client_secret_ciphertext IS NOT NULL AS secret_configured
         FROM institution_oidc_providers WHERE institution_id = $1",
    )
    .bind(institution_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("oidc_provider_not_found"))?;
    Ok(Json(provider_view(
        row.try_get("issuer")?,
        row.try_get("client_id")?,
        row.try_get("enabled")?,
        row.try_get("secret_configured")?,
    )))
}

pub async fn configure_provider(
    State(state): State<Arc<AppState>>,
    Path(institution_id): Path<Uuid>,
    headers: HeaderMap,
    user: AuthUser,
    Json(req): Json<ConfigureOidcReq>,
) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers)?;
    let issuer = req.issuer.trim();
    let client_id = req.client_id.trim();
    validate_issuer(issuer)?;
    if client_id.is_empty() || client_id.len() > 255 || client_id.chars().any(char::is_control) {
        return Err(ApiError::unprocessable(
            "invalid_oidc_client_id",
            "client_id must be 1-255 characters without control characters",
        ));
    }
    if req.client_secret.as_ref().is_some_and(|secret| {
        secret.is_empty() || secret.len() > 4096 || secret.chars().any(char::is_control)
    }) {
        return Err(ApiError::unprocessable(
            "invalid_oidc_client_secret",
            "client_secret must be 1-4096 characters without control characters",
        ));
    }
    if req.client_secret.is_some() && req.clear_client_secret {
        return Err(ApiError::unprocessable(
            "conflicting_oidc_secret_change",
            "provide a new client_secret or clear_client_secret, not both",
        ));
    }
    let encrypted_secret = match req.client_secret.as_deref() {
        Some(secret) => Some(encrypt_secret(
            state.oidc_credential_key.as_deref().ok_or_else(|| {
                ApiError::unprocessable(
                    "oidc_encryption_unavailable",
                    "configure OIDC_CREDENTIAL_KEY before storing a client secret",
                )
            })?,
            institution_id,
            issuer,
            secret,
        )?),
        None => None,
    };

    let mut tx = state.pool.begin().await?;
    let institution = sqlx::query("SELECT 1 FROM institutions WHERE id = $1 FOR UPDATE")
        .bind(institution_id)
        .fetch_optional(&mut *tx)
        .await?;
    if institution.is_none() {
        return Err(ApiError::not_found("institution_not_found"));
    }
    let existing = sqlx::query(
        "SELECT issuer, client_id, client_secret_ciphertext
         FROM institution_oidc_providers WHERE institution_id = $1 FOR UPDATE",
    )
    .bind(institution_id)
    .fetch_optional(&mut *tx)
    .await?;
    let existing_secret = match existing {
        Some(row)
            if row.try_get::<String, _>("issuer")? == issuer
                && row.try_get::<String, _>("client_id")? == client_id =>
        {
            row.try_get::<Option<Vec<u8>>, _>("client_secret_ciphertext")?
        }
        _ => None,
    };
    let stored_secret = if req.clear_client_secret {
        None
    } else {
        encrypted_secret.or(existing_secret)
    };
    sqlx::query(
        "INSERT INTO institution_oidc_providers
            (institution_id, issuer, client_id, client_secret_ciphertext, enabled)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (institution_id) DO UPDATE SET
            issuer = EXCLUDED.issuer,
            client_id = EXCLUDED.client_id,
            client_secret_ciphertext = EXCLUDED.client_secret_ciphertext,
            enabled = EXCLUDED.enabled,
            updated_at = now()",
    )
    .bind(institution_id)
    .bind(issuer)
    .bind(client_id)
    .bind(stored_secret.as_deref())
    .bind(req.enabled)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM oidc_login_states WHERE institution_id = $1")
        .bind(institution_id)
        .execute(&mut *tx)
        .await?;
    crate::routes::admin::audit_scoped(
        &mut *tx,
        user.user_id,
        institution_id,
        "oidc_provider_configured",
        "oidc_provider",
        institution_id,
        json!({
            "issuer": issuer,
            "client_id": client_id,
            "enabled": req.enabled,
            "client_secret_configured": stored_secret.is_some()
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(provider_view(
        issuer.to_owned(),
        client_id.to_owned(),
        req.enabled,
        stored_secret.is_some(),
    )))
}

pub async fn start_login(
    State(state): State<Arc<AppState>>,
    Path(institution_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let row = sqlx::query(
        "SELECT issuer, client_id, client_secret_ciphertext
         FROM institution_oidc_providers
         WHERE institution_id = $1 AND enabled",
    )
    .bind(institution_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("sso_unavailable"))?;
    let issuer: String = row.try_get("issuer")?;
    let client_id: String = row.try_get("client_id")?;
    let encrypted_secret: Option<Vec<u8>> = row.try_get("client_secret_ciphertext")?;
    let client_secret = encrypted_secret
        .as_deref()
        .map(|secret| {
            let key = state
                .oidc_credential_key
                .as_deref()
                .ok_or_else(ApiError::internal)?;
            decrypt_secret(key, institution_id, &issuer, secret)
        })
        .transpose()?;

    let http_client = oidc_http_client()?;
    let metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new(issuer.clone()).map_err(|_| ApiError::internal())?,
        &http_client,
    )
    .await
    .map_err(|_| ApiError::internal())?;
    let provider_metadata = serde_json::to_value(&metadata).map_err(|_| ApiError::internal())?;
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(client_id.clone()),
        client_secret.map(ClientSecret::new),
    )
    .set_redirect_uri(
        RedirectUrl::new(callback_uri(&state, institution_id)?)
            .map_err(|_| ApiError::internal())?,
    );
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (authorization_url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            OidcNonce::new_random,
        )
        // The AuthorizationCode flow adds the `openid` scope itself; adding
        // it again produced a doubled scope in the authorize URL.
        .set_pkce_challenge(challenge)
        .url();
    let state_value = csrf.secret();
    sqlx::query("DELETE FROM oidc_login_states WHERE expires_at <= now()")
        .execute(&state.pool)
        .await?;
    sqlx::query("DELETE FROM oidc_login_tickets WHERE expires_at <= now()")
        .execute(&state.pool)
        .await?;
    sqlx::query(
        "INSERT INTO oidc_login_states
            (state_hash, institution_id, issuer, client_id, nonce, pkce_verifier, provider_metadata, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, now() + interval '5 minutes')",
    )
    .bind(sha256_hex(state_value))
    .bind(institution_id)
    .bind(&issuer)
    .bind(&client_id)
    .bind(nonce.secret())
    .bind(verifier.secret())
    .bind(provider_metadata)
    .execute(&state.pool)
    .await?;
    Ok(Json(
        json!({ "authorization_url": authorization_url.to_string() }),
    ))
}

async fn finish_callback(
    state: &AppState,
    institution_id: Uuid,
    query: OidcCallbackQuery,
) -> ApiResult<String> {
    let raw_state = query.state.as_deref().ok_or_else(ApiError::unauthorized)?;
    if raw_state.len() > 512 {
        return Err(ApiError::unauthorized());
    }
    let login = sqlx::query(
        "DELETE FROM oidc_login_states
         WHERE state_hash = $1 AND institution_id = $2 AND expires_at > now()
         RETURNING issuer, client_id, nonce, pkce_verifier, provider_metadata",
    )
    .bind(sha256_hex(raw_state))
    .bind(institution_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    if query.error.is_some() {
        return Err(ApiError::unauthorized());
    }
    let code = query
        .code
        .filter(|code| !code.is_empty() && code.len() <= 4096)
        .ok_or_else(ApiError::unauthorized)?;
    let issuer: String = login.try_get("issuer")?;
    let client_id: String = login.try_get("client_id")?;
    let nonce: String = login.try_get("nonce")?;
    let verifier: String = login.try_get("pkce_verifier")?;
    let provider_metadata: Value = login.try_get("provider_metadata")?;

    let configured = sqlx::query(
        "SELECT issuer, client_id, client_secret_ciphertext FROM institution_oidc_providers
         WHERE institution_id = $1 AND enabled",
    )
    .bind(institution_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    let configured_issuer: String = configured.try_get("issuer")?;
    let configured_client_id: String = configured.try_get("client_id")?;
    if issuer != configured_issuer || client_id != configured_client_id {
        return Err(ApiError::unauthorized());
    }
    let encrypted_secret: Option<Vec<u8>> = configured.try_get("client_secret_ciphertext")?;
    let client_secret = encrypted_secret
        .as_deref()
        .map(|secret| {
            let key = state
                .oidc_credential_key
                .as_deref()
                .ok_or_else(ApiError::internal)?;
            decrypt_secret(key, institution_id, &issuer, secret)
        })
        .transpose()?;
    let metadata: CoreProviderMetadata =
        serde_json::from_value(provider_metadata).map_err(|_| ApiError::internal())?;
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(client_id),
        client_secret.map(ClientSecret::new),
    )
    .set_redirect_uri(
        RedirectUrl::new(callback_uri(state, institution_id)?).map_err(|_| ApiError::internal())?,
    );
    let http_client = oidc_http_client()?;
    let tokens = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|_| ApiError::unauthorized())?
        .set_pkce_verifier(PkceCodeVerifier::new(verifier))
        .request_async(&http_client)
        .await
        .map_err(|_| ApiError::unauthorized())?;
    let id_token = tokens.id_token().ok_or_else(ApiError::unauthorized)?;
    let claims = id_token
        .claims(&client.id_token_verifier(), &OidcNonce::new(nonce))
        .map_err(|_| ApiError::unauthorized())?;
    let subject = claims.subject().as_str();
    if subject.is_empty() || subject.len() > 500 || subject.chars().any(char::is_control) {
        return Err(ApiError::unauthorized());
    }
    let user_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT u.id FROM external_identities e
         JOIN users u ON u.id = e.user_id
         WHERE e.institution_id = $1 AND e.provider = $2 AND e.subject = $3
           AND u.deleted_at IS NULL",
    )
    .bind(institution_id)
    .bind(&issuer)
    .bind(subject)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;

    let ticket = new_session_token().token;
    sqlx::query(
        "INSERT INTO oidc_login_tickets (token_hash, user_id, expires_at)
         VALUES ($1, $2, now() + interval '60 seconds')",
    )
    .bind(sha256_hex(&ticket))
    .bind(user_id)
    .execute(&state.pool)
    .await?;
    Ok(ticket)
}

pub async fn callback(
    State(state): State<Arc<AppState>>,
    Path(institution_id): Path<Uuid>,
    Query(query): Query<OidcCallbackQuery>,
) -> Redirect {
    match finish_callback(&state, institution_id, query).await {
        Ok(ticket) => frontend_callback_uri(&state, Some(&ticket))
            .map(|url| Redirect::to(&url))
            .unwrap_or_else(|_| Redirect::to("/login/sso/callback?error=sso_failed")),
        Err(_) => frontend_callback_uri(&state, None)
            .map(|url| Redirect::to(&url))
            .unwrap_or_else(|_| Redirect::to("/login/sso/callback?error=sso_failed")),
    }
}

pub async fn complete(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CompleteOidcReq>,
) -> ApiResult<Json<Value>> {
    if req.ticket.len() < 32 || req.ticket.len() > 128 {
        return Err(ApiError::unauthorized());
    }
    let user_id = sqlx::query_scalar::<_, Uuid>(
        "DELETE FROM oidc_login_tickets
         WHERE token_hash = $1 AND expires_at > now()
         RETURNING user_id",
    )
    .bind(sha256_hex(&req.ticket))
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    let token = issue_session(&state.pool, user_id).await?;
    Ok(Json(json!({ "token": token })))
}
