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

        let mut q = sqlx::query_as::<_, (
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
        )>(&query);

        for binding in bindings {
            q = q.bind(binding);
        }

        let rows = q.fetch_all(self.pool).await?;

        self.parse_realized_pnls(rows)
    }

    /// List realized P&L by account
    pub async fn list_by_account(&self, account_id: &str) -> Result<Vec<RealizedPnL>> {
        let rows = sqlx::query_as::<_, (
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
        )>(
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
        let rows = sqlx::query_as::<_, (
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
        )>(
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
            query.push_str(&format!(" LIMIT {}", lim));
        }

        let mut q = sqlx::query_as::<_, (
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
        )>(&query);

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
        let quantity = Decimal::from_str(&quantity)
            .map_err(|_| CryptofolioError::InvalidAmount(quantity))?;

        let proceeds = Decimal::from_str(&proceeds)
            .map_err(|_| CryptofolioError::InvalidAmount(proceeds))?;

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
