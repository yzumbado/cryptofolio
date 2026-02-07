use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::core::account::{Account, AccountConfig, AccountType, Category, WalletAddress};
use crate::error::{CryptofolioError, Result};

pub struct AccountRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> AccountRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    // === Categories ===

    pub async fn list_categories(&self) -> Result<Vec<Category>> {
        let rows = sqlx::query_as::<_, (String, String, i32, String)>(
            "SELECT id, name, sort_order, created_at FROM categories ORDER BY sort_order"
        )
        .fetch_all(self.pool)
        .await?;

        rows.into_iter()
            .map(|(id, name, sort_order, created_at)| {
                Ok(Category {
                    id,
                    name,
                    sort_order,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .collect()
    }

    pub async fn get_category(&self, id: &str) -> Result<Option<Category>> {
        let row = sqlx::query_as::<_, (String, String, i32, String)>(
            "SELECT id, name, sort_order, created_at FROM categories WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        match row {
            Some((id, name, sort_order, created_at)) => Ok(Some(Category {
                id,
                name,
                sort_order,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })),
            None => Ok(None),
        }
    }

    pub async fn get_category_by_name(&self, name: &str) -> Result<Option<Category>> {
        let row = sqlx::query_as::<_, (String, String, i32, String)>(
            "SELECT id, name, sort_order, created_at FROM categories WHERE LOWER(name) = LOWER(?)"
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?;

        match row {
            Some((id, name, sort_order, created_at)) => Ok(Some(Category {
                id,
                name,
                sort_order,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })),
            None => Ok(None),
        }
    }

    pub async fn create_category(&self, id: &str, name: &str) -> Result<()> {
        let max_order: Option<(i32,)> = sqlx::query_as(
            "SELECT MAX(sort_order) FROM categories"
        )
        .fetch_optional(self.pool)
        .await?;

        let sort_order = max_order.map(|(o,)| o + 1).unwrap_or(1);

        sqlx::query(
            "INSERT INTO categories (id, name, sort_order) VALUES (?, ?, ?)"
        )
        .bind(id)
        .bind(name)
        .bind(sort_order)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn rename_category(&self, old_name: &str, new_name: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE categories SET name = ? WHERE LOWER(name) = LOWER(?)"
        )
        .bind(new_name)
        .bind(old_name)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(CryptofolioError::CategoryNotFound(old_name.to_string()));
        }

        Ok(())
    }

    pub async fn delete_category(&self, name: &str) -> Result<()> {
        let result = sqlx::query(
            "DELETE FROM categories WHERE LOWER(name) = LOWER(?)"
        )
        .bind(name)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(CryptofolioError::CategoryNotFound(name.to_string()));
        }

        Ok(())
    }

    // === Accounts ===

    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, bool, String)>(
            "SELECT id, name, category_id, account_type, config, sync_enabled, created_at FROM accounts ORDER BY name"
        )
        .fetch_all(self.pool)
        .await?;

        rows.into_iter()
            .map(|(id, name, category_id, account_type, config, sync_enabled, created_at)| {
                let account_type = AccountType::from_str(&account_type)
                    .ok_or_else(|| CryptofolioError::Other(format!("Invalid account type: {}", account_type)))?;

                let config: AccountConfig = config
                    .as_deref()
                    .map(|c| serde_json::from_str(c).unwrap_or_default())
                    .unwrap_or_default();

                Ok(Account {
                    id,
                    name,
                    category_id,
                    account_type,
                    config,
                    sync_enabled,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .collect()
    }

    pub async fn get_account(&self, name: &str) -> Result<Option<Account>> {
        let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, bool, String)>(
            "SELECT id, name, category_id, account_type, config, sync_enabled, created_at FROM accounts WHERE LOWER(name) = LOWER(?)"
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?;

        match row {
            Some((id, name, category_id, account_type, config, sync_enabled, created_at)) => {
                let account_type = AccountType::from_str(&account_type)
                    .ok_or_else(|| CryptofolioError::Other(format!("Invalid account type: {}", account_type)))?;

                let config: AccountConfig = config
                    .as_deref()
                    .map(|c| serde_json::from_str(c).unwrap_or_default())
                    .unwrap_or_default();

                Ok(Some(Account {
                    id,
                    name,
                    category_id,
                    account_type,
                    config,
                    sync_enabled,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_account_by_id(&self, id: &str) -> Result<Option<Account>> {
        let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, bool, String)>(
            "SELECT id, name, category_id, account_type, config, sync_enabled, created_at FROM accounts WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        match row {
            Some((id, name, category_id, account_type, config, sync_enabled, created_at)) => {
                let account_type = AccountType::from_str(&account_type)
                    .ok_or_else(|| CryptofolioError::Other(format!("Invalid account type: {}", account_type)))?;

                let config: AccountConfig = config
                    .as_deref()
                    .map(|c| serde_json::from_str(c).unwrap_or_default())
                    .unwrap_or_default();

                Ok(Some(Account {
                    id,
                    name,
                    category_id,
                    account_type,
                    config,
                    sync_enabled,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn create_account(&self, account: &Account) -> Result<()> {
        let config_json = serde_json::to_string(&account.config)?;

        sqlx::query(
            "INSERT INTO accounts (id, name, category_id, account_type, config, sync_enabled) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&account.id)
        .bind(&account.name)
        .bind(&account.category_id)
        .bind(account.account_type.as_str())
        .bind(&config_json)
        .bind(account.sync_enabled)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_account(&self, name: &str) -> Result<()> {
        // First, get the account ID
        let account = self.get_account(name).await?
            .ok_or_else(|| CryptofolioError::AccountNotFound(name.to_string()))?;

        // Delete related records in order (respecting foreign keys)
        // 1. Delete wallet addresses
        sqlx::query("DELETE FROM wallet_addresses WHERE account_id = ?")
            .bind(&account.id)
            .execute(self.pool)
            .await?;

        // 2. Delete holdings
        sqlx::query("DELETE FROM holdings WHERE account_id = ?")
            .bind(&account.id)
            .execute(self.pool)
            .await?;

        // 3. Delete transactions (both from and to)
        sqlx::query("DELETE FROM transactions WHERE from_account_id = ? OR to_account_id = ?")
            .bind(&account.id)
            .bind(&account.id)
            .execute(self.pool)
            .await?;

        // 4. Finally delete the account
        sqlx::query("DELETE FROM accounts WHERE id = ?")
            .bind(&account.id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    // === Wallet Addresses ===

    pub async fn list_addresses(&self, account_id: &str) -> Result<Vec<WalletAddress>> {
        let rows = sqlx::query_as::<_, (i64, String, String, String, Option<String>, String)>(
            "SELECT id, account_id, blockchain, address, label, created_at FROM wallet_addresses WHERE account_id = ? ORDER BY blockchain"
        )
        .bind(account_id)
        .fetch_all(self.pool)
        .await?;

        rows.into_iter()
            .map(|(id, account_id, blockchain, address, label, created_at)| {
                Ok(WalletAddress {
                    id,
                    account_id,
                    blockchain,
                    address,
                    label,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .collect()
    }

    pub async fn add_address(
        &self,
        account_id: &str,
        blockchain: &str,
        address: &str,
        label: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO wallet_addresses (account_id, blockchain, address, label) VALUES (?, ?, ?, ?)"
        )
        .bind(account_id)
        .bind(blockchain)
        .bind(address)
        .bind(label)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_address(&self, account_id: &str, address: &str) -> Result<()> {
        let result = sqlx::query(
            "DELETE FROM wallet_addresses WHERE account_id = ? AND address = ?"
        )
        .bind(account_id)
        .bind(address)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(CryptofolioError::Other(format!("Address not found: {}", address)));
        }

        Ok(())
    }
}
