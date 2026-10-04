#![allow(dead_code)]

pub mod accounts;
pub mod address_registry;
pub mod currencies;
pub mod discovery_queue;
pub mod holdings;
pub mod keychain;
pub mod realized_pnl;
pub mod reconciliation;
pub mod schema;
pub mod sync_state;
pub mod tax_lots;
pub mod transactions;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use crate::config::AppConfig;
use crate::error::Result;

pub use accounts::AccountRepository;
pub use address_registry::AddressRegistry;
pub use discovery_queue::DiscoveryQueue;
pub use holdings::HoldingRepository;
pub use keychain::KeychainKeyRepository;
pub use realized_pnl::RealizedPnLRepository;
pub use reconciliation::ReconciliationLogRepository;
pub use sync_state::SyncStateRepository;
pub use tax_lots::TaxLotRepository;
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

    schema::create(&pool).await?;

    Ok(pool)
}

/// Initialize an in-memory database (for testing)
pub async fn init_memory_pool() -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    schema::create(&pool).await?;

    Ok(pool)
}
