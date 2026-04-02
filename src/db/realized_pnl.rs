use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::core::pnl::{CostBasisMethod, RealizedPnL};
use crate::error::{CryptofolioError, Result};

pub struct RealizedPnLRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> RealizedPnLRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Record realized P&L from a disposal
    pub async fn create(&self, pnl: &RealizedPnL) -> Result<i64> {
        let quantity_str = pnl.quantity.to_string();
        let proceeds_str = pnl.proceeds.to_string();
        let cost_basis_str = pnl.cost_basis.to_string();
        let realized_gain_str = pnl.realized_gain.to_string();
        let method_str = self.cost_basis_method_to_string(&pnl.cost_basis_method);
        let disposal_date_str = pnl.disposal_date.to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO realized_pnl (
                account_id, asset, disposal_date, disposal_tx_id, quantity,
                proceeds, cost_basis, realized_gain, holding_period_days,
                tax_lot_id, cost_basis_method, created_at
            )
            VALUES (?, UPPER(?), ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(&pnl.account_id)
        .bind(&pnl.asset)
        .bind(&disposal_date_str)
        .bind(pnl.disposal_tx_id)
        .bind(&quantity_str)
        .bind(&proceeds_str)
        .bind(&cost_basis_str)
        .bind(&realized_gain_str)
        .bind(pnl.holding_period_days)
        .bind(pnl.tax_lot_id)
        .bind(&method_str)
        .execute(self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// List realized P&L by date range
    pub async fn list_by_date_range(
        &self,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<RealizedPnL>> {
        let mut query = String::from(
            r#"
            SELECT id, account_id, asset, disposal_date, disposal_tx_id, quantity,
                   proceeds, cost_basis, realized_gain, holding_period_days,
                   tax_lot_id, cost_basis_method, created_at
            FROM realized_pnl
            WHERE 1=1
            "#,
        );

        let mut bindings: Vec<String> = Vec::new();

        if let Some(from_date) = from {
            query.push_str(" AND disposal_date >= ?");
            bindings.push(from_date.to_rfc3339());
        }

        if let Some(to_date) = to {
            query.push_str(" AND disposal_date <= ?");
            bindings.push(to_date.to_rfc3339());
        }

        query.push_str(" ORDER BY disposal_date DESC");

        let mut q = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                Option<i64>,
                String,
                String,
                String,
                String,
                Option<i64>,
                Option<i64>,
                String,
                String,
            ),
        >(&query);

        for binding in bindings {
            q = q.bind(binding);
        }

        let rows = q.fetch_all(self.pool).await?;

        self.parse_realized_pnls(rows)
    }

    /// List realized P&L by account
    pub async fn list_by_account(&self, account_id: &str) -> Result<Vec<RealizedPnL>> {
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                Option<i64>,
                String,
                String,
                String,
                String,
                Option<i64>,
                Option<i64>,
                String,
                String,
            ),
        >(
            r#"
            SELECT id, account_id, asset, disposal_date, disposal_tx_id, quantity,
                   proceeds, cost_basis, realized_gain, holding_period_days,
                   tax_lot_id, cost_basis_method, created_at
            FROM realized_pnl
            WHERE account_id = ?
            ORDER BY disposal_date DESC
            "#,
        )
        .bind(account_id)
        .fetch_all(self.pool)
        .await?;

        self.parse_realized_pnls(rows)
    }

    /// List realized P&L by asset
    pub async fn list_by_asset(&self, asset: &str) -> Result<Vec<RealizedPnL>> {
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                Option<i64>,
                String,
                String,
                String,
                String,
                Option<i64>,
                Option<i64>,
                String,
                String,
            ),
        >(
            r#"
            SELECT id, account_id, asset, disposal_date, disposal_tx_id, quantity,
                   proceeds, cost_basis, realized_gain, holding_period_days,
                   tax_lot_id, cost_basis_method, created_at
            FROM realized_pnl
            WHERE UPPER(asset) = UPPER(?)
            ORDER BY disposal_date DESC
            "#,
        )
        .bind(asset)
        .fetch_all(self.pool)
        .await?;

        self.parse_realized_pnls(rows)
    }

    /// List realized P&L with filters
    pub async fn list_filtered(
        &self,
        account_id: Option<&str>,
        asset: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<i64>,
    ) -> Result<Vec<RealizedPnL>> {
        let mut query = String::from(
            r#"
            SELECT id, account_id, asset, disposal_date, disposal_tx_id, quantity,
                   proceeds, cost_basis, realized_gain, holding_period_days,
                   tax_lot_id, cost_basis_method, created_at
            FROM realized_pnl
            WHERE 1=1
            "#,
        );

        let mut bindings: Vec<String> = Vec::new();

        if let Some(acc) = account_id {
            query.push_str(" AND account_id = ?");
            bindings.push(acc.to_string());
        }

        if let Some(ast) = asset {
            query.push_str(" AND UPPER(asset) = UPPER(?)");
            bindings.push(ast.to_string());
        }

        if let Some(from_date) = from {
            query.push_str(" AND disposal_date >= ?");
            bindings.push(from_date.to_rfc3339());
        }

        if let Some(to_date) = to {
            query.push_str(" AND disposal_date <= ?");
            bindings.push(to_date.to_rfc3339());
        }

        query.push_str(" ORDER BY disposal_date DESC");

        if let Some(lim) = limit {
            // sqlx does not support binding LIMIT as a parameter in query_as;
            // lim is typed as usize so string formatting is injection-safe.
            query.push_str(&format!(" LIMIT {}", lim));
        }

        let mut q = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                Option<i64>,
                String,
                String,
                String,
                String,
                Option<i64>,
                Option<i64>,
                String,
                String,
            ),
        >(&query);

        for binding in bindings {
            q = q.bind(binding);
        }

        let rows = q.fetch_all(self.pool).await?;

        self.parse_realized_pnls(rows)
    }

    /// Get summary of realized gains (total)
    pub async fn get_summary(
        &self,
        account_id: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Decimal> {
        let mut query = String::from(
            "SELECT COALESCE(SUM(CAST(realized_gain AS REAL)), 0.0) FROM realized_pnl WHERE 1=1",
        );

        let mut bindings: Vec<String> = Vec::new();

        if let Some(acc) = account_id {
            query.push_str(" AND account_id = ?");
            bindings.push(acc.to_string());
        }

        if let Some(from_date) = from {
            query.push_str(" AND disposal_date >= ?");
            bindings.push(from_date.to_rfc3339());
        }

        if let Some(to_date) = to {
            query.push_str(" AND disposal_date <= ?");
            bindings.push(to_date.to_rfc3339());
        }

        let mut q = sqlx::query_scalar::<_, f64>(&query);

        for binding in bindings {
            q = q.bind(binding);
        }

        let sum = q.fetch_one(self.pool).await?;

        Ok(Decimal::from_f64_retain(sum).unwrap_or(Decimal::ZERO))
    }

    fn parse_realized_pnls(
        &self,
        rows: Vec<(
            i64,
            String,
            String,
            String,
            Option<i64>,
            String,
            String,
            String,
            String,
            Option<i64>,
            Option<i64>,
            String,
            String,
        )>,
    ) -> Result<Vec<RealizedPnL>> {
        rows.into_iter()
            .map(|r| self.parse_realized_pnl(r))
            .collect()
    }

    fn parse_realized_pnl(
        &self,
        (
            id,
            account_id,
            asset,
            disposal_date,
            disposal_tx_id,
            quantity,
            proceeds,
            cost_basis,
            realized_gain,
            holding_period_days,
            tax_lot_id,
            cost_basis_method,
            created_at,
        ): (
            i64,
            String,
            String,
            String,
            Option<i64>,
            String,
            String,
            String,
            String,
            Option<i64>,
            Option<i64>,
            String,
            String,
        ),
    ) -> Result<RealizedPnL> {
        let quantity =
            Decimal::from_str(&quantity).map_err(|_| CryptofolioError::InvalidAmount(quantity))?;

        let proceeds =
            Decimal::from_str(&proceeds).map_err(|_| CryptofolioError::InvalidAmount(proceeds))?;

        let cost_basis = Decimal::from_str(&cost_basis)
            .map_err(|_| CryptofolioError::InvalidAmount(cost_basis))?;

        let realized_gain = Decimal::from_str(&realized_gain)
            .map_err(|_| CryptofolioError::InvalidAmount(realized_gain))?;

        let disposal_date = DateTime::parse_from_rfc3339(&disposal_date)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| {
                CryptofolioError::Other(format!("Invalid disposal date: {}", disposal_date))
            })?;

        let cost_basis_method = self.string_to_cost_basis_method(&cost_basis_method)?;

        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(RealizedPnL {
            id,
            account_id,
            asset,
            disposal_date,
            disposal_tx_id,
            quantity,
            proceeds,
            cost_basis,
            realized_gain,
            holding_period_days,
            tax_lot_id,
            cost_basis_method,
            created_at,
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
    use chrono::TimeZone;

    async fn setup_test_db() -> Result<SqlitePool> {
        let pool = SqlitePool::connect(":memory:").await?;
        migrations::run(&pool).await?;

        // Create test account and category
        sqlx::query("INSERT INTO categories (id, name) VALUES ('test-cat', 'Test Category')")
            .execute(&pool)
            .await?;

        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) VALUES ('test-acc', 'test-cat', 'Test Account', 'exchange')",
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    #[tokio::test]
    async fn test_create_realized_pnl() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = RealizedPnLRepository::new(&pool);

        let disposal_date = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let pnl = RealizedPnL {
            id: 0, // Will be set by database
            account_id: "test-acc".to_string(),
            asset: "BTC".to_string(),
            disposal_date,
            disposal_tx_id: None,
            quantity: Decimal::from_str("1.5").unwrap(),
            proceeds: Decimal::from_str("75000").unwrap(),
            cost_basis: Decimal::from_str("60000").unwrap(),
            realized_gain: Decimal::from_str("15000").unwrap(),
            holding_period_days: Some(365),
            tax_lot_id: None,
            cost_basis_method: CostBasisMethod::Fifo,
            created_at: Utc::now(),
        };

        let pnl_id = repo.create(&pnl).await?;
        assert!(pnl_id > 0);

        // Verify it was created
        let results = repo.list_by_account("test-acc").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].asset, "BTC");
        assert_eq!(results[0].quantity, Decimal::from_str("1.5").unwrap());
        assert_eq!(
            results[0].realized_gain,
            Decimal::from_str("15000").unwrap()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_date_range() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = RealizedPnLRepository::new(&pool);

        // Create P&L records on different dates
        let date1 = Utc.with_ymd_and_hms(2024, 1, 10, 12, 0, 0).unwrap();
        let date2 = Utc.with_ymd_and_hms(2024, 1, 20, 12, 0, 0).unwrap();
        let date3 = Utc.with_ymd_and_hms(2024, 1, 30, 12, 0, 0).unwrap();

        for (date, gain) in [(date1, 1000), (date2, 2000), (date3, 3000)] {
            let pnl = RealizedPnL {
                id: 0,
                account_id: "test-acc".to_string(),
                asset: "BTC".to_string(),
                disposal_date: date,
                disposal_tx_id: None,
                quantity: Decimal::from_str("1.0").unwrap(),
                proceeds: Decimal::from_str(&(40000 + gain).to_string()).unwrap(),
                cost_basis: Decimal::from_str("40000").unwrap(),
                realized_gain: Decimal::from_str(&gain.to_string()).unwrap(),
                holding_period_days: None,
                tax_lot_id: None,
                cost_basis_method: CostBasisMethod::Fifo,
                created_at: Utc::now(),
            };
            repo.create(&pnl).await?;
        }

        // Test date range filtering
        let from = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2024, 1, 25, 0, 0, 0).unwrap();
        let results = repo.list_by_date_range(Some(from), Some(to)).await?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].realized_gain, Decimal::from_str("2000").unwrap());

        // Test from-only
        let results = repo.list_by_date_range(Some(from), None).await?;
        assert_eq!(results.len(), 2); // date2 and date3

        // Test to-only
        let results = repo.list_by_date_range(None, Some(to)).await?;
        assert_eq!(results.len(), 2); // date1 and date2

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_account() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = RealizedPnLRepository::new(&pool);

        // Create second account
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) VALUES ('test-acc-2', 'test-cat', 'Test Account 2', 'wallet')",
        )
        .execute(&pool)
        .await?;

        // Create P&L for both accounts
        let date = Utc::now();
        for (account, gain) in [("test-acc", 1000), ("test-acc-2", 2000)] {
            let pnl = RealizedPnL {
                id: 0,
                account_id: account.to_string(),
                asset: "BTC".to_string(),
                disposal_date: date,
                disposal_tx_id: None,
                quantity: Decimal::from_str("1.0").unwrap(),
                proceeds: Decimal::from_str(&(40000 + gain).to_string()).unwrap(),
                cost_basis: Decimal::from_str("40000").unwrap(),
                realized_gain: Decimal::from_str(&gain.to_string()).unwrap(),
                holding_period_days: None,
                tax_lot_id: None,
                cost_basis_method: CostBasisMethod::Fifo,
                created_at: Utc::now(),
            };
            repo.create(&pnl).await?;
        }

        // Test account filtering
        let results = repo.list_by_account("test-acc").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].account_id, "test-acc");
        assert_eq!(results[0].realized_gain, Decimal::from_str("1000").unwrap());

        let results = repo.list_by_account("test-acc-2").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].account_id, "test-acc-2");
        assert_eq!(results[0].realized_gain, Decimal::from_str("2000").unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_asset() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = RealizedPnLRepository::new(&pool);

        // Create P&L for different assets
        let date = Utc::now();
        for (asset, gain) in [("BTC", 1000), ("ETH", 2000), ("BTC", 3000)] {
            let pnl = RealizedPnL {
                id: 0,
                account_id: "test-acc".to_string(),
                asset: asset.to_string(),
                disposal_date: date,
                disposal_tx_id: None,
                quantity: Decimal::from_str("1.0").unwrap(),
                proceeds: Decimal::from_str(&(10000 + gain).to_string()).unwrap(),
                cost_basis: Decimal::from_str("10000").unwrap(),
                realized_gain: Decimal::from_str(&gain.to_string()).unwrap(),
                holding_period_days: None,
                tax_lot_id: None,
                cost_basis_method: CostBasisMethod::Fifo,
                created_at: Utc::now(),
            };
            repo.create(&pnl).await?;
        }

        // Test asset filtering (case-insensitive)
        let results = repo.list_by_asset("btc").await?;
        assert_eq!(results.len(), 2);

        let results = repo.list_by_asset("ETH").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].realized_gain, Decimal::from_str("2000").unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_filtered() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = RealizedPnLRepository::new(&pool);

        // Create second account
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) VALUES ('test-acc-2', 'test-cat', 'Test Account 2', 'wallet')",
        )
        .execute(&pool)
        .await?;

        // Create multiple P&L records
        let date1 = Utc.with_ymd_and_hms(2024, 1, 10, 12, 0, 0).unwrap();
        let date2 = Utc.with_ymd_and_hms(2024, 1, 20, 12, 0, 0).unwrap();

        for (account, asset, date) in [
            ("test-acc", "BTC", date1),
            ("test-acc", "ETH", date2),
            ("test-acc-2", "BTC", date1),
            ("test-acc-2", "BTC", date2),
        ] {
            let pnl = RealizedPnL {
                id: 0,
                account_id: account.to_string(),
                asset: asset.to_string(),
                disposal_date: date,
                disposal_tx_id: None,
                quantity: Decimal::from_str("1.0").unwrap(),
                proceeds: Decimal::from_str("50000").unwrap(),
                cost_basis: Decimal::from_str("40000").unwrap(),
                realized_gain: Decimal::from_str("10000").unwrap(),
                holding_period_days: None,
                tax_lot_id: None,
                cost_basis_method: CostBasisMethod::Fifo,
                created_at: Utc::now(),
            };
            repo.create(&pnl).await?;
        }

        // Test combined filters
        let results = repo
            .list_filtered(Some("test-acc"), Some("BTC"), None, None, None)
            .await?;
        assert_eq!(results.len(), 1);

        let from = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();
        let results = repo
            .list_filtered(None, Some("BTC"), Some(from), None, None)
            .await?;
        assert_eq!(results.len(), 1); // Only test-acc-2 BTC on date2

        // Test limit
        let results = repo.list_filtered(None, None, None, None, Some(2)).await?;
        assert_eq!(results.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_summary() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = RealizedPnLRepository::new(&pool);

        // Create P&L records with different gains
        let date = Utc::now();
        for gain in [1000, 2000, -500] {
            let pnl = RealizedPnL {
                id: 0,
                account_id: "test-acc".to_string(),
                asset: "BTC".to_string(),
                disposal_date: date,
                disposal_tx_id: None,
                quantity: Decimal::from_str("1.0").unwrap(),
                proceeds: Decimal::from_str(&(40000 + gain).to_string()).unwrap(),
                cost_basis: Decimal::from_str("40000").unwrap(),
                realized_gain: Decimal::from_str(&gain.to_string()).unwrap(),
                holding_period_days: None,
                tax_lot_id: None,
                cost_basis_method: CostBasisMethod::Fifo,
                created_at: Utc::now(),
            };
            repo.create(&pnl).await?;
        }

        // Test total summary
        let summary = repo.get_summary(None, None, None).await?;
        assert_eq!(summary, Decimal::from_str("2500").unwrap()); // 1000 + 2000 - 500

        Ok(())
    }

    #[tokio::test]
    async fn test_get_summary_with_filters() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = RealizedPnLRepository::new(&pool);

        // Create second account
        sqlx::query(
            "INSERT INTO accounts (id, category_id, name, account_type) VALUES ('test-acc-2', 'test-cat', 'Test Account 2', 'wallet')",
        )
        .execute(&pool)
        .await?;

        // Create P&L for different accounts and dates
        let date1 = Utc.with_ymd_and_hms(2024, 1, 10, 12, 0, 0).unwrap();
        let date2 = Utc.with_ymd_and_hms(2024, 1, 20, 12, 0, 0).unwrap();

        for (account, date, gain) in [
            ("test-acc", date1, 1000),
            ("test-acc", date2, 2000),
            ("test-acc-2", date1, 3000),
        ] {
            let pnl = RealizedPnL {
                id: 0,
                account_id: account.to_string(),
                asset: "BTC".to_string(),
                disposal_date: date,
                disposal_tx_id: None,
                quantity: Decimal::from_str("1.0").unwrap(),
                proceeds: Decimal::from_str(&(40000 + gain).to_string()).unwrap(),
                cost_basis: Decimal::from_str("40000").unwrap(),
                realized_gain: Decimal::from_str(&gain.to_string()).unwrap(),
                holding_period_days: None,
                tax_lot_id: None,
                cost_basis_method: CostBasisMethod::Fifo,
                created_at: Utc::now(),
            };
            repo.create(&pnl).await?;
        }

        // Test account filter
        let summary = repo.get_summary(Some("test-acc"), None, None).await?;
        assert_eq!(summary, Decimal::from_str("3000").unwrap()); // 1000 + 2000

        // Test date range filter
        let from = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();
        let summary = repo.get_summary(None, Some(from), None).await?;
        assert_eq!(summary, Decimal::from_str("2000").unwrap()); // Only date2

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_results() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = RealizedPnLRepository::new(&pool);

        // Test empty database
        let results = repo.list_by_account("test-acc").await?;
        assert_eq!(results.len(), 0);

        let summary = repo.get_summary(None, None, None).await?;
        assert_eq!(summary, Decimal::ZERO);

        Ok(())
    }

    #[tokio::test]
    async fn test_cost_basis_method_conversion() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = RealizedPnLRepository::new(&pool);

        // Test all cost basis methods
        for method in [
            CostBasisMethod::Fifo,
            CostBasisMethod::Lifo,
            CostBasisMethod::AverageCost,
        ] {
            let pnl = RealizedPnL {
                id: 0,
                account_id: "test-acc".to_string(),
                asset: "BTC".to_string(),
                disposal_date: Utc::now(),
                disposal_tx_id: None,
                quantity: Decimal::from_str("1.0").unwrap(),
                proceeds: Decimal::from_str("50000").unwrap(),
                cost_basis: Decimal::from_str("40000").unwrap(),
                realized_gain: Decimal::from_str("10000").unwrap(),
                holding_period_days: None,
                tax_lot_id: None,
                cost_basis_method: method.clone(),
                created_at: Utc::now(),
            };
            repo.create(&pnl).await?;
        }

        // Verify all methods were stored and retrieved correctly
        let results = repo.list_by_account("test-acc").await?;
        assert_eq!(results.len(), 3);

        // Check that we can retrieve all three methods
        let methods: Vec<CostBasisMethod> = results
            .iter()
            .map(|r| r.cost_basis_method.clone())
            .collect();
        assert!(methods.contains(&CostBasisMethod::Fifo));
        assert!(methods.contains(&CostBasisMethod::Lifo));
        assert!(methods.contains(&CostBasisMethod::AverageCost));

        Ok(())
    }
}
