//! Platform sign-in through Zitadel (.scratch/auth-zitadel/spec.md).
//!
//! The API is Zitadel's only OIDC client, so the SPA and the Tauri webviews
//! never hold IdP tokens. A sign-in ends exactly like institution SSO
//! (oidc.rs): a one-use ticket at /login/sso/callback, exchanged for the same
//! opaque session — which here also carries the platform-role snapshot and
//! whether a second factor was used (authz.rs).

use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{new_session_token, sha256_hex, AuthUser};
use crate::authz::{institution_grants, Permission};
use crate::error::{ApiError, ApiResult};
use crate::routes::oidc::{
    api_url, frontend_callback_uri, oidc_http_client, OidcCallbackQuery,
    StartInstitutionSsoResponse,
};
use crate::state::{AppState, ZitadelConfig};

/// Zitadel's role claim: `{ "<role>": { "<org id>": "<org domain>" } }`.
const ROLES_CLAIM: &str = "urn:zitadel:iam:org:project:roles";
const CALLBACK_PATH: &str = "/v1/auth/oidc/callback";

#[derive(Deserialize)]
pub struct StartQuery {
    /// `google` or `apple` skips Zitadel's chooser; absent shows the hosted
    /// email/password page (which also offers every configured IdP).
    pub idp: Option<String>,
}

fn config(state: &AppState) -> ApiResult<&ZitadelConfig> {
    state
        .zitadel
        .as_ref()
        .ok_or_else(|| ApiError::not_found("sign_in_unavailable"))
}

pub async fn start(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StartQuery>,
) -> ApiResult<Json<StartInstitutionSsoResponse>> {
    let cfg = config(&state)?;
    let unavailable = || ApiError::not_found("idp_unavailable");
    let idp_hint = match query.idp.as_deref() {
        None | Some("") => None,
        Some("google") => Some(cfg.google_idp_id.clone().ok_or_else(unavailable)?),
        Some("apple") => Some(cfg.apple_idp_id.clone().ok_or_else(unavailable)?),
        Some(_) => {
            return Err(ApiError::bad_request(
                "unknown_idp",
                "idp must be google or apple",
            ))
        }
    };

    let http_client = oidc_http_client()?;
    let metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new(cfg.issuer.clone()).map_err(|_| ApiError::internal())?,
        &http_client,
    )
    .await
    .map_err(|_| ApiError::internal())?;
    let stored_metadata = serde_json::to_value(&metadata).map_err(|_| ApiError::internal())?;
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(cfg.client_id.clone()),
        Some(ClientSecret::new(cfg.client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(api_url(&state, CALLBACK_PATH)?).map_err(|_| ApiError::internal())?,
    );
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let mut request = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()))
        .add_scope(Scope::new("urn:zitadel:iam:org:projects:roles".into()))
        .set_pkce_challenge(challenge);
    if let Some(project) = &cfg.project_id {
        request = request.add_scope(Scope::new(format!(
            "urn:zitadel:iam:org:project:id:{project}:aud"
        )));
    }
    if let Some(idp) = idp_hint {
        request = request.add_scope(Scope::new(format!("urn:zitadel:iam:org:idp:id:{idp}")));
    }
    let (authorization_url, csrf, nonce) = request.url();

    sqlx::query("DELETE FROM platform_login_states WHERE expires_at <= now()")
        .execute(&state.pool)
        .await?;
    sqlx::query(
        "INSERT INTO platform_login_states
            (state_hash, nonce, pkce_verifier, provider_metadata, expires_at)
         VALUES ($1, $2, $3, $4, now() + interval '5 minutes')",
    )
    .bind(sha256_hex(csrf.secret()))
    .bind(nonce.secret())
    .bind(verifier.secret())
    .bind(stored_metadata)
    .execute(&state.pool)
    .await?;
    Ok(Json(StartInstitutionSsoResponse {
        authorization_url: authorization_url.to_string(),
    }))
}

pub async fn callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OidcCallbackQuery>,
) -> Redirect {
    let ticket = match finish(&state, query).await {
        Ok(ticket) => Some(ticket),
        Err(err) => {
            tracing::warn!(code = err.code, "platform sign-in failed");
            None
        }
    };
    frontend_callback_uri(&state, ticket.as_deref())
        .map(|url| Redirect::to(&url))
        .unwrap_or_else(|_| Redirect::to("/login/sso/callback?error=sso_failed"))
}

async fn finish(state: &AppState, query: OidcCallbackQuery) -> ApiResult<String> {
    let cfg = config(state)?;
    let raw_state = query
        .state
        .as_deref()
        .filter(|value| value.len() <= 512)
        .ok_or_else(ApiError::unauthorized)?;
    let login = sqlx::query(
        "DELETE FROM platform_login_states
         WHERE state_hash = $1 AND expires_at > now()
         RETURNING nonce, pkce_verifier, provider_metadata",
    )
    .bind(sha256_hex(raw_state))
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
    let nonce: String = login.try_get("nonce")?;
    let verifier: String = login.try_get("pkce_verifier")?;
    let metadata: CoreProviderMetadata =
        serde_json::from_value(login.try_get::<Value, _>("provider_metadata")?)
            .map_err(|_| ApiError::internal())?;

    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(cfg.client_id.clone()),
        Some(ClientSecret::new(cfg.client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(api_url(state, CALLBACK_PATH)?).map_err(|_| ApiError::internal())?,
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
        .claims(&client.id_token_verifier(), &Nonce::new(nonce))
        .map_err(|_| ApiError::unauthorized())?;
    let subject = claims.subject().as_str().to_owned();
    if subject.is_empty() || subject.len() > 500 || subject.chars().any(char::is_control) {
        return Err(ApiError::unauthorized());
    }

    // Signature, issuer, audience, expiry and nonce are verified above; read
    // Zitadel's custom claims from the same (now trusted) payload.
    let payload = verified_payload(id_token)?;
    let roles = roles_from(&payload);
    let mfa = payload["amr"]
        .as_array()
        .is_some_and(|methods| methods.iter().any(|method| method == "mfa"));
    let verified_email = payload["email"]
        .as_str()
        .filter(|_| payload["email_verified"].as_bool() == Some(true));
    let user_id = resolve_user(&state.pool, &subject, verified_email).await?;

    let ticket = new_session_token().token;
    sqlx::query(
        "INSERT INTO oidc_login_tickets (token_hash, user_id, expires_at, roles, mfa)
         VALUES ($1, $2, now() + interval '60 seconds', $3, $4)",
    )
    .bind(sha256_hex(&ticket))
    .bind(user_id)
    .bind(&roles)
    .bind(mfa)
    .execute(&state.pool)
    .await?;
    Ok(ticket)
}

/// Decode the claims of an ID token whose signature is already verified.
fn verified_payload(id_token: &impl Serialize) -> ApiResult<Value> {
    let compact = serde_json::to_value(id_token)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(ApiError::internal)?;
    let body = compact
        .split('.')
        .nth(1)
        .ok_or_else(ApiError::unauthorized)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(body.trim_end_matches('='))
        .map_err(|_| ApiError::unauthorized())?;
    serde_json::from_slice(&bytes).map_err(|_| ApiError::unauthorized())
}

fn roles_from(payload: &Value) -> Vec<String> {
    let mut roles: Vec<String> = payload
        .get(ROLES_CLAIM)
        .and_then(Value::as_object)
        .map(|granted| {
            granted
                .keys()
                .filter(|role| role.len() <= 64)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    roles.sort();
    roles
}

/// Find the account for a Zitadel subject, creating it on first sign-in.
/// Never links by email: an existing password account joins Zitadel through
/// the importer (same UUID), not because an address happens to match.
async fn resolve_user(
    pool: &sqlx::PgPool,
    subject: &str,
    verified_email: Option<&str>,
) -> ApiResult<Uuid> {
    let linked = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE idp_subject = $1 AND deleted_at IS NULL",
    )
    .bind(subject)
    .fetch_optional(pool)
    .await?;
    if let Some(user_id) = linked {
        return Ok(user_id);
    }
    // Imported accounts keep their UUID as their Zitadel user id.
    if let Ok(imported) = Uuid::parse_str(subject) {
        let claimed = sqlx::query_scalar::<_, Uuid>(
            "UPDATE users SET idp_subject = $1
             WHERE id = $2 AND idp_subject IS NULL AND deleted_at IS NULL
             RETURNING id",
        )
        .bind(subject)
        .bind(imported)
        .fetch_optional(pool)
        .await?;
        if let Some(user_id) = claimed {
            return Ok(user_id);
        }
    }
    let email = verified_email
        .map(|email| email.trim().to_lowercase())
        .filter(|email| email.contains('@') && email.len() <= 254)
        .ok_or_else(|| {
            ApiError::forbidden("email_unverified", "verify your email address to sign in")
        })?;
    let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
        .bind(&email)
        .fetch_one(pool)
        .await?;
    if taken {
        return Err(ApiError::conflict(
            "account_pending_migration",
            "an account with this email exists and has not been moved to the new sign-in yet",
        ));
    }
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, idp_subject) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&email)
        .bind(subject)
        .execute(pool)
        .await?;
    Ok(user_id)
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "auth/MeView.ts")
)]
pub struct MeView {
    pub user_id: Uuid,
    pub email: String,
    /// This session signed in with a second factor.
    pub mfa: bool,
    /// Platform roles (Zitadel project grants) on this session.
    pub roles: Vec<String>,
    /// Platform permissions those roles grant (see authz.rs).
    pub permissions: Vec<Permission>,
    pub institutions: Vec<MeInstitution>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "auth/MeInstitution.ts")
)]
pub struct MeInstitution {
    pub institution_id: Uuid,
    pub name: String,
    pub role: String,
    pub permissions: Vec<Permission>,
}

/// Who am I and what may I do — drives which workspaces the UI offers. The
/// API still enforces every permission server-side.
pub async fn me(State(state): State<Arc<AppState>>, user: AuthUser) -> ApiResult<Json<MeView>> {
    let email: String =
        sqlx::query_scalar("SELECT email FROM users WHERE id = $1 AND deleted_at IS NULL")
            .bind(user.user_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(ApiError::unauthorized)?;
    let institutions = sqlx::query(
        "SELECT i.id, i.name, m.role FROM institution_members m
         JOIN institutions i ON i.id = m.institution_id
         WHERE m.user_id = $1 ORDER BY i.name, i.id",
    )
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|row| {
        let role: String = row.try_get("role")?;
        Ok(MeInstitution {
            institution_id: row.try_get("id")?,
            name: row.try_get("name")?,
            permissions: institution_grants(&role).to_vec(),
            role,
        })
    })
    .collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(MeView {
        user_id: user.user_id,
        email,
        mfa: user.mfa,
        permissions: user.permissions(),
        roles: user.roles,
        institutions,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zitadel_role_claim_becomes_a_sorted_role_list() {
        let mut claims = serde_json::Map::new();
        claims.insert(
            ROLES_CLAIM.into(),
            serde_json::json!({
                "platform_owner": { "281": "medical-os.localhost" },
                "author": { "281": "medical-os.localhost" }
            }),
        );
        let payload = Value::Object(claims);
        assert_eq!(roles_from(&payload), vec!["author", "platform_owner"]);
        assert!(roles_from(&serde_json::json!({})).is_empty());
    }
}
