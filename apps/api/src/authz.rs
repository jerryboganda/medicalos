//! RBAC (§18.1, §19.3, §25): the one place that maps roles to permissions.
//!
//! Platform roles are Zitadel project-role grants, snapshotted onto the
//! session at sign-in (`auth_sessions.roles`). Institution roles live in
//! `institution_members.role`. Relationship rules — community moderators,
//! scenario teams, author ≠ approver, resource ownership — stay next to their
//! data; this module only answers "may this person do this kind of thing".

use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};

/// Only permissions some shipped code path checks. Add one when a handler
/// needs it — never speculatively.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(export, export_to = "auth/Permission.ts")
)]
pub enum Permission {
    // Platform-wide.
    ContentAuthor,
    ClinicalApprove,
    ContentPublish,
    ExamConfigure,
    ExamAssess,
    ReportTriage,
    PlatformOps,
    OwnerDashboard,
    // Scoped to one institution.
    InstitutionAdmin,
    ProgramManage,
    InstitutionTeach,
}

use Permission::*;

impl Permission {
    /// Privileged permissions need a multi-factor sign-in (§6.3, §25).
    /// Teaching reads a cohort's work but changes no one's access.
    pub fn needs_mfa(self) -> bool {
        self != InstitutionTeach
    }
}

/// Zitadel project roles (provisioned by infra/zitadel/provision.sh).
/// Authoring, clinical approval and publishing stay separate (§18.1);
/// publishing is the owner's alone until a publisher role is decided.
pub fn platform_grants(role: &str) -> &'static [Permission] {
    match role {
        "platform_owner" => &[
            ContentAuthor,
            ClinicalApprove,
            ContentPublish,
            ExamConfigure,
            ExamAssess,
            ReportTriage,
            PlatformOps,
            OwnerDashboard,
        ],
        "author" => &[ContentAuthor],
        "medical_reviewer" => &[ClinicalApprove],
        "examiner" => &[ExamAssess],
        "support" => &[ReportTriage],
        // billing_admin: no billing code exists yet (COM-02).
        _ => &[],
    }
}

/// `institution_members.role` values (migration 0061 CHECK). Institution
/// authors/reviewers/examiners hold no institution permission yet.
pub fn institution_grants(role: &str) -> &'static [Permission] {
    match role {
        "admin" => &[InstitutionAdmin, ProgramManage, InstitutionTeach],
        "program_lead" => &[ProgramManage, InstitutionTeach],
        "instructor" => &[InstitutionTeach],
        _ => &[],
    }
}

fn collect(roles: &[String], grants: fn(&str) -> &'static [Permission]) -> Vec<Permission> {
    let mut out: Vec<Permission> = Vec::new();
    for permission in roles.iter().flat_map(|role| grants(role)) {
        if !out.contains(permission) {
            out.push(*permission);
        }
    }
    out
}

fn check(granted: &[Permission], mfa: bool, permission: Permission) -> ApiResult<()> {
    if !granted.contains(&permission) {
        return Err(ApiError::forbidden(
            "permission_required",
            "your account does not have permission for this action",
        ));
    }
    if permission.needs_mfa() && !mfa {
        return Err(ApiError::forbidden(
            "mfa_required",
            "sign in with a second factor to use this permission",
        ));
    }
    Ok(())
}

impl AuthUser {
    /// Platform permissions this session holds (before the MFA check).
    pub fn permissions(&self) -> Vec<Permission> {
        collect(&self.roles, platform_grants)
    }

    /// Gate a platform-wide action.
    pub fn require(&self, permission: Permission) -> ApiResult<()> {
        check(&self.permissions(), self.mfa, permission)
    }
}

/// Gate an action inside one institution by the caller's membership role.
pub async fn require_in(
    pool: &sqlx::PgPool,
    user: &AuthUser,
    institution_id: Uuid,
    permission: Permission,
) -> ApiResult<()> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM institution_members WHERE institution_id = $1 AND user_id = $2",
    )
    .bind(institution_id)
    .bind(user.user_id)
    .fetch_optional(pool)
    .await?;
    let roles: Vec<String> = role.into_iter().collect();
    check(&collect(&roles, institution_grants), user.mfa, permission)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(roles: &[&str], mfa: bool) -> AuthUser {
        AuthUser {
            user_id: Uuid::nil(),
            roles: roles.iter().map(|r| r.to_string()).collect(),
            mfa,
        }
    }

    fn code(result: ApiResult<()>) -> String {
        match result {
            Ok(()) => "ok".into(),
            Err(err) => err.code.to_string(),
        }
    }

    #[test]
    fn roles_grant_only_their_permissions_and_privilege_needs_mfa() {
        assert_eq!(code(user(&["author"], true).require(ContentAuthor)), "ok");
        assert_eq!(
            code(user(&["author"], false).require(ContentAuthor)),
            "mfa_required"
        );
        assert_eq!(
            code(user(&["author"], true).require(ClinicalApprove)),
            "permission_required"
        );
        assert_eq!(
            code(user(&["author"], true).require(ContentPublish)),
            "permission_required"
        );
        assert_eq!(
            code(user(&[], true).require(OwnerDashboard)),
            "permission_required"
        );
        assert_eq!(
            code(user(&["unknown_role"], true).require(PlatformOps)),
            "permission_required"
        );
        assert_eq!(
            code(user(&["platform_owner"], true).require(OwnerDashboard)),
            "ok"
        );
        assert_eq!(
            user(&["author", "medical_reviewer", "author"], true).permissions(),
            vec![ContentAuthor, ClinicalApprove]
        );
    }

    #[test]
    fn institution_roles_map_and_teaching_skips_mfa() {
        let teach = collect(&["instructor".into()], institution_grants);
        assert_eq!(code(check(&teach, false, InstitutionTeach)), "ok");
        assert_eq!(
            code(check(&teach, false, InstitutionAdmin)),
            "permission_required"
        );
        let admin = collect(&["admin".into()], institution_grants);
        assert_eq!(code(check(&admin, false, InstitutionAdmin)), "mfa_required");
        assert_eq!(code(check(&admin, true, ProgramManage)), "ok");
        assert!(collect(&["learner".into()], institution_grants).is_empty());
    }
}
