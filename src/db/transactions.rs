use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::core::transaction::{Transaction, TransactionType};
use crate::error::{CryptofolioError, Result};

pub struct TransactionRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> TransactionRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, limit: Option<i64>) -> Result<Vec<Transaction>> {
        let limit = limit.unwrap_or(50);

        let rows = sqlx::query_as::<_, TransactionRow>(
            r#"
            SELECT id, tx_type, from_account_id, from_asset, from_quantity,
                   to_account_id, to_asset, to_quantity, price_usd, fee, fee_asset,
                   external_id, notes, timestamp, created_at
            FROM transactions
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        rows.into_iter()
            .map(|r| self.parse_transaction(r))
            .collect()
    }

    pub async fn list_by_account(
        &self,
        account_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<Transaction>> {
        let limit = limit.unwrap_or(50);

        let rows = sqlx::query_as::<_, TransactionRow>(
            r#"
            SELECT id, tx_type, from_account_id, from_asset, from_quantity,
                   to_account_id, to_asset, to_quantity, price_usd, fee, fee_asset,
                   external_id, notes, timestamp, created_at
            FROM transactions
            WHERE from_account_id = ? OR to_account_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(account_id)
        .bind(account_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        rows.into_iter()
            .map(|r| self.parse_transaction(r))
            .collect()
    }

    pub async fn insert(&self, tx: &Transaction) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO transactions (
                tx_type, from_account_id, from_asset, from_quantity,
                to_account_id, to_asset, to_quantity, price_usd, fee, fee_asset,
                external_id, notes, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(tx.tx_type.as_str())
        .bind(&tx.from_account_id)
        .bind(&tx.from_asset)
        .bind(tx.from_quantity.map(|d| d.to_string()))
        .bind(&tx.to_account_id)
        .bind(&tx.to_asset)
        .bind(tx.to_quantity.map(|d| d.to_string()))
        .bind(tx.price_usd.map(|d| d.to_string()))
        .bind(tx.fee.map(|d| d.to_string()))
        .bind(&tx.fee_asset)
        .bind(&tx.external_id)
        .bind(&tx.notes)
        .bind(tx.timestamp.to_rfc3339())
        .execute(self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    fn parse_transaction(&self, row: TransactionRow) -> Result<Transaction> {
        let tx_type = TransactionType::from_str(&row.tx_type).ok_or_else(|| {
            CryptofolioError::Other(format!("Invalid transaction type: {}", row.tx_type))
        })?;

        let parse_decimal = |s: Option<String>| -> Result<Option<Decimal>> {
            s.map(|v| Decimal::from_str(&v).map_err(|_| CryptofolioError::InvalidAmount(v)))
                .transpose()
        };

        Ok(Transaction {
            id: row.id,
            tx_type,
            from_account_id: row.from_account_id,
            from_asset: row.from_asset,
            from_quantity: parse_decimal(row.from_quantity)?,
            to_account_id: row.to_account_id,
            to_asset: row.to_asset,
            to_quantity: parse_decimal(row.to_quantity)?,
            price_usd: parse_decimal(row.price_usd)?,
            price_currency: None,     // TODO: Load from database
            price_amount: None,       // TODO: Load from database
            exchange_rate: None,      // TODO: Load from database
            exchange_rate_pair: None, // TODO: Load from database
            fee: parse_decimal(row.fee)?,
            fee_asset: row.fee_asset,
            external_id: row.external_id,
            notes: row.notes,
            timestamp: DateTime::parse_from_rfc3339(&row.timestamp)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

#[derive(sqlx::FromRow)]
struct TransactionRow {
    id: i64,
    tx_type: String,
    from_account_id: Option<String>,
    from_asset: Option<String>,
    from_quantity: Option<String>,
    to_account_id: Option<String>,
    to_asset: Option<String>,
    to_quantity: Option<String>,
    price_usd: Option<String>,
    fee: Option<String>,
    fee_asset: Option<String>,
    external_id: Option<String>,
    notes: Option<String>,
    timestamp: String,
    created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use chrono::TimeZone;

    async fn setup_test_db() -> Result<SqlitePool> {
        let pool = SqlitePool::connect(":memory:").await?;
        migrations::run(&pool).await?;

        // Create test category and account
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test-cat', 'Test Category')")
            .execute(&pool)
            .await?;

        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) VALUES ('test-acc-1', 'test-cat', 'Test Account 1', 'exchange')",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) VALUES ('test-acc-2', 'test-cat', 'Test Account 2', 'wallet')",
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    #[tokio::test]
    async fn test_insert_buy_transaction() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TransactionRepository::new(&pool);

        let tx = Transaction {
            id: 0,
            tx_type: TransactionType::Buy,
            from_account_id: None,
            from_asset: None,
            from_quantity: None,
            to_account_id: Some("test-acc-1".to_string()),
            to_asset: Some("BTC".to_string()),
            to_quantity: Some(Decimal::from_str("1.5").unwrap()),
            price_usd: Some(Decimal::from_str("45000").unwrap()),
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: Some(Decimal::from_str("10").unwrap()),
            fee_asset: Some("USD".to_string()),
            external_id: Some("EXT123".to_string()),
            notes: Some("Test buy".to_string()),
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        let tx_id = repo.insert(&tx).await?;
        assert!(tx_id > 0);

        // Verify it was inserted
        let transactions = repo.list(Some(10)).await?;
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].tx_type, TransactionType::Buy);
        assert_eq!(transactions[0].to_asset, Some("BTC".to_string()));
        assert_eq!(
            transactions[0].to_quantity,
            Some(Decimal::from_str("1.5").unwrap())
        );
        assert_eq!(transactions[0].external_id, Some("EXT123".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_sell_transaction() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TransactionRepository::new(&pool);

        let tx = Transaction {
            id: 0,
            tx_type: TransactionType::Sell,
            from_account_id: Some("test-acc-1".to_string()),
            from_asset: Some("ETH".to_string()),
            from_quantity: Some(Decimal::from_str("2.0").unwrap()),
            to_account_id: None,
            to_asset: None,
            to_quantity: None,
            price_usd: Some(Decimal::from_str("3000").unwrap()),
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: None,
            fee_asset: None,
            external_id: None,
            notes: None,
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        let tx_id = repo.insert(&tx).await?;
        assert!(tx_id > 0);

        let transactions = repo.list(Some(10)).await?;
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].tx_type, TransactionType::Sell);
        assert_eq!(transactions[0].from_asset, Some("ETH".to_string()));
        assert_eq!(
            transactions[0].from_quantity,
            Some(Decimal::from_str("2.0").unwrap())
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_transfer_transaction() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TransactionRepository::new(&pool);

        let tx = Transaction {
            id: 0,
            tx_type: TransactionType::TransferInternal,
            from_account_id: Some("test-acc-1".to_string()),
            from_asset: Some("BTC".to_string()),
            from_quantity: Some(Decimal::from_str("0.5").unwrap()),
            to_account_id: Some("test-acc-2".to_string()),
            to_asset: Some("BTC".to_string()),
            to_quantity: Some(Decimal::from_str("0.5").unwrap()),
            price_usd: None,
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: Some(Decimal::from_str("0.0001").unwrap()),
            fee_asset: Some("BTC".to_string()),
            external_id: None,
            notes: Some("Cold storage transfer".to_string()),
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        let tx_id = repo.insert(&tx).await?;
        assert!(tx_id > 0);

        let transactions = repo.list(Some(10)).await?;
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].tx_type, TransactionType::TransferInternal);
        assert_eq!(
            transactions[0].from_account_id,
            Some("test-acc-1".to_string())
        );
        assert_eq!(
            transactions[0].to_account_id,
            Some("test-acc-2".to_string())
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_swap_transaction() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TransactionRepository::new(&pool);

        let tx = Transaction {
            id: 0,
            tx_type: TransactionType::Swap,
            from_account_id: Some("test-acc-1".to_string()),
            from_asset: Some("BTC".to_string()),
            from_quantity: Some(Decimal::from_str("1.0").unwrap()),
            to_account_id: Some("test-acc-1".to_string()),
            to_asset: Some("ETH".to_string()),
            to_quantity: Some(Decimal::from_str("15.0").unwrap()),
            price_usd: Some(Decimal::from_str("3000").unwrap()),
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: Some(Decimal::from_str("0.01").unwrap()),
            fee_asset: Some("ETH".to_string()),
            external_id: None,
            notes: Some("BTC to ETH swap".to_string()),
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        let tx_id = repo.insert(&tx).await?;
        assert!(tx_id > 0);

        let transactions = repo.list(Some(10)).await?;
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].tx_type, TransactionType::Swap);
        assert_eq!(transactions[0].from_asset, Some("BTC".to_string()));
        assert_eq!(transactions[0].to_asset, Some("ETH".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_list_with_limit() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TransactionRepository::new(&pool);

        // Insert 5 transactions with different timestamps
        for i in 1..=5 {
            let timestamp = Utc.with_ymd_and_hms(2024, 1, i, 12, 0, 0).unwrap();
            let tx = Transaction {
                id: 0,
                tx_type: TransactionType::Buy,
                from_account_id: None,
                from_asset: None,
                from_quantity: None,
                to_account_id: Some("test-acc-1".to_string()),
                to_asset: Some("BTC".to_string()),
                to_quantity: Some(Decimal::from_str("1.0").unwrap()),
                price_usd: Some(Decimal::from_str(&(40000 + i * 1000).to_string()).unwrap()),
                price_currency: None,
                price_amount: None,
                exchange_rate: None,
                exchange_rate_pair: None,
                fee: None,
                fee_asset: None,
                external_id: None,
                notes: None,
                timestamp,
                created_at: Utc::now(),
            };
            repo.insert(&tx).await?;
        }

        // Test limit
        let transactions = repo.list(Some(3)).await?;
        assert_eq!(transactions.len(), 3);

        // Should be ordered by timestamp DESC (most recent first)
        assert_eq!(
            transactions[0].price_usd,
            Some(Decimal::from_str("45000").unwrap())
        );
        assert_eq!(
            transactions[1].price_usd,
            Some(Decimal::from_str("44000").unwrap())
        );
        assert_eq!(
            transactions[2].price_usd,
            Some(Decimal::from_str("43000").unwrap())
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_account() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TransactionRepository::new(&pool);

        // Insert transactions for different accounts
        let tx1 = Transaction {
            id: 0,
            tx_type: TransactionType::Buy,
            from_account_id: None,
            from_asset: None,
            from_quantity: None,
            to_account_id: Some("test-acc-1".to_string()),
            to_asset: Some("BTC".to_string()),
            to_quantity: Some(Decimal::from_str("1.0").unwrap()),
            price_usd: Some(Decimal::from_str("40000").unwrap()),
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: None,
            fee_asset: None,
            external_id: None,
            notes: None,
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        let tx2 = Transaction {
            id: 0,
            tx_type: TransactionType::Buy,
            from_account_id: None,
            from_asset: None,
            from_quantity: None,
            to_account_id: Some("test-acc-2".to_string()),
            to_asset: Some("ETH".to_string()),
            to_quantity: Some(Decimal::from_str("10.0").unwrap()),
            price_usd: Some(Decimal::from_str("3000").unwrap()),
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: None,
            fee_asset: None,
            external_id: None,
            notes: None,
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        let tx3 = Transaction {
            id: 0,
            tx_type: TransactionType::TransferInternal,
            from_account_id: Some("test-acc-1".to_string()),
            from_asset: Some("BTC".to_string()),
            from_quantity: Some(Decimal::from_str("0.5").unwrap()),
            to_account_id: Some("test-acc-2".to_string()),
            to_asset: Some("BTC".to_string()),
            to_quantity: Some(Decimal::from_str("0.5").unwrap()),
            price_usd: None,
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: None,
            fee_asset: None,
            external_id: None,
            notes: None,
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        repo.insert(&tx1).await?;
        repo.insert(&tx2).await?;
        repo.insert(&tx3).await?;

        // test-acc-1 should have 2 transactions (buy and transfer out)
        let acc1_txs = repo.list_by_account("test-acc-1", Some(10)).await?;
        assert_eq!(acc1_txs.len(), 2);

        // test-acc-2 should have 2 transactions (buy and transfer in)
        let acc2_txs = repo.list_by_account("test-acc-2", Some(10)).await?;
        assert_eq!(acc2_txs.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_decimal_precision() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TransactionRepository::new(&pool);

        // Test high precision decimals
        let tx = Transaction {
            id: 0,
            tx_type: TransactionType::Buy,
            from_account_id: None,
            from_asset: None,
            from_quantity: None,
            to_account_id: Some("test-acc-1".to_string()),
            to_asset: Some("BTC".to_string()),
            to_quantity: Some(Decimal::from_str("0.12345678").unwrap()),
            price_usd: Some(Decimal::from_str("45123.456789").unwrap()),
            price_currency: None,
            price_amount: None,
            exchange_rate: None,
            exchange_rate_pair: None,
            fee: Some(Decimal::from_str("0.00000001").unwrap()),
            fee_asset: Some("BTC".to_string()),
            external_id: None,
            notes: None,
            timestamp: Utc::now(),
            created_at: Utc::now(),
        };

        repo.insert(&tx).await?;

        let transactions = repo.list(Some(10)).await?;
        assert_eq!(transactions.len(), 1);
        assert_eq!(
            transactions[0].to_quantity,
            Some(Decimal::from_str("0.12345678").unwrap())
        );
        assert_eq!(
            transactions[0].price_usd,
            Some(Decimal::from_str("45123.456789").unwrap())
        );
        assert_eq!(
            transactions[0].fee,
            Some(Decimal::from_str("0.00000001").unwrap())
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_list() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TransactionRepository::new(&pool);

        let transactions = repo.list(Some(10)).await?;
        assert_eq!(transactions.len(), 0);

        let acc_txs = repo.list_by_account("test-acc-1", Some(10)).await?;
        assert_eq!(acc_txs.len(), 0);

        Ok(())
    }
}
