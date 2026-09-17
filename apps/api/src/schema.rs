//! Schema application. One forward migration + one down file, applied by the
//! same SQL everywhere (CI psql bootstrap, tests, startup) — no separate
//! migration runner state to keep consistent. §31.1: the rollback path is
//! exercised by a test (up -> down -> up).

use sqlx::PgPool;

pub const UP_SQL: &str = include_str!("../migrations/0001_init.up.sql");
pub const DOWN_SQL: &str = include_str!("../migrations/0001_init.down.sql");

pub async fn apply_up(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(UP_SQL).execute(pool).await.map(|_| ())
}

pub async fn apply_down(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(DOWN_SQL).execute(pool).await.map(|_| ())
}
