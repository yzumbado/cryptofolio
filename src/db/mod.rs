pub mod accounts;
pub mod holdings;
pub mod migrations;
pub mod transactions;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;

use crate::config::AppConfig;
use crate::error::Result;

pub use accounts::AccountRepository;
pub use holdings::HoldingRepository;
pub use transactions::TransactionRepository;

/// Initialize the database connection pool
pub async fn init_pool() -> Result<SqlitePool> {
    let db_path = AppConfig::database_path()?;

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Run migrations
    migrations::run(&pool).await?;

    Ok(pool)
}

/// Initialize an in-memory database (for testing)
pub async fn init_memory_pool() -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    migrations::run(&pool).await?;

    Ok(pool)
}
