use cryptofolio::db::schema;
use cryptofolio::error::Result;
use sqlx::SqlitePool;

pub async fn setup_test_db() -> Result<SqlitePool> {
    let pool = SqlitePool::connect(":memory:").await?;
    schema::create(&pool).await?;
    Ok(pool)
}
