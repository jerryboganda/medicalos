//! CORE-04: explicit personal/institution authorization contexts.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use domain_contracts::{PlatformRole, TenantRole};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Serialize)]
pub struct PersonalContext {
    kind: &'static str,
    user_id: Uuid,
}

#[derive(Serialize)]
pub struct TenantContext {
    tenant_id: Uuid,
    name: String,
    status: String,
    roles: Vec<TenantRole>,
}

#[derive(Serialize)]
pub struct ContextsResponse {
    personal: PersonalContext,
    tenants: Vec<TenantContext>,
    platform_roles: Vec<PlatformRole>,
}

#[derive(Serialize)]
pub struct ActiveTenantContext {
    user_id: Uuid,
    tenant_id: Uuid,
    name: String,
    status: String,
    roles: Vec<TenantRole>,
}

fn parse_tenant_role(value: &str) -> ApiResult<TenantRole> {
    TenantRole::from_str(value).map_err(|_| ApiError::internal())
}

fn parse_platform_role(value: &str) -> ApiResult<PlatformRole> {
    PlatformRole::from_str(value).map_err(|_| ApiError::internal())
}

fn tenant_id(headers: &HeaderMap) -> ApiResult<Uuid> {
    let raw = headers
        .get("X-Tenant-Id")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            ApiError::unprocessable("tenant_scope_required", "X-Tenant-Id is required")
        })?;
    Uuid::parse_str(raw)
        .map_err(|_| ApiError::unprocessable("tenant_scope_invalid", "X-Tenant-Id must be a UUID"))
}

async fn roles_for_membership(state: &AppState, membership_id: Uuid) -> ApiResult<Vec<TenantRole>> {
    let rows = sqlx::query!(
        "SELECT role FROM tenant_membership_roles WHERE membership_id = $1 ORDER BY role",
        membership_id
    )
    .fetch_all(&state.pool)
    .await?;
    rows.into_iter()
        .map(|row| parse_tenant_role(&row.role))
        .collect()
}

pub async fn my_contexts(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<ContextsResponse>> {
    let memberships = sqlx::query!(
        "SELECT m.id AS membership_id, t.id AS tenant_id, t.name, t.status
         FROM tenant_memberships m
         JOIN tenants t ON t.id = m.tenant_id
         WHERE m.user_id = $1
         ORDER BY t.name, t.id",
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?;

    let mut tenants = Vec::with_capacity(memberships.len());
    for membership in memberships {
        tenants.push(TenantContext {
            tenant_id: membership.tenant_id,
            name: membership.name,
            status: membership.status,
            roles: roles_for_membership(&state, membership.membership_id).await?,
        });
    }

    let platform_roles = sqlx::query!(
        "SELECT role FROM platform_roles WHERE user_id = $1 ORDER BY role",
        user.user_id
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|row| parse_platform_role(&row.role))
    .collect::<ApiResult<Vec<_>>>()?;

    Ok(Json(ContextsResponse {
        personal: PersonalContext {
            kind: "personal",
            user_id: user.user_id,
        },
        tenants,
        platform_roles,
    }))
}

pub async fn tenant_context(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<ActiveTenantContext>> {
    let tenant_id = tenant_id(&headers)?;
    let membership = sqlx::query!(
        "SELECT m.id AS membership_id, t.name, t.status
         FROM tenant_memberships m
         JOIN tenants t ON t.id = m.tenant_id
         WHERE m.user_id = $1 AND m.tenant_id = $2",
        user.user_id,
        tenant_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        ApiError::forbidden(
            "tenant_access_denied",
            "authenticated user is not a member of this tenant",
        )
    })?;

    Ok(Json(ActiveTenantContext {
        user_id: user.user_id,
        tenant_id,
        name: membership.name,
        status: membership.status,
        roles: roles_for_membership(&state, membership.membership_id).await?,
    }))
}
