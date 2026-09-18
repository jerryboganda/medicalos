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
