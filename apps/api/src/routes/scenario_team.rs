//! SIM-08: invite-only teams, role-attributed actions, and SBAR handovers.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{sha256_hex, AuthUser};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub(crate) async fn role_on(
    connection: &mut PgConnection,
    run_id: Uuid,
    user_id: Uuid,
) -> ApiResult<Option<String>> {
    let role = sqlx::query_scalar::<_, Option<String>>(
        r#"SELECT COALESCE(member.role, CASE WHEN run.user_id = $2 THEN 'team_lead' END)
		   FROM scenario_runs run
		   LEFT JOIN scenario_team_members member
		     ON member.run_id = run.id AND member.user_id = $2
		   WHERE run.id = $1"#,
    )
    .bind(run_id)
    .bind(user_id)
    .fetch_optional(&mut *connection)
    .await?;
    Ok(role.flatten())
}

async fn role_for(pool: &PgPool, run_id: Uuid, user_id: Uuid) -> ApiResult<Option<String>> {
    let mut connection = pool.acquire().await?;
    role_on(&mut connection, run_id, user_id).await
}

#[derive(sqlx::FromRow)]
struct TeamRun {
    owner_id: Uuid,
    finished_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct TeamMemberRow {
    member_id: Uuid,
    role: String,
    joined_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioTeamRole.ts",
        rename = "ScenarioTeamRole"
    )
)]
pub enum ScenarioTeamRole {
    TeamLead,
    HistoryTaker,
    Scribe,
    Observer,
}

impl ScenarioTeamRole {
    fn parse(role: &str) -> Option<Self> {
        match role {
            "team_lead" => Some(Self::TeamLead),
            "history_taker" => Some(Self::HistoryTaker),
            "scribe" => Some(Self::Scribe),
            "observer" => Some(Self::Observer),
            _ => None,
        }
    }

    fn is_invitable(&self) -> bool {
        matches!(self, Self::HistoryTaker | Self::Scribe | Self::Observer)
    }
}

fn parse_team_role(role: &str) -> ApiResult<ScenarioTeamRole> {
    ScenarioTeamRole::parse(role).ok_or_else(ApiError::internal)
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioTeamMember.ts",
        rename = "ScenarioTeamMember"
    )
)]
pub struct ScenarioTeamMember {
    member_id: Uuid,
    role: ScenarioTeamRole,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    joined_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "scenario/ScenarioTeam.ts", rename = "ScenarioTeam")
)]
pub struct ScenarioTeam {
    run_id: Uuid,
    current_role: ScenarioTeamRole,
    current_member_id: Uuid,
    members: Vec<ScenarioTeamMember>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioTeamInviteCreatedResponse.ts",
        rename = "ScenarioTeamInviteCreatedResponse"
    )
)]
pub struct ScenarioTeamInviteCreatedResponse {
    invite_code: String,
    role: ScenarioTeamRole,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioTeamJoinResponse.ts",
        rename = "ScenarioTeamJoinResponse"
    )
)]
pub struct ScenarioTeamJoinResponse {
    run_id: Uuid,
    member_id: Uuid,
    role: ScenarioTeamRole,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioHandover.ts",
        rename = "ScenarioHandover"
    )
)]
pub struct ScenarioHandover {
    handover_id: Uuid,
    from_role: ScenarioTeamRole,
    to_role: ScenarioTeamRole,
    situation: String,
    background: String,
    assessment: String,
    recommendation: String,
    #[cfg_attr(feature = "type-export", ts(type = "string"))]
    created_at: DateTime<Utc>,
    acknowledged: bool,
    #[cfg_attr(feature = "type-export", ts(type = "string | null"))]
    acknowledged_at: Option<DateTime<Utc>>,
    can_ack: bool,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioHandoversResponse.ts",
        rename = "ScenarioHandoversResponse"
    )
)]
pub struct ScenarioHandoversResponse {
    handovers: Vec<ScenarioHandover>,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioHandoverCreatedResponse.ts",
        rename = "ScenarioHandoverCreatedResponse"
    )
)]
pub struct ScenarioHandoverCreatedResponse {
    handover_id: Uuid,
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "scenario/ScenarioHandoverAcknowledgedResponse.ts",
        rename = "ScenarioHandoverAcknowledgedResponse"
    )
)]
pub struct ScenarioHandoverAcknowledgedResponse {
    handover_id: Uuid,
    #[cfg_attr(feature = "type-export", ts(type = "true"))]
    acknowledged: bool,
}

pub async fn list_team(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
) -> ApiResult<Json<ScenarioTeam>> {
    let current_role = role_for(&state.pool, run_id, user.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    let current_member_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM scenario_team_members WHERE run_id = $1 AND user_id = $2",
    )
    .bind(run_id)
    .bind(user.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::internal)?;
    let members = sqlx::query_as::<_, TeamMemberRow>(
        "SELECT id AS member_id, role, joined_at FROM scenario_team_members
		 WHERE run_id = $1 ORDER BY joined_at, id",
    )
    .bind(run_id)
    .fetch_all(&state.pool)
    .await?;
    let members = members
        .into_iter()
        .map(|member| {
            Ok(ScenarioTeamMember {
                member_id: member.member_id,
                role: parse_team_role(&member.role)?,
                joined_at: member.joined_at,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(ScenarioTeam {
        run_id,
        current_role: parse_team_role(&current_role)?,
        current_member_id,
        members,
    }))
}

#[derive(Deserialize)]
pub struct CreateTeamInviteReq {
    pub role: String,
}

pub async fn create_invite(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
    Json(req): Json<CreateTeamInviteReq>,
) -> ApiResult<(StatusCode, Json<ScenarioTeamInviteCreatedResponse>)> {
    let response_role = ScenarioTeamRole::parse(&req.role)
        .filter(ScenarioTeamRole::is_invitable)
        .ok_or_else(|| {
            ApiError::unprocessable(
                "invalid_scenario_team_role",
                "role must be history_taker, scribe, or observer",
            )
        })?;
    let mut tx = state.pool.begin().await?;
    let run = sqlx::query_as::<_, TeamRun>(
        "SELECT user_id AS owner_id, finished_at FROM scenario_runs WHERE id = $1 FOR UPDATE",
    )
    .bind(run_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    if run.owner_id != user.user_id {
        return Err(ApiError::forbidden(
            "team_lead_required",
            "only the team lead can invite members",
        ));
    }
    if run.finished_at.is_some() {
        return Err(ApiError::conflict(
            "scenario_run_finished",
            "team invitations close when the station is finished",
        ));
    }
    let members = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM scenario_team_members WHERE run_id = $1",
    )
    .bind(run_id)
    .fetch_one(&mut *tx)
    .await?;
    let pending = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM scenario_team_invites
		 WHERE run_id = $1 AND accepted_at IS NULL AND expires_at > now()",
    )
    .bind(run_id)
    .fetch_one(&mut *tx)
    .await?;
    if members + pending >= 6 {
        return Err(ApiError::conflict(
            "scenario_team_full",
            "a simulation team can contain at most six members including pending invitations",
        ));
    }
    let invite_id = Uuid::new_v4();
    let code = Uuid::new_v4().simple().to_string();
    let token_hash = sha256_hex(&code);
    let expires_at = Utc::now() + Duration::hours(24);
    sqlx::query(
        "INSERT INTO scenario_team_invites (id, run_id, created_by, role, token_sha256, expires_at)
		 VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(invite_id)
    .bind(run_id)
    .bind(user.user_id)
    .bind(&req.role)
    .bind(token_hash)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "scenario_team_invite_created",
        "scenario_team_invite",
        invite_id,
        json!({"role": req.role}),
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(ScenarioTeamInviteCreatedResponse {
            invite_code: code,
            role: response_role,
            expires_at,
        }),
    ))
}

#[derive(Deserialize)]
pub struct JoinTeamInviteReq {
    pub invite_code: String,
}

#[derive(sqlx::FromRow)]
struct InviteRow {
    id: Uuid,
    run_id: Uuid,
    role: String,
    expires_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
}

pub async fn join_invite(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(req): Json<JoinTeamInviteReq>,
) -> ApiResult<(StatusCode, Json<ScenarioTeamJoinResponse>)> {
    let code = Uuid::parse_str(req.invite_code.trim())
        .map_err(|_| ApiError::not_found("scenario_team_invite_not_found"))?
        .simple()
        .to_string();
    let token_hash = sha256_hex(&code);
    let mut tx = state.pool.begin().await?;
    let invite = sqlx::query_as::<_, InviteRow>(
        "SELECT id, run_id, role, expires_at, accepted_at FROM scenario_team_invites
		 WHERE token_sha256 = $1 FOR UPDATE",
    )
    .bind(token_hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("scenario_team_invite_not_found"))?;
    let response_role = parse_team_role(&invite.role)?;
    if invite.accepted_at.is_some() {
        return Err(ApiError::conflict(
            "scenario_team_invite_used",
            "this team invitation has already been used",
        ));
    }
    if invite.expires_at <= Utc::now() {
        return Err(ApiError::conflict(
            "scenario_team_invite_expired",
            "this team invitation has expired",
        ));
    }
    let run = sqlx::query_as::<_, TeamRun>(
        "SELECT user_id AS owner_id, finished_at FROM scenario_runs WHERE id = $1 FOR UPDATE",
    )
    .bind(invite.run_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    if run.finished_at.is_some() {
        return Err(ApiError::conflict(
            "scenario_run_finished",
            "team invitations close when the station is finished",
        ));
    }
    if run.owner_id == user.user_id {
        return Err(ApiError::conflict(
            "already_scenario_team_member",
            "the team lead is already part of the team",
        ));
    }
    let already_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM scenario_team_members WHERE run_id = $1 AND user_id = $2)",
    )
    .bind(invite.run_id)
    .bind(user.user_id)
    .fetch_one(&mut *tx)
    .await?;
    if already_member {
        return Err(ApiError::conflict(
            "already_scenario_team_member",
            "this account has already joined the team",
        ));
    }
    let members = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM scenario_team_members WHERE run_id = $1",
    )
    .bind(invite.run_id)
    .fetch_one(&mut *tx)
    .await?;
    if members >= 6 {
        return Err(ApiError::conflict(
            "scenario_team_full",
            "the simulation team is full",
        ));
    }
    let member_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO scenario_team_members (id, run_id, user_id, role, invited_by)
		 VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(member_id)
    .bind(invite.run_id)
    .bind(user.user_id)
    .bind(&invite.role)
    .bind(run.owner_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE scenario_team_invites SET accepted_by = $2, accepted_at = now() WHERE id = $1",
    )
    .bind(invite.id)
    .bind(user.user_id)
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "scenario_team_invite_accepted",
        "scenario_team_member",
        member_id,
        json!({"run_id": invite.run_id, "role": invite.role}),
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(ScenarioTeamJoinResponse {
            run_id: invite.run_id,
            member_id,
            role: response_role,
        }),
    ))
}

fn valid_sbar_text(value: &str) -> bool {
    let length = value.trim().chars().count();
    (1..=2000).contains(&length)
        && !value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
}

#[derive(Deserialize)]
pub struct CreateHandoverReq {
    pub recipient_member_id: Uuid,
    pub situation: String,
    pub background: String,
    pub assessment: String,
    pub recommendation: String,
}

pub async fn create_handover(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
    Json(req): Json<CreateHandoverReq>,
) -> ApiResult<(StatusCode, Json<ScenarioHandoverCreatedResponse>)> {
    if ![
        &req.situation,
        &req.background,
        &req.assessment,
        &req.recommendation,
    ]
    .into_iter()
    .all(|value| valid_sbar_text(value))
    {
        return Err(ApiError::unprocessable(
            "invalid_scenario_handover",
            "each handover section must contain 1-2000 printable characters",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let run = sqlx::query_as::<_, TeamRun>(
        "SELECT user_id AS owner_id, finished_at FROM scenario_runs WHERE id = $1 FOR UPDATE",
    )
    .bind(run_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    let role = role_on(&mut tx, run_id, user.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    if run.finished_at.is_some() {
        return Err(ApiError::conflict(
            "scenario_run_finished",
            "handover records can be added only while the station is active",
        ));
    }
    let from_member_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM scenario_team_members WHERE run_id = $1 AND user_id = $2",
    )
    .bind(run_id)
    .bind(user.user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::internal)?;
    if req.recipient_member_id == from_member_id {
        return Err(ApiError::unprocessable(
            "invalid_handover_recipient",
            "a handover must go to a different team member",
        ));
    }
    let recipient_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM scenario_team_members WHERE run_id = $1 AND id = $2)",
    )
    .bind(run_id)
    .bind(req.recipient_member_id)
    .fetch_one(&mut *tx)
    .await?;
    if !recipient_exists {
        return Err(ApiError::not_found("scenario_team_member_not_found"));
    }
    let handover_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO scenario_handovers
		 (id, run_id, from_member_id, to_member_id, situation, background, assessment, recommendation)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(handover_id)
    .bind(run_id)
    .bind(from_member_id)
    .bind(req.recipient_member_id)
    .bind(req.situation.trim())
    .bind(req.background.trim())
    .bind(req.assessment.trim())
    .bind(req.recommendation.trim())
    .execute(&mut *tx)
    .await?;
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "scenario_handover_created",
        "scenario_handover",
        handover_id,
        json!({"from_role": role}),
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(ScenarioHandoverCreatedResponse { handover_id }),
    ))
}

#[derive(sqlx::FromRow)]
struct HandoverRow {
    handover_id: Uuid,
    from_role: String,
    to_role: String,
    situation: String,
    background: String,
    assessment: String,
    recommendation: String,
    created_at: DateTime<Utc>,
    acknowledged_at: Option<DateTime<Utc>>,
    can_ack: bool,
}

pub async fn list_handovers(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(run_id): Path<Uuid>,
) -> ApiResult<Json<ScenarioHandoversResponse>> {
    role_for(&state.pool, run_id, user.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("run_not_found"))?;
    let rows = sqlx::query_as::<_, HandoverRow>(
        r#"SELECT handover.id AS handover_id,
		          sender.role AS from_role, recipient.role AS to_role,
		          handover.situation, handover.background, handover.assessment,
		          handover.recommendation, handover.created_at,
		          acknowledgement.acknowledged_at,
		          (recipient.user_id = $2) AS can_ack
		   FROM scenario_handovers handover
		   JOIN scenario_team_members sender ON sender.id = handover.from_member_id
		   JOIN scenario_team_members recipient ON recipient.id = handover.to_member_id
		   LEFT JOIN scenario_handover_acknowledgements acknowledgement
		     ON acknowledgement.handover_id = handover.id
		   WHERE handover.run_id = $1
		   ORDER BY handover.created_at, handover.id
		   LIMIT 100"#,
    )
    .bind(run_id)
    .bind(user.user_id)
    .fetch_all(&state.pool)
    .await?;
    let handovers = rows
        .into_iter()
        .map(|row| {
            Ok(ScenarioHandover {
                handover_id: row.handover_id,
                from_role: parse_team_role(&row.from_role)?,
                to_role: parse_team_role(&row.to_role)?,
                situation: row.situation,
                background: row.background,
                assessment: row.assessment,
                recommendation: row.recommendation,
                created_at: row.created_at,
                acknowledged: row.acknowledged_at.is_some(),
                acknowledged_at: row.acknowledged_at,
                can_ack: row.can_ack,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(ScenarioHandoversResponse { handovers }))
}

#[derive(sqlx::FromRow)]
struct HandoverRecipient {
    user_id: Uuid,
}

pub async fn acknowledge_handover(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path((run_id, handover_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<ScenarioHandoverAcknowledgedResponse>> {
    let mut tx = state.pool.begin().await?;
    let recipient = sqlx::query_as::<_, HandoverRecipient>(
        r#"SELECT member.user_id FROM scenario_handovers handover
		   JOIN scenario_team_members member ON member.id = handover.to_member_id
		   WHERE handover.id = $1 AND handover.run_id = $2"#,
    )
    .bind(handover_id)
    .bind(run_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("scenario_handover_not_found"))?;
    if recipient.user_id != user.user_id {
        return Err(ApiError::forbidden(
            "handover_recipient_required",
            "only the designated handover recipient can acknowledge it",
        ));
    }
    let acknowledgement = sqlx::query(
        "INSERT INTO scenario_handover_acknowledgements (handover_id, acknowledged_by)
		 VALUES ($1, $2) ON CONFLICT (handover_id) DO NOTHING",
    )
    .bind(handover_id)
    .bind(user.user_id)
    .execute(&mut *tx)
    .await?;
    if acknowledgement.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "handover_already_acknowledged",
            "this handover has already been acknowledged",
        ));
    }
    crate::routes::admin::audit(
        &mut *tx,
        user.user_id,
        "scenario_handover_acknowledged",
        "scenario_handover",
        handover_id,
        json!({"acknowledged": true}),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(ScenarioHandoverAcknowledgedResponse {
        handover_id,
        acknowledged: true,
    }))
}
