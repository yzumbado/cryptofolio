#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::error::Result;

/// Tracks the last successful sync point for each Binance history endpoint.
/// One row per account_id. Used for incremental syncs so we only fetch new data.
#[derive(Debug, Clone)]
pub struct SyncState {
    pub account_id: String,
    /// Timestamp of the most recent trade successfully synced
    pub last_trade_sync: Option<DateTime<Utc>>,
    /// Timestamp of the most recent deposit successfully synced
    pub last_deposit_sync: Option<DateTime<Utc>>,
    /// Timestamp of the most recent withdrawal successfully synced
    pub last_withdrawal_sync: Option<DateTime<Utc>>,
    /// Timestamp of the most recent fiat order successfully synced
    pub last_fiat_sync: Option<DateTime<Utc>>,
    /// Timestamp of the most recent internal transfer successfully synced
    pub last_transfer_sync: Option<DateTime<Utc>>,
    /// Last Binance trade ID processed (used for fromId pagination per symbol)
    pub last_trade_id: Option<i64>,
    /// Last symbol processed (used to resume mid-symbol-list)
    pub last_sync_symbol: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SyncState {
    /// Returns true if this account has never been synced for trades.
    pub fn needs_full_trade_sync(&self) -> bool {
        self.last_trade_sync.is_none()
    }

    /// Returns true if this account has never been synced for deposits.
    pub fn needs_full_deposit_sync(&self) -> bool {
        self.last_deposit_sync.is_none()
    }

    /// Returns true if this account has never been synced for withdrawals.
    pub fn needs_full_withdrawal_sync(&self) -> bool {
        self.last_withdrawal_sync.is_none()
    }
}

pub struct SyncStateRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SyncStateRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Get the sync state for an account, or create a blank one if it doesn't exist yet.
    pub async fn get_or_create(&self, account_id: &str) -> Result<SyncState> {
        // Try to fetch existing
        if let Some(state) = self.get(account_id).await? {
            return Ok(state);
        }

        // Insert a blank record
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO binance_sync_state (account_id)
            VALUES (?)
            "#,
        )
        .bind(account_id)
        .execute(self.pool)
        .await?;

        // Fetch the newly-created row
        self.get(account_id).await?.ok_or_else(|| {
            crate::error::CryptofolioError::Other(format!(
                "Failed to create sync state for account '{}'",
                account_id
            ))
        })
    }

    /// Fetch sync state for an account (returns None if no row exists).
    pub async fn get(&self, account_id: &str) -> Result<Option<SyncState>> {
        let row = sqlx::query_as::<
            _,
            (
                String,         // account_id
                Option<String>, // last_trade_sync
                Option<String>, // last_deposit_sync
                Option<String>, // last_withdrawal_sync
                Option<String>, // last_fiat_sync
                Option<String>, // last_transfer_sync
                Option<i64>,    // last_trade_id
                Option<String>, // last_sync_symbol
                String,         // created_at
                String,         // updated_at
            ),
        >(
            r#"
            SELECT account_id,
                   last_trade_sync,
                   last_deposit_sync,
                   last_withdrawal_sync,
                   last_fiat_sync,
                   last_transfer_sync,
                   last_trade_id,
                   last_sync_symbol,
                   created_at,
                   updated_at
            FROM binance_sync_state
            WHERE account_id = ?
            "#,
        )
        .bind(account_id)
        .fetch_optional(self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.parse_row(r)?)),
            None => Ok(None),
        }
    }

    /// Mark trades as synced up to `timestamp`. Optionally record the last trade ID.
    pub async fn update_trade_sync(
        &self,
        account_id: &str,
        timestamp: DateTime<Utc>,
        last_trade_id: Option<i64>,
        last_symbol: Option<&str>,
    ) -> Result<()> {
        let ts = timestamp.to_rfc3339();
        sqlx::query(
            r#"
            UPDATE binance_sync_state
            SET last_trade_sync    = ?,
                last_trade_id      = COALESCE(?, last_trade_id),
                last_sync_symbol   = COALESCE(?, last_sync_symbol),
                updated_at         = CURRENT_TIMESTAMP
            WHERE account_id = ?
            "#,
        )
        .bind(&ts)
        .bind(last_trade_id)
        .bind(last_symbol)
        .bind(account_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Mark deposits as synced up to `timestamp`.
    pub async fn update_deposit_sync(
        &self,
        account_id: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let ts = timestamp.to_rfc3339();
        sqlx::query(
            r#"
            UPDATE binance_sync_state
            SET last_deposit_sync = ?,
                updated_at        = CURRENT_TIMESTAMP
            WHERE account_id = ?
            "#,
        )
        .bind(&ts)
        .bind(account_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Mark withdrawals as synced up to `timestamp`.
    pub async fn update_withdrawal_sync(
        &self,
        account_id: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let ts = timestamp.to_rfc3339();
        sqlx::query(
            r#"
            UPDATE binance_sync_state
            SET last_withdrawal_sync = ?,
                updated_at           = CURRENT_TIMESTAMP
            WHERE account_id = ?
            "#,
        )
        .bind(&ts)
        .bind(account_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Mark fiat orders as synced up to `timestamp`.
    pub async fn update_fiat_sync(&self, account_id: &str, timestamp: DateTime<Utc>) -> Result<()> {
        let ts = timestamp.to_rfc3339();
        sqlx::query(
            r#"
            UPDATE binance_sync_state
            SET last_fiat_sync = ?,
                updated_at     = CURRENT_TIMESTAMP
            WHERE account_id = ?
            "#,
        )
        .bind(&ts)
        .bind(account_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Mark internal transfers as synced up to `timestamp`.
    pub async fn update_transfer_sync(
        &self,
        account_id: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let ts = timestamp.to_rfc3339();
        sqlx::query(
            r#"
            UPDATE binance_sync_state
            SET last_transfer_sync = ?,
                updated_at         = CURRENT_TIMESTAMP
            WHERE account_id = ?
            "#,
        )
        .bind(&ts)
        .bind(account_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Reset all sync state for an account (forces a full re-sync on next run).
    pub async fn reset(&self, account_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE binance_sync_state
            SET last_trade_sync      = NULL,
                last_deposit_sync    = NULL,
                last_withdrawal_sync = NULL,
                last_fiat_sync       = NULL,
                last_transfer_sync   = NULL,
                last_trade_id        = NULL,
                last_sync_symbol     = NULL,
                updated_at           = CURRENT_TIMESTAMP
            WHERE account_id = ?
            "#,
        )
        .bind(account_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    fn parse_row(
        &self,
        (
            account_id,
            last_trade_sync,
            last_deposit_sync,
            last_withdrawal_sync,
            last_fiat_sync,
            last_transfer_sync,
            last_trade_id,
            last_sync_symbol,
            created_at,
            updated_at,
        ): (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<String>,
            String,
            String,
        ),
    ) -> Result<SyncState> {
        Ok(SyncState {
            account_id,
            last_trade_sync: parse_optional_datetime(last_trade_sync),
            last_deposit_sync: parse_optional_datetime(last_deposit_sync),
            last_withdrawal_sync: parse_optional_datetime(last_withdrawal_sync),
            last_fiat_sync: parse_optional_datetime(last_fiat_sync),
            last_transfer_sync: parse_optional_datetime(last_transfer_sync),
            last_trade_id,
            last_sync_symbol,
            created_at: parse_datetime(&created_at),
            updated_at: parse_datetime(&updated_at),
        })
    }
}

fn parse_optional_datetime(s: Option<String>) -> Option<DateTime<Utc>> {
    s.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    })
}

fn parse_datetime(s: &str) -> DateTime<Utc> {
    // SQLite stores CURRENT_TIMESTAMP as "YYYY-MM-DD HH:MM:SS"
    // Try RFC3339 first (written by Rust), then SQLite default format
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").map(|ndt| ndt.and_utc())
        })
        .unwrap_or_else(|_| Utc::now())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> Result<SqlitePool> {
        let pool = SqlitePool::connect(":memory:").await?;
        migrations::run(&pool).await?;

        // Create test category and account
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test-cat', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) \
             VALUES ('acc-1', 'test-cat', 'Binance', 'exchange')",
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    #[tokio::test]
    async fn test_get_or_create_new_account() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = SyncStateRepository::new(&pool);

        let state = repo.get_or_create("acc-1").await?;
        assert_eq!(state.account_id, "acc-1");
        assert!(state.last_trade_sync.is_none());
        assert!(state.last_deposit_sync.is_none());
        assert!(state.last_withdrawal_sync.is_none());
        assert!(state.last_trade_id.is_none());
        assert!(state.needs_full_trade_sync());
        assert!(state.needs_full_deposit_sync());
        assert!(state.needs_full_withdrawal_sync());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_or_create_idempotent() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = SyncStateRepository::new(&pool);

        // Call twice — should not error or create duplicates
        let _s1 = repo.get_or_create("acc-1").await?;
        let s2 = repo.get_or_create("acc-1").await?;
        assert_eq!(s2.account_id, "acc-1");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_trade_sync() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = SyncStateRepository::new(&pool);
        repo.get_or_create("acc-1").await?;

        let ts = Utc::now();
        repo.update_trade_sync("acc-1", ts, Some(99999), Some("BTCUSDT"))
            .await?;

        let state = repo.get("acc-1").await?.unwrap();
        assert!(state.last_trade_sync.is_some());
        assert_eq!(state.last_trade_id, Some(99999));
        assert_eq!(state.last_sync_symbol, Some("BTCUSDT".to_string()));
        assert!(!state.needs_full_trade_sync());

        Ok(())
    }

    #[tokio::test]
    async fn test_update_deposit_sync() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = SyncStateRepository::new(&pool);
        repo.get_or_create("acc-1").await?;

        let ts = Utc::now();
        repo.update_deposit_sync("acc-1", ts).await?;

        let state = repo.get("acc-1").await?.unwrap();
        assert!(state.last_deposit_sync.is_some());
        assert!(!state.needs_full_deposit_sync());

        Ok(())
    }

    #[tokio::test]
    async fn test_update_withdrawal_sync() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = SyncStateRepository::new(&pool);
        repo.get_or_create("acc-1").await?;

        let ts = Utc::now();
        repo.update_withdrawal_sync("acc-1", ts).await?;

        let state = repo.get("acc-1").await?.unwrap();
        assert!(state.last_withdrawal_sync.is_some());
        assert!(!state.needs_full_withdrawal_sync());

        Ok(())
    }

    #[tokio::test]
    async fn test_reset_clears_all_sync_state() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = SyncStateRepository::new(&pool);
        repo.get_or_create("acc-1").await?;

        let ts = Utc::now();
        repo.update_trade_sync("acc-1", ts, Some(42), Some("ETHUSDT"))
            .await?;
        repo.update_deposit_sync("acc-1", ts).await?;
        repo.update_withdrawal_sync("acc-1", ts).await?;

        // Verify state was written
        let state = repo.get("acc-1").await?.unwrap();
        assert!(state.last_trade_sync.is_some());

        // Reset
        repo.reset("acc-1").await?;

        let state = repo.get("acc-1").await?.unwrap();
        assert!(state.last_trade_sync.is_none());
        assert!(state.last_deposit_sync.is_none());
        assert!(state.last_withdrawal_sync.is_none());
        assert!(state.last_trade_id.is_none());
        assert!(state.last_sync_symbol.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_returns_none_for_unknown_account() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = SyncStateRepository::new(&pool);

        let state = repo.get("nonexistent").await?;
        assert!(state.is_none());

        Ok(())
    }
}
