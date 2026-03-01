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
