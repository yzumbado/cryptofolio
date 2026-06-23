use sqlx::SqlitePool;

use crate::error::Result;

pub struct DiscoveryQueue<'a> {
    pool: &'a SqlitePool,
}

impl<'a> DiscoveryQueue<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Enqueue an address for later classification/sync.
    /// UNIQUE(address, chain) — duplicate silently ignored.
    pub async fn enqueue(
        &self,
        address: &str,
        chain: &str,
        discovered_from_tx: Option<i64>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO discovery_queue (address, chain, discovered_from_tx)
            VALUES (?, ?, ?)
            ON CONFLICT(address, chain) DO NOTHING
            "#,
        )
        .bind(address)
        .bind(chain)
        .bind(discovered_from_tx)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn pending_count(&self) -> Result<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM discovery_queue WHERE status = 'pending'")
                .fetch_one(self.pool)
                .await?;
        Ok(count)
    }
}
