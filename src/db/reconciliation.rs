//! Repository for the append-only `reconciliation_log` table.
//!
//! The log records each reconciliation reading so drift can be tracked over
//! time. Writing a row here is non-destructive: it never touches the immutable
//! `transactions` table or `holdings`.

use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::error::Result;

pub struct ReconciliationLogRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ReconciliationLogRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Record one reconciliation reading. `status` must satisfy the schema
    /// CHECK (`verified` | `partial` | `unreconciled`).
    pub async fn insert(
        &self,
        account_id: &str,
        asset: &str,
        onchain_balance: &Decimal,
        computed_balance: &Decimal,
        delta: &Decimal,
        status: &str,
        block_height: Option<i64>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO reconciliation_log
             (account_id, asset, onchain_balance, computed_balance, delta, status, block_height)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(account_id)
        .bind(asset)
        .bind(onchain_balance.to_string())
        .bind(computed_balance.to_string())
        .bind(delta.to_string())
        .bind(status)
        .bind(block_height)
        .execute(self.pool)
        .await?;

        Ok(())
    }
}
