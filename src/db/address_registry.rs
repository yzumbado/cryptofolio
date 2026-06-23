use sqlx::SqlitePool;

use crate::error::Result;

pub struct AddressRegistry<'a> {
    pool: &'a SqlitePool,
}

#[derive(Debug, Clone)]
pub struct AddressRecord {
    pub address: String,
    pub chain: String,
    pub classification: String,
    pub label: Option<String>,
    pub account_id: Option<String>,
}

impl<'a> AddressRegistry<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert or ignore (address is the PK; duplicate silently skipped).
    pub async fn upsert(
        &self,
        address: &str,
        chain: &str,
        classification: &str,
        label: Option<&str>,
        account_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO address_registry (address, chain, classification, label, account_id)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(address) DO NOTHING
            "#,
        )
        .bind(address)
        .bind(chain)
        .bind(classification)
        .bind(label)
        .bind(account_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn get(&self, address: &str) -> Result<Option<AddressRecord>> {
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>)>(
            r#"
            SELECT address, chain, classification, label, account_id
            FROM address_registry
            WHERE address = ?
            "#,
        )
        .bind(address)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(
            |(address, chain, classification, label, account_id)| AddressRecord {
                address,
                chain,
                classification,
                label,
                account_id,
            },
        ))
    }
}
