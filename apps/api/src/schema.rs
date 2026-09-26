//! Schema application. Numbered forward migrations + paired down files,
//! applied by the same SQL everywhere (CI psql bootstrap, tests, startup) —
//! no separate migration runner state to keep consistent. §31.1: the rollback
//! path is exercised by a test (up -> down -> up).

use sqlx::PgPool;

macro_rules! migrations {
    ($($name:literal),* $(,)?) => {
        /// Forward migrations in declaration order.
        pub const UP_SQLS: &[&str] = &[$(include_str!(concat!("../migrations/", $name, ".up.sql"))),*];
        /// Same files; apply_down iterates this in REVERSE.
        pub const DOWN_SQLS: &[&str] = &[$(include_str!(concat!("../migrations/", $name, ".down.sql"))),*];
    };
}

migrations!(
    "0001_init",
    "0002_session_timing",
    "0003_entitlements",
    "0004_recall",
    "0005_item_reports",
    "0006_mocks",
    "0007_editorial",
    "0008_coach",
    "0009_phase1",
    "0010_program",
    "0011_appeals",
    "0012_fix_ce_hours",
    "0013_program2",
    "0014_program3",
    "0015_integrity",
    "0016_phase2_completion",
    "0017_offline",
    "0018_completion_kernel",
    "0019_engagement",
    "0020_assessment_form_immutability",
    "0021_tenant_foundations",
    "0022_reserved_families",
    "0023_community",
    "0024_library_cards",
    "0025_ops_admin",
    "0026_session_tails",
    "0027_recovery_qa",
    "0028_inst04_program_curriculum",
    "0029_inst03_oidc_sso",
    "0030_qb16_report_sla",
    "0031_qb16_public_correction_note",
    "0032_qb13_session_tools",
    "0033_off02_submit_receipt",
    "0034_core10_concepts",
    "0035_ai08_plan_protection",
    "0036_ai08_task_identity",
    "0037_lib05_source_change_propagation",
    "0038_ai08_shared_task_identity",
    "0039_lib06_private_imports",
    "0040_sim04_criterion_evidence",
    "0041_admin02_rights_scope_terms",
    "0042_lib07_extraction_reports",
    "0043_sim07_assessment_appeals",
    "0044_sim08_team_handover",
    "0045_img02_reviewed_annotations",
    "0046_sim03_transcript_corrections",
    "0047_img02_structured_findings",
    "0048_admin06_import_metadata",
    "0049_admin06_review_event_kind",
    "0050_comp02_scoring_leaderboard",
    "0051_community_post_reports",
    "0052_competition_series_leagues",
    "0053_eng01_shared_qotd",
);

pub async fn apply_up(pool: &PgPool) -> Result<(), sqlx::Error> {
    for sql in UP_SQLS {
        sqlx::raw_sql(sql).execute(pool).await?;
    }
    Ok(())
}

pub async fn apply_down(pool: &PgPool) -> Result<(), sqlx::Error> {
    for sql in DOWN_SQLS.iter().rev() {
        sqlx::raw_sql(sql).execute(pool).await?;
    }
    Ok(())
}
