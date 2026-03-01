use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::core::pnl::{CostBasisMethod, TaxLot};
use crate::error::{CryptofolioError, Result};

pub struct TaxLotRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> TaxLotRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new tax lot when buying crypto
    pub async fn create(&self, lot: &TaxLot) -> Result<i64> {
        let quantity_str = lot.quantity.to_string();
        let remaining_str = lot.remaining_quantity.to_string();
        let price_str = lot.acquisition_price.to_string();
        let method_str = self.cost_basis_method_to_string(&lot.cost_basis_method);
        let acquisition_date_str = lot.acquisition_date.to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO tax_lots (
                account_id, asset, quantity, remaining_quantity, acquisition_price,
                acquisition_date, acquisition_tx_id, cost_basis_method, fully_disposed,
                created_at, updated_at
            )
            VALUES (?, UPPER(?), ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(&lot.account_id)
        .bind(&lot.asset)
        .bind(&quantity_str)
        .bind(&remaining_str)
        .bind(&price_str)
        .bind(&acquisition_date_str)
        .bind(lot.acquisition_tx_id)
        .bind(&method_str)
        .bind(lot.fully_disposed)
        .execute(self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get available lots for matching (FIFO/LIFO)
    /// Returns lots ordered by acquisition date based on the cost basis method
    pub async fn get_available_lots(
        &self,
        account_id: &str,
        asset: &str,
        method: CostBasisMethod,
    ) -> Result<Vec<TaxLot>> {
        let order_clause = match method {
            CostBasisMethod::Fifo => "ORDER BY acquisition_date ASC",
            CostBasisMethod::Lifo => "ORDER BY acquisition_date DESC",
            CostBasisMethod::AverageCost => "ORDER BY acquisition_date ASC", // Default to FIFO for average cost
        };

        let query = format!(
            r#"
            SELECT id, account_id, asset, quantity, remaining_quantity, acquisition_price,
                   acquisition_date, acquisition_tx_id, cost_basis_method, fully_disposed,
                   created_at, updated_at
            FROM tax_lots
            WHERE account_id = ? AND UPPER(asset) = UPPER(?)
              AND remaining_quantity > 0 AND fully_disposed = 0
            {}
            "#,
            order_clause
        );

        let rows = sqlx::query_as::<_, (
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<i64>,
            String,
            bool,
            String,
            String,
        )>(&query)
        .bind(account_id)
        .bind(asset)
        .fetch_all(self.pool)
        .await?;

        self.parse_tax_lots(rows)
    }

    /// Update the remaining quantity of a tax lot after partial disposal
    pub async fn update_remaining(&self, lot_id: i64, new_remaining: Decimal) -> Result<()> {
        let remaining_str = new_remaining.to_string();

        sqlx::query(
            "UPDATE tax_lots SET remaining_quantity = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(&remaining_str)
        .bind(lot_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Mark a lot as fully disposed (remaining = 0)
    pub async fn mark_disposed(&self, lot_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE tax_lots SET fully_disposed = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(lot_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// List all lots for a specific account and asset
    pub async fn list_by_account_asset(
        &self,
        account_id: &str,
        asset: &str,
    ) -> Result<Vec<TaxLot>> {
        let rows = sqlx::query_as::<_, (
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<i64>,
            String,
            bool,
            String,
            String,
        )>(
            r#"
            SELECT id, account_id, asset, quantity, remaining_quantity, acquisition_price,
                   acquisition_date, acquisition_tx_id, cost_basis_method, fully_disposed,
                   created_at, updated_at
            FROM tax_lots
            WHERE account_id = ? AND UPPER(asset) = UPPER(?)
            ORDER BY acquisition_date ASC
            "#,
        )
        .bind(account_id)
        .bind(asset)
        .fetch_all(self.pool)
        .await?;

        self.parse_tax_lots(rows)
    }

    /// List all lots for a specific account
    pub async fn list_by_account(&self, account_id: &str) -> Result<Vec<TaxLot>> {
        let rows = sqlx::query_as::<_, (
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<i64>,
            String,
            bool,
            String,
            String,
        )>(
            r#"
            SELECT id, account_id, asset, quantity, remaining_quantity, acquisition_price,
                   acquisition_date, acquisition_tx_id, cost_basis_method, fully_disposed,
                   created_at, updated_at
            FROM tax_lots
            WHERE account_id = ?
            ORDER BY acquisition_date ASC
            "#,
        )
        .bind(account_id)
        .fetch_all(self.pool)
        .await?;

        self.parse_tax_lots(rows)
    }

    fn parse_tax_lots(
        &self,
        rows: Vec<(
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<i64>,
            String,
            bool,
            String,
            String,
        )>,
    ) -> Result<Vec<TaxLot>> {
        rows.into_iter().map(|r| self.parse_tax_lot(r)).collect()
    }

    fn parse_tax_lot(
        &self,
        (
            id,
            account_id,
            asset,
            quantity,
            remaining_quantity,
            acquisition_price,
            acquisition_date,
            acquisition_tx_id,
            cost_basis_method,
            fully_disposed,
            created_at,
            updated_at,
        ): (
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<i64>,
            String,
            bool,
            String,
            String,
        ),
    ) -> Result<TaxLot> {
        let quantity = Decimal::from_str(&quantity)
            .map_err(|_| CryptofolioError::InvalidAmount(quantity))?;

        let remaining_quantity = Decimal::from_str(&remaining_quantity)
            .map_err(|_| CryptofolioError::InvalidAmount(remaining_quantity))?;

        let acquisition_price = Decimal::from_str(&acquisition_price)
            .map_err(|_| CryptofolioError::InvalidAmount(acquisition_price))?;

        let acquisition_date = DateTime::parse_from_rfc3339(&acquisition_date)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| {
                CryptofolioError::Other(format!(
                    "Invalid acquisition date: {}",
                    acquisition_date
                ))
            })?;

        let cost_basis_method = self.string_to_cost_basis_method(&cost_basis_method)?;

        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(TaxLot {
            id,
            account_id,
            asset,
            quantity,
            remaining_quantity,
            acquisition_price,
            acquisition_date,
            acquisition_tx_id,
            cost_basis_method,
            fully_disposed,
            created_at,
            updated_at,
        })
    }

    fn cost_basis_method_to_string(&self, method: &CostBasisMethod) -> String {
        match method {
            CostBasisMethod::Fifo => "fifo".to_string(),
            CostBasisMethod::Lifo => "lifo".to_string(),
            CostBasisMethod::AverageCost => "average".to_string(),
        }
    }

    fn string_to_cost_basis_method(&self, s: &str) -> Result<CostBasisMethod> {
        match s.to_lowercase().as_str() {
            "fifo" => Ok(CostBasisMethod::Fifo),
            "lifo" => Ok(CostBasisMethod::Lifo),
            "average" => Ok(CostBasisMethod::AverageCost),
            _ => Err(CryptofolioError::InvalidCostBasisMethod(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use std::str::FromStr;

    async fn setup_test_db() -> Result<SqlitePool> {
        let pool = SqlitePool::connect(":memory:").await?;
        migrations::run(&pool).await?;
        Ok(pool)
    }

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).expect("Invalid decimal in test")
    }

    #[tokio::test]
    async fn test_create_tax_lot() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TaxLotRepository::new(&pool);

        // Create test account first
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO accounts (id, name, account_type, category_id, config, sync_enabled) VALUES ('test_acct', 'Test', 'exchange', 'test', '{}', 0)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO transactions (id, timestamp, tx_type) VALUES (1, CURRENT_TIMESTAMP, 'buy')")
            .execute(&pool)
            .await?;

        let lot = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("1.5"),
            remaining_quantity: dec("1.5"),
            acquisition_price: dec("45000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Fifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let lot_id = repo.create(&lot).await?;
        assert!(lot_id > 0, "Should return valid lot ID");

        // Verify lot was created
        let lots = repo.list_by_account_asset("test_acct", "BTC").await?;
        assert_eq!(lots.len(), 1, "Should have 1 tax lot");
        assert_eq!(lots[0].quantity, dec("1.5"));
        assert_eq!(lots[0].acquisition_price, dec("45000"));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_available_lots_fifo_ordering() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TaxLotRepository::new(&pool);

        // Setup account
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO accounts (id, name, account_type, category_id, config, sync_enabled) VALUES ('test_acct', 'Test', 'exchange', 'test', '{}', 0)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO transactions (id, timestamp, tx_type) VALUES (1, CURRENT_TIMESTAMP, 'buy')")
            .execute(&pool)
            .await?;

        // Create 3 lots at different times
        use std::thread;
        use std::time::Duration;

        let lot1 = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("1.0"),
            remaining_quantity: dec("1.0"),
            acquisition_price: dec("40000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Fifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&lot1).await?;

        thread::sleep(Duration::from_millis(10));

        let lot2 = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("2.0"),
            remaining_quantity: dec("2.0"),
            acquisition_price: dec("45000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Fifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&lot2).await?;

        thread::sleep(Duration::from_millis(10));

        let lot3 = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("0.5"),
            remaining_quantity: dec("0.5"),
            acquisition_price: dec("50000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Fifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&lot3).await?;

        // Get lots with FIFO ordering (oldest first)
        let fifo_lots = repo
            .get_available_lots("test_acct", "BTC", CostBasisMethod::Fifo)
            .await?;

        assert_eq!(fifo_lots.len(), 3, "Should have 3 available lots");
        assert_eq!(fifo_lots[0].acquisition_price, dec("40000"), "First lot should be oldest");
        assert_eq!(fifo_lots[1].acquisition_price, dec("45000"), "Second lot should be middle");
        assert_eq!(fifo_lots[2].acquisition_price, dec("50000"), "Third lot should be newest");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_available_lots_lifo_ordering() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TaxLotRepository::new(&pool);

        // Setup account
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO accounts (id, name, account_type, category_id, config, sync_enabled) VALUES ('test_acct', 'Test', 'exchange', 'test', '{}', 0)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO transactions (id, timestamp, tx_type) VALUES (1, CURRENT_TIMESTAMP, 'buy')")
            .execute(&pool)
            .await?;

        // Create lots (same as FIFO test)
        use std::thread;
        use std::time::Duration;

        let lot1 = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("1.0"),
            remaining_quantity: dec("1.0"),
            acquisition_price: dec("40000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Lifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&lot1).await?;

        thread::sleep(Duration::from_millis(10));

        let lot2 = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("2.0"),
            remaining_quantity: dec("2.0"),
            acquisition_price: dec("45000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Lifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&lot2).await?;

        thread::sleep(Duration::from_millis(10));

        let lot3 = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("0.5"),
            remaining_quantity: dec("0.5"),
            acquisition_price: dec("50000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Lifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&lot3).await?;

        // Get lots with LIFO ordering (newest first)
        let lifo_lots = repo
            .get_available_lots("test_acct", "BTC", CostBasisMethod::Lifo)
            .await?;

        assert_eq!(lifo_lots.len(), 3, "Should have 3 available lots");
        assert_eq!(lifo_lots[0].acquisition_price, dec("50000"), "First lot should be newest");
        assert_eq!(lifo_lots[1].acquisition_price, dec("45000"), "Second lot should be middle");
        assert_eq!(lifo_lots[2].acquisition_price, dec("40000"), "Third lot should be oldest");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_remaining_quantity() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TaxLotRepository::new(&pool);

        // Setup
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO accounts (id, name, account_type, category_id, config, sync_enabled) VALUES ('test_acct', 'Test', 'exchange', 'test', '{}', 0)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO transactions (id, timestamp, tx_type) VALUES (1, CURRENT_TIMESTAMP, 'buy')")
            .execute(&pool)
            .await?;

        let lot = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("1.0"),
            remaining_quantity: dec("1.0"),
            acquisition_price: dec("40000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Fifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let lot_id = repo.create(&lot).await?;

        // Update remaining quantity
        repo.update_remaining(lot_id, dec("0.3")).await?;

        // Verify update
        let lots = repo.list_by_account_asset("test_acct", "BTC").await?;
        assert_eq!(lots[0].remaining_quantity, dec("0.3"), "Remaining quantity should be updated");
        assert_eq!(lots[0].quantity, dec("1.0"), "Original quantity should not change");

        Ok(())
    }

    #[tokio::test]
    async fn test_mark_disposed() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TaxLotRepository::new(&pool);

        // Setup
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO accounts (id, name, account_type, category_id, config, sync_enabled) VALUES ('test_acct', 'Test', 'exchange', 'test', '{}', 0)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO transactions (id, timestamp, tx_type) VALUES (1, CURRENT_TIMESTAMP, 'buy')")
            .execute(&pool)
            .await?;

        let lot = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("1.0"),
            remaining_quantity: dec("1.0"),
            acquisition_price: dec("40000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Fifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let lot_id = repo.create(&lot).await?;

        // Mark as disposed
        repo.mark_disposed(lot_id).await?;

        // Verify it's marked as disposed
        let lots = repo.list_by_account_asset("test_acct", "BTC").await?;
        assert_eq!(lots[0].fully_disposed, true, "Lot should be marked as fully disposed");

        // Verify it's excluded from available lots
        let available = repo
            .get_available_lots("test_acct", "BTC", CostBasisMethod::Fifo)
            .await?;
        assert_eq!(available.len(), 0, "Disposed lot should not be available");

        Ok(())
    }

    #[tokio::test]
    async fn test_cost_basis_method_string_conversion() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TaxLotRepository::new(&pool);

        // Test to_string
        assert_eq!(repo.cost_basis_method_to_string(&CostBasisMethod::Fifo), "fifo");
        assert_eq!(repo.cost_basis_method_to_string(&CostBasisMethod::Lifo), "lifo");
        assert_eq!(repo.cost_basis_method_to_string(&CostBasisMethod::AverageCost), "average");

        // Test from_string
        assert!(matches!(repo.string_to_cost_basis_method("fifo")?, CostBasisMethod::Fifo));
        assert!(matches!(repo.string_to_cost_basis_method("FIFO")?, CostBasisMethod::Fifo));
        assert!(matches!(repo.string_to_cost_basis_method("lifo")?, CostBasisMethod::Lifo));
        assert!(matches!(repo.string_to_cost_basis_method("LIFO")?, CostBasisMethod::Lifo));
        assert!(matches!(repo.string_to_cost_basis_method("average")?, CostBasisMethod::AverageCost));
        assert!(matches!(repo.string_to_cost_basis_method("AVERAGE")?, CostBasisMethod::AverageCost));

        // Test invalid method
        let result = repo.string_to_cost_basis_method("invalid");
        assert!(result.is_err(), "Invalid method should return error");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_account() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = TaxLotRepository::new(&pool);

        // Setup
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO accounts (id, name, account_type, category_id, config, sync_enabled) VALUES ('test_acct', 'Test', 'exchange', 'test', '{}', 0)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO transactions (id, timestamp, tx_type) VALUES (1, CURRENT_TIMESTAMP, 'buy')")
            .execute(&pool)
            .await?;

        // Create lots for different assets
        let btc_lot = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "BTC".to_string(),
            quantity: dec("1.0"),
            remaining_quantity: dec("1.0"),
            acquisition_price: dec("40000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Fifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&btc_lot).await?;

        let eth_lot = TaxLot {
            id: 0,
            account_id: "test_acct".to_string(),
            asset: "ETH".to_string(),
            quantity: dec("10.0"),
            remaining_quantity: dec("10.0"),
            acquisition_price: dec("3000"),
            acquisition_date: Utc::now(),
            acquisition_tx_id: Some(1),
            cost_basis_method: CostBasisMethod::Fifo,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        repo.create(&eth_lot).await?;

        // List all lots for account
        let all_lots = repo.list_by_account("test_acct").await?;
        assert_eq!(all_lots.len(), 2, "Should have 2 lots across different assets");

        // Verify we have both BTC and ETH
        let assets: Vec<&str> = all_lots.iter().map(|lot| lot.asset.as_str()).collect();
        assert!(assets.contains(&"BTC"), "Should have BTC lot");
        assert!(assets.contains(&"ETH"), "Should have ETH lot");

        Ok(())
    }
}
