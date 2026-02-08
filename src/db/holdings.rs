use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::core::holdings::Holding;
use crate::error::{CryptofolioError, Result};

pub struct HoldingRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> HoldingRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_all(&self) -> Result<Vec<Holding>> {
        let rows = sqlx::query_as::<_, (i64, String, String, String, Option<String>, String)>(
            "SELECT id, account_id, asset, quantity, avg_cost_basis, updated_at FROM holdings ORDER BY asset"
        )
        .fetch_all(self.pool)
        .await?;

        self.parse_holdings(rows)
    }

    pub async fn list_by_account(&self, account_id: &str) -> Result<Vec<Holding>> {
        let rows = sqlx::query_as::<_, (i64, String, String, String, Option<String>, String)>(
            "SELECT id, account_id, asset, quantity, avg_cost_basis, updated_at FROM holdings WHERE account_id = ? ORDER BY asset"
        )
        .bind(account_id)
        .fetch_all(self.pool)
        .await?;

        self.parse_holdings(rows)
    }

    pub async fn get(&self, account_id: &str, asset: &str) -> Result<Option<Holding>> {
        let row = sqlx::query_as::<_, (i64, String, String, String, Option<String>, String)>(
            "SELECT id, account_id, asset, quantity, avg_cost_basis, updated_at FROM holdings WHERE account_id = ? AND UPPER(asset) = UPPER(?)"
        )
        .bind(account_id)
        .bind(asset)
        .fetch_optional(self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.parse_holding(r)?)),
            None => Ok(None),
        }
    }

    pub async fn upsert(&self, holding: &Holding) -> Result<()> {
        let quantity_str = holding.quantity.to_string();
        let cost_basis_str = holding.avg_cost_basis.map(|d| d.to_string());

        sqlx::query(
            r#"
            INSERT INTO holdings (account_id, asset, quantity, avg_cost_basis, updated_at)
            VALUES (?, UPPER(?), ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(account_id, asset) DO UPDATE SET
                quantity = excluded.quantity,
                avg_cost_basis = excluded.avg_cost_basis,
                updated_at = CURRENT_TIMESTAMP
            "#
        )
        .bind(&holding.account_id)
        .bind(&holding.asset)
        .bind(&quantity_str)
        .bind(&cost_basis_str)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_quantity(
        &self,
        account_id: &str,
        asset: &str,
        quantity: Decimal,
        cost_per_unit: Option<Decimal>,
    ) -> Result<()> {
        let existing = self.get(account_id, asset).await?;

        match existing {
            Some(mut holding) => {
                // Calculate new average cost basis if both old and new have cost basis
                if let (Some(old_cost), Some(new_cost)) = (holding.avg_cost_basis, cost_per_unit) {
                    let old_total = old_cost * holding.quantity;
                    let new_total = new_cost * quantity;
                    let total_quantity = holding.quantity + quantity;
                    if total_quantity > Decimal::ZERO {
                        holding.avg_cost_basis = Some((old_total + new_total) / total_quantity);
                    }
                } else if cost_per_unit.is_some() {
                    // If no old cost basis but new one provided, use new
                    holding.avg_cost_basis = cost_per_unit;
                }

                holding.quantity += quantity;
                self.upsert(&holding).await
            }
            None => {
                let holding = Holding {
                    id: 0,
                    account_id: account_id.to_string(),
                    asset: asset.to_uppercase(),
                    quantity,
                    avg_cost_basis: cost_per_unit,
                    updated_at: Utc::now(),
                };
                self.upsert(&holding).await
            }
        }
    }

    pub async fn remove_quantity(
        &self,
        account_id: &str,
        asset: &str,
        quantity: Decimal,
    ) -> Result<()> {
        let existing = self.get(account_id, asset).await?;

        match existing {
            Some(mut holding) => {
                if holding.quantity < quantity {
                    return Err(CryptofolioError::InsufficientBalance {
                        available: holding.quantity.to_string(),
                        required: quantity.to_string(),
                    });
                }

                holding.quantity -= quantity;

                if holding.quantity == Decimal::ZERO {
                    self.delete(account_id, asset).await
                } else {
                    self.upsert(&holding).await
                }
            }
            None => Err(CryptofolioError::AssetNotFound(asset.to_string())),
        }
    }

    pub async fn set_quantity(
        &self,
        account_id: &str,
        asset: &str,
        quantity: Decimal,
        cost_per_unit: Option<Decimal>,
    ) -> Result<()> {
        if quantity == Decimal::ZERO {
            // Delete if setting to zero
            return self.delete(account_id, asset).await;
        }

        let holding = Holding {
            id: 0,
            account_id: account_id.to_string(),
            asset: asset.to_uppercase(),
            quantity,
            avg_cost_basis: cost_per_unit,
            updated_at: Utc::now(),
        };
        self.upsert(&holding).await
    }

    pub async fn delete(&self, account_id: &str, asset: &str) -> Result<()> {
        sqlx::query("DELETE FROM holdings WHERE account_id = ? AND UPPER(asset) = UPPER(?)")
            .bind(account_id)
            .bind(asset)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_all_for_account(&self, account_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM holdings WHERE account_id = ?")
            .bind(account_id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    fn parse_holdings(
        &self,
        rows: Vec<(i64, String, String, String, Option<String>, String)>,
    ) -> Result<Vec<Holding>> {
        rows.into_iter().map(|r| self.parse_holding(r)).collect()
    }

    fn parse_holding(
        &self,
        (id, account_id, asset, quantity, avg_cost_basis, updated_at): (i64, String, String, String, Option<String>, String),
    ) -> Result<Holding> {
        let quantity = Decimal::from_str(&quantity)
            .map_err(|_| CryptofolioError::InvalidAmount(quantity))?;

        let avg_cost_basis = avg_cost_basis
            .map(|s| Decimal::from_str(&s))
            .transpose()
            .map_err(|_| CryptofolioError::InvalidAmount("cost basis".to_string()))?;

        Ok(Holding {
            id,
            account_id,
            asset,
            quantity,
            avg_cost_basis,
            updated_at: DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}
