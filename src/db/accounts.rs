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
            "SELECT id, name, sort_order, created_at FROM categories ORDER BY sort_order",
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
            "SELECT id, name, sort_order, created_at FROM categories WHERE id = ?",
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
            "SELECT id, name, sort_order, created_at FROM categories WHERE LOWER(name) = LOWER(?)",
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
        let max_order: Option<(i32,)> = sqlx::query_as("SELECT MAX(sort_order) FROM categories")
            .fetch_optional(self.pool)
            .await?;

        let sort_order = max_order.map(|(o,)| o + 1).unwrap_or(1);

        sqlx::query("INSERT INTO categories (id, name, sort_order) VALUES (?, ?, ?)")
            .bind(id)
            .bind(name)
            .bind(sort_order)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    pub async fn rename_category(&self, old_name: &str, new_name: &str) -> Result<()> {
        let result = sqlx::query("UPDATE categories SET name = ? WHERE LOWER(name) = LOWER(?)")
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
        let result = sqlx::query("DELETE FROM categories WHERE LOWER(name) = LOWER(?)")
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
            .map(
                |(id, name, category_id, account_type, config, sync_enabled, created_at)| {
                    let account_type = AccountType::from_str(&account_type).ok_or_else(|| {
                        CryptofolioError::Other(format!("Invalid account type: {}", account_type))
                    })?;

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
                },
            )
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
                let account_type = AccountType::from_str(&account_type).ok_or_else(|| {
                    CryptofolioError::Other(format!("Invalid account type: {}", account_type))
                })?;

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
                let account_type = AccountType::from_str(&account_type).ok_or_else(|| {
                    CryptofolioError::Other(format!("Invalid account type: {}", account_type))
                })?;

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
        let account = self
            .get_account(name)
            .await?
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
        let rows = sqlx::query_as::<_, (i64, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String)>(
            "SELECT id, account_id, blockchain, address, address_type, label, xpub, derivation_path, network, last_synced_at, created_at FROM wallet_addresses WHERE account_id = ? ORDER BY blockchain"
        )
        .bind(account_id)
        .fetch_all(self.pool)
        .await?;

        rows.into_iter()
            .map(
                |(
                    id,
                    account_id,
                    blockchain,
                    address,
                    address_type,
                    label,
                    xpub,
                    derivation_path,
                    network,
                    last_synced_at,
                    created_at,
                )| {
                    Ok(WalletAddress {
                        id,
                        account_id,
                        blockchain,
                        address,
                        address_type,
                        label,
                        xpub,
                        derivation_path,
                        network,
                        last_synced_at: last_synced_at.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                        }),
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                },
            )
            .collect()
    }

    pub async fn add_address(
        &self,
        account_id: &str,
        blockchain: &str,
        address: &str,
        label: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO wallet_addresses (account_id, blockchain, address, label) VALUES (?, ?, ?, ?)"
        )
        .bind(account_id)
        .bind(blockchain)
        .bind(address)
        .bind(label)
        .execute(self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn add_address_with_xpub(
        &self,
        account_id: &str,
        blockchain: &str,
        address: &str,
        label: Option<&str>,
        xpub: Option<&str>,
        derivation_path: Option<&str>,
        address_type: Option<&str>,
        network: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO wallet_addresses (account_id, blockchain, address, label, xpub, derivation_path, address_type, network) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(account_id)
        .bind(blockchain)
        .bind(address)
        .bind(label)
        .bind(xpub)
        .bind(derivation_path)
        .bind(address_type)
        .bind(network)
        .execute(self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn remove_address(&self, account_id: &str, address: &str) -> Result<()> {
        let result =
            sqlx::query("DELETE FROM wallet_addresses WHERE account_id = ? AND address = ?")
                .bind(account_id)
                .bind(address)
                .execute(self.pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(CryptofolioError::Other(format!(
                "Address not found: {}",
                address
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::account::{AccountConfig, AccountType};
    use crate::db::migrations;

    async fn setup_test_db() -> Result<SqlitePool> {
        let pool = SqlitePool::connect(":memory:").await?;
        migrations::run(&pool).await?;
        Ok(pool)
    }

    // ============ Category Tests ============

    #[tokio::test]
    async fn test_list_categories() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let categories = repo.list_categories().await?;

        // Should have default categories from migration
        assert!(categories.len() >= 3);
        assert!(categories.iter().any(|c| c.id == "trading"));
        assert!(categories.iter().any(|c| c.id == "cold-storage"));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_category() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let category = repo.get_category("trading").await?;
        assert!(category.is_some());
        assert_eq!(category.unwrap().name, "Trading");

        let not_found = repo.get_category("nonexistent").await?;
        assert!(not_found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_category_by_name() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        // Case-insensitive lookup
        let category = repo.get_category_by_name("trading").await?;
        assert!(category.is_some());

        let category = repo.get_category_by_name("TRADING").await?;
        assert!(category.is_some());

        let not_found = repo.get_category_by_name("nonexistent").await?;
        assert!(not_found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_create_category() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        repo.create_category("test-cat", "Test Category").await?;

        let category = repo.get_category("test-cat").await?;
        assert!(category.is_some());
        let cat = category.unwrap();
        assert_eq!(cat.id, "test-cat");
        assert_eq!(cat.name, "Test Category");
        assert!(cat.sort_order > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_rename_category() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        repo.create_category("test-cat", "Old Name").await?;
        repo.rename_category("Old Name", "New Name").await?;

        let category = repo.get_category_by_name("New Name").await?;
        assert!(category.is_some());

        let old = repo.get_category_by_name("Old Name").await?;
        assert!(old.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_rename_category_not_found() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let result = repo.rename_category("NonExistent", "New Name").await;
        assert!(result.is_err());
        match result {
            Err(CryptofolioError::CategoryNotFound(_)) => {}
            _ => panic!("Expected CategoryNotFound error"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_category() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        repo.create_category("test-cat", "Test Category").await?;
        repo.delete_category("Test Category").await?;

        let category = repo.get_category("test-cat").await?;
        assert!(category.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_category_not_found() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let result = repo.delete_category("NonExistent").await;
        assert!(result.is_err());
        match result {
            Err(CryptofolioError::CategoryNotFound(_)) => {}
            _ => panic!("Expected CategoryNotFound error"),
        }

        Ok(())
    }

    // ============ Account Tests ============

    #[tokio::test]
    async fn test_create_and_list_accounts() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let account = Account {
            id: "test-acc-1".to_string(),
            name: "Test Account".to_string(),
            category_id: "trading".to_string(),
            account_type: AccountType::Exchange,
            config: AccountConfig::default(),
            sync_enabled: false,
            created_at: Utc::now(),
        };

        repo.create_account(&account).await?;

        let accounts = repo.list_accounts().await?;
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].name, "Test Account");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_account() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let account = Account {
            id: "test-acc".to_string(),
            name: "My Wallet".to_string(),
            category_id: "trading".to_string(),
            account_type: AccountType::SoftwareWallet,
            config: AccountConfig::default(),
            sync_enabled: true,
            created_at: Utc::now(),
        };

        repo.create_account(&account).await?;

        // Case-insensitive lookup
        let found = repo.get_account("my wallet").await?;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "test-acc");

        let found = repo.get_account("MY WALLET").await?;
        assert!(found.is_some());

        let not_found = repo.get_account("NonExistent").await?;
        assert!(not_found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_account_by_id() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let account = Account {
            id: "test-acc-id".to_string(),
            name: "Test Account".to_string(),
            category_id: "trading".to_string(),
            account_type: AccountType::Exchange,
            config: AccountConfig::default(),
            sync_enabled: false,
            created_at: Utc::now(),
        };

        repo.create_account(&account).await?;

        let found = repo.get_account_by_id("test-acc-id").await?;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Account");

        let not_found = repo.get_account_by_id("nonexistent-id").await?;
        assert!(not_found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_account_with_config() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let account = Account {
            id: "test-acc".to_string(),
            name: "Test Account".to_string(),
            category_id: "trading".to_string(),
            account_type: AccountType::Exchange,
            config: AccountConfig { is_testnet: true },
            sync_enabled: false,
            created_at: Utc::now(),
        };

        repo.create_account(&account).await?;

        let found = repo.get_account("Test Account").await?;
        assert!(found.is_some());
        assert!(found.unwrap().config.is_testnet);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_account() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let account = Account {
            id: "test-acc".to_string(),
            name: "Test Account".to_string(),
            category_id: "trading".to_string(),
            account_type: AccountType::Exchange,
            config: AccountConfig::default(),
            sync_enabled: false,
            created_at: Utc::now(),
        };

        repo.create_account(&account).await?;

        // Add a wallet address and holding to test cascading delete
        repo.add_address("test-acc", "Bitcoin", "bc1qtest", None)
            .await?;

        sqlx::query("INSERT INTO holdings (account_id, asset, quantity) VALUES (?, ?, ?)")
            .bind("test-acc")
            .bind("BTC")
            .bind("1.0")
            .execute(&pool)
            .await?;

        repo.delete_account("Test Account").await?;

        // Verify account is deleted
        let account = repo.get_account("Test Account").await?;
        assert!(account.is_none());

        // Verify related records are deleted (cascading)
        let addresses = repo.list_addresses("test-acc").await?;
        assert_eq!(addresses.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_account_not_found() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        let result = repo.delete_account("NonExistent").await;
        assert!(result.is_err());
        match result {
            Err(CryptofolioError::AccountNotFound(_)) => {}
            _ => panic!("Expected AccountNotFound error"),
        }

        Ok(())
    }

    // ============ Wallet Address Tests ============

    #[tokio::test]
    async fn test_add_and_list_addresses() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        // Create account first
        let account = Account {
            id: "test-acc".to_string(),
            name: "Test Wallet".to_string(),
            category_id: "trading".to_string(),
            account_type: AccountType::HardwareWallet,
            config: AccountConfig::default(),
            sync_enabled: false,
            created_at: Utc::now(),
        };

        repo.create_account(&account).await?;

        // Add addresses
        repo.add_address("test-acc", "Bitcoin", "bc1qtest1", Some("Main"))
            .await?;
        repo.add_address("test-acc", "Ethereum", "0xtest1", None)
            .await?;

        let addresses = repo.list_addresses("test-acc").await?;
        assert_eq!(addresses.len(), 2);

        // Should be ordered by blockchain
        assert_eq!(addresses[0].blockchain, "Bitcoin");
        assert_eq!(addresses[0].label, Some("Main".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_address() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        // Create account first
        let account = Account {
            id: "test-acc".to_string(),
            name: "Test Wallet".to_string(),
            category_id: "trading".to_string(),
            account_type: AccountType::HardwareWallet,
            config: AccountConfig::default(),
            sync_enabled: false,
            created_at: Utc::now(),
        };

        repo.create_account(&account).await?;
        repo.add_address("test-acc", "Bitcoin", "bc1qtest", None)
            .await?;

        repo.remove_address("test-acc", "bc1qtest").await?;

        let addresses = repo.list_addresses("test-acc").await?;
        assert_eq!(addresses.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_address_not_found() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = AccountRepository::new(&pool);

        // Create account first
        let account = Account {
            id: "test-acc".to_string(),
            name: "Test Wallet".to_string(),
            category_id: "trading".to_string(),
            account_type: AccountType::HardwareWallet,
            config: AccountConfig::default(),
            sync_enabled: false,
            created_at: Utc::now(),
        };

        repo.create_account(&account).await?;

        let result = repo.remove_address("test-acc", "nonexistent").await;
        assert!(result.is_err());

        Ok(())
    }
}
