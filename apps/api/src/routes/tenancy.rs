//! CORE-04: explicit personal/institution authorization contexts.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use domain_contracts::{PlatformRole, TenantRole};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

#[derive(Deserialize)]
pub struct CreateTenantRequest {
    name: String,
    initial_administrator_user_id: Uuid,
}

#[derive(Serialize)]
pub struct CreateTenantResponse {
    tenant_id: Uuid,
    membership_id: Uuid,
    initial_administrator_user_id: Uuid,
    audit_event_id: Uuid,
}

#[derive(Deserialize)]
pub struct MembershipRequest {
    user_id: Uuid,
    role: TenantRole,
}

#[derive(Serialize)]
pub struct MembershipResponse {
    tenant_id: Uuid,
    membership_id: Uuid,
    user_id: Uuid,
    role: TenantRole,
    aggregate_version: i32,
    changed: bool,
    audit_event_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct AuditEventResponse {
    id: Uuid,
    actor_user_id: Uuid,
    tenant_id: Uuid,
    subject_user_id: Option<Uuid>,
    aggregate_type: String,
    aggregate_id: Uuid,
    aggregate_version: i32,
    occurred_at: DateTime<Utc>,
    received_at: DateTime<Utc>,
    device: Option<String>,
    correlation_id: Uuid,
    privacy_scope: String,
    action: String,
    payload: Value,
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

fn device(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

async fn require_platform_owner(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    let role = PlatformRole::PlatformOwner.as_str();
    let authorized = sqlx::query!(
        "SELECT user_id FROM platform_roles WHERE user_id = $1 AND role = $2",
        user_id,
        role
    )
    .fetch_optional(&state.pool)
    .await?
    .is_some();
    if !authorized {
        return Err(ApiError::forbidden(
            "platform_owner_required",
            "platform owner role is required",
        ));
    }
    Ok(())
}

async fn require_tenant_admin(state: &AppState, user_id: Uuid, tenant_id: Uuid) -> ApiResult<()> {
    let role = TenantRole::InstitutionAdministrator.as_str();
    let authorized = sqlx::query!(
        "SELECT m.id
         FROM tenant_memberships m
         JOIN tenant_membership_roles r ON r.membership_id = m.id
         WHERE m.user_id = $1 AND m.tenant_id = $2 AND r.role = $3",
        user_id,
        tenant_id,
        role
    )
    .fetch_optional(&state.pool)
    .await?
    .is_some();
    if !authorized {
        return Err(ApiError::forbidden(
            "tenant_admin_required",
            "institution administrator role is required in this tenant",
        ));
    }
    Ok(())
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

pub async fn create_tenant(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Json(req): Json<CreateTenantRequest>,
) -> ApiResult<(StatusCode, Json<CreateTenantResponse>)> {
    require_platform_owner(&state, user.user_id).await?;
    let name = req.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > 160 {
        return Err(ApiError::unprocessable(
            "invalid_tenant_name",
            "tenant name must be between 1 and 160 characters",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let administrator_exists = sqlx::query!(
        "SELECT id FROM users WHERE id = $1",
        req.initial_administrator_user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .is_some();
    if !administrator_exists {
        return Err(ApiError::not_found("user_not_found"));
    }

    let tenant_id = Uuid::new_v4();
    let membership_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO tenants (id, name) VALUES ($1, $2)",
        tenant_id,
        name
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO tenant_memberships (id, tenant_id, user_id) VALUES ($1, $2, $3)",
        membership_id,
        tenant_id,
        req.initial_administrator_user_id
    )
    .execute(&mut *tx)
    .await?;
    let administrator_role = TenantRole::InstitutionAdministrator.as_str();
    sqlx::query!(
        "INSERT INTO tenant_membership_roles (membership_id, role) VALUES ($1, $2)",
        membership_id,
        administrator_role
    )
    .execute(&mut *tx)
    .await?;

    let audit_event_id = Uuid::new_v4();
    let correlation_id = Uuid::new_v4();
    let occurred_at = Utc::now();
    let device = device(&headers);
    let payload = serde_json::json!({
        "name": name,
        "initial_role": TenantRole::InstitutionAdministrator,
    });
    sqlx::query!(
        "INSERT INTO audit_events
             (id, actor_user_id, tenant_id, subject_user_id, aggregate_type, aggregate_id,
              aggregate_version, occurred_at, device, correlation_id, privacy_scope, action, payload)
         VALUES ($1, $2, $3, $4, 'tenant', $3, 1, $5, $6, $7, 'tenant_admin', 'tenant.created', $8)",
        audit_event_id,
        user.user_id,
        tenant_id,
        req.initial_administrator_user_id,
        occurred_at,
        device,
        correlation_id,
        payload
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateTenantResponse {
            tenant_id,
            membership_id,
            initial_administrator_user_id: req.initial_administrator_user_id,
            audit_event_id,
        }),
    ))
}

pub async fn add_membership(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Json(req): Json<MembershipRequest>,
) -> ApiResult<Json<MembershipResponse>> {
    let tenant_id = tenant_id(&headers)?;
    require_tenant_admin(&state, user.user_id, tenant_id).await?;

    let mut tx = state.pool.begin().await?;
    let user_exists = sqlx::query!("SELECT id FROM users WHERE id = $1", req.user_id)
        .fetch_optional(&mut *tx)
        .await?
        .is_some();
    if !user_exists {
        return Err(ApiError::not_found("user_not_found"));
    }

    let membership_id = if let Some(row) = sqlx::query!(
        "INSERT INTO tenant_memberships (id, tenant_id, user_id)
         VALUES ($1, $2, $3)
         ON CONFLICT (tenant_id, user_id) DO NOTHING
         RETURNING id",
        Uuid::new_v4(),
        tenant_id,
        req.user_id
    )
    .fetch_optional(&mut *tx)
    .await?
    {
        row.id
    } else {
        sqlx::query!(
            "SELECT id FROM tenant_memberships WHERE tenant_id = $1 AND user_id = $2",
            tenant_id,
            req.user_id
        )
        .fetch_one(&mut *tx)
        .await?
        .id
    };

    let role = req.role.as_str();
    let changed = sqlx::query!(
        "INSERT INTO tenant_membership_roles (membership_id, role)
         VALUES ($1, $2)
         ON CONFLICT (membership_id, role) DO NOTHING
         RETURNING role",
        membership_id,
        role
    )
    .fetch_optional(&mut *tx)
    .await?
    .is_some();

    let aggregate_version = sqlx::query!(
        "SELECT COUNT(*)::INT AS \"count!\" FROM tenant_membership_roles WHERE membership_id = $1",
        membership_id
    )
    .fetch_one(&mut *tx)
    .await?
    .count;

    let audit_event_id = if changed {
        let audit_event_id = Uuid::new_v4();
        let correlation_id = Uuid::new_v4();
        let occurred_at = Utc::now();
        let device = device(&headers);
        let payload = serde_json::json!({ "role": req.role });
        sqlx::query!(
            "INSERT INTO audit_events
                 (id, actor_user_id, tenant_id, subject_user_id, aggregate_type, aggregate_id,
                  aggregate_version, occurred_at, device, correlation_id, privacy_scope, action, payload)
             VALUES ($1, $2, $3, $4, 'tenant_membership', $5, $6, $7, $8, $9,
                     'tenant_admin', 'tenant_membership.role_added', $10)",
            audit_event_id,
            user.user_id,
            tenant_id,
            req.user_id,
            membership_id,
            aggregate_version,
            occurred_at,
            device,
            correlation_id,
            payload
        )
        .execute(&mut *tx)
        .await?;
        Some(audit_event_id)
    } else {
        None
    };

    tx.commit().await?;

    Ok(Json(MembershipResponse {
        tenant_id,
        membership_id,
        user_id: req.user_id,
        role: req.role,
        aggregate_version,
        changed,
        audit_event_id,
    }))
}

pub async fn get_audit_event(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
) -> ApiResult<Json<AuditEventResponse>> {
    let tenant_id = tenant_id(&headers)?;
    require_tenant_admin(&state, user.user_id, tenant_id).await?;

    let event = sqlx::query!(
        "SELECT id, actor_user_id, tenant_id, subject_user_id, aggregate_type, aggregate_id,
                aggregate_version, occurred_at, received_at, device, correlation_id,
                privacy_scope, action, payload
         FROM audit_events
         WHERE id = $1 AND tenant_id = $2",
        event_id,
        tenant_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::not_found("audit_event_not_found"))?;

    Ok(Json(AuditEventResponse {
        id: event.id,
        actor_user_id: event.actor_user_id,
        tenant_id: event.tenant_id,
        subject_user_id: event.subject_user_id,
        aggregate_type: event.aggregate_type,
        aggregate_id: event.aggregate_id,
        aggregate_version: event.aggregate_version,
        occurred_at: event.occurred_at,
        received_at: event.received_at,
        device: event.device,
        correlation_id: event.correlation_id,
        privacy_scope: event.privacy_scope,
        action: event.action,
        payload: event.payload,
    }))
}
