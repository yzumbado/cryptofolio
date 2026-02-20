use cryptofolio::db::migrations;
use cryptofolio::error::Result;
use sqlx::SqlitePool;

/// Sets up a test database with migrations applied
pub async fn setup_test_db() -> Result<SqlitePool> {
    let pool = SqlitePool::connect(":memory:").await?;
    migrations::run(&pool).await?;
    Ok(pool)
}
