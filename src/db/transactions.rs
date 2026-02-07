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
            "#
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(|r| self.parse_transaction(r)).collect()
    }

    pub async fn list_by_account(&self, account_id: &str, limit: Option<i64>) -> Result<Vec<Transaction>> {
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
            "#
        )
        .bind(account_id)
        .bind(account_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(|r| self.parse_transaction(r)).collect()
    }

    pub async fn insert(&self, tx: &Transaction) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO transactions (
                tx_type, from_account_id, from_asset, from_quantity,
                to_account_id, to_asset, to_quantity, price_usd, fee, fee_asset,
                external_id, notes, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
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
        let tx_type = TransactionType::from_str(&row.tx_type)
            .ok_or_else(|| CryptofolioError::Other(format!("Invalid transaction type: {}", row.tx_type)))?;

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
