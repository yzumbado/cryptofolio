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
            "#,
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
                    cost_basis_currency: Some("USD".to_string()),
                    avg_cost_basis_base: cost_per_unit,
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
            cost_basis_currency: Some("USD".to_string()),
            avg_cost_basis_base: cost_per_unit,
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
        (id, account_id, asset, quantity, avg_cost_basis, updated_at): (
            i64,
            String,
            String,
            String,
            Option<String>,
            String,
        ),
    ) -> Result<Holding> {
        let quantity =
            Decimal::from_str(&quantity).map_err(|_| CryptofolioError::InvalidAmount(quantity))?;

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
            cost_basis_currency: None, // TODO: Load from database
            avg_cost_basis_base: None, // TODO: Load from database
            updated_at: DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

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
    async fn test_add_quantity_new_holding() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity(
            "test-acc-1",
            "BTC",
            Decimal::from_str("1.5").unwrap(),
            Some(Decimal::from_str("40000").unwrap()),
        )
        .await?;

        let holding = repo.get("test-acc-1", "BTC").await?;
        assert!(holding.is_some());
        let h = holding.unwrap();
        assert_eq!(h.asset, "BTC");
        assert_eq!(h.quantity, Decimal::from_str("1.5").unwrap());
        assert_eq!(h.avg_cost_basis, Some(Decimal::from_str("40000").unwrap()));

        Ok(())
    }

    #[tokio::test]
    async fn test_add_quantity_existing_holding() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        // Add first batch
        repo.add_quantity(
            "test-acc-1",
            "BTC",
            Decimal::from_str("1.0").unwrap(),
            Some(Decimal::from_str("40000").unwrap()),
        )
        .await?;

        // Add second batch
        repo.add_quantity(
            "test-acc-1",
            "BTC",
            Decimal::from_str("1.0").unwrap(),
            Some(Decimal::from_str("50000").unwrap()),
        )
        .await?;

        let holding = repo.get("test-acc-1", "BTC").await?;
        assert!(holding.is_some());
        let h = holding.unwrap();
        assert_eq!(h.quantity, Decimal::from_str("2.0").unwrap());
        // Average cost basis: (1.0 * 40000 + 1.0 * 50000) / 2.0 = 45000
        assert_eq!(h.avg_cost_basis, Some(Decimal::from_str("45000").unwrap()));

        Ok(())
    }

    #[tokio::test]
    async fn test_add_quantity_weighted_average() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        // Add 1 BTC @ $40k
        repo.add_quantity(
            "test-acc-1",
            "BTC",
            Decimal::from_str("1.0").unwrap(),
            Some(Decimal::from_str("40000").unwrap()),
        )
        .await?;

        // Add 3 BTC @ $50k
        repo.add_quantity(
            "test-acc-1",
            "BTC",
            Decimal::from_str("3.0").unwrap(),
            Some(Decimal::from_str("50000").unwrap()),
        )
        .await?;

        let holding = repo.get("test-acc-1", "BTC").await?;
        let h = holding.unwrap();
        assert_eq!(h.quantity, Decimal::from_str("4.0").unwrap());
        // Weighted average: (1 * 40000 + 3 * 50000) / 4 = 47500
        assert_eq!(h.avg_cost_basis, Some(Decimal::from_str("47500").unwrap()));

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_quantity() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity(
            "test-acc-1",
            "BTC",
            Decimal::from_str("2.0").unwrap(),
            Some(Decimal::from_str("40000").unwrap()),
        )
        .await?;

        repo.remove_quantity("test-acc-1", "BTC", Decimal::from_str("0.5").unwrap())
            .await?;

        let holding = repo.get("test-acc-1", "BTC").await?;
        assert!(holding.is_some());
        assert_eq!(holding.unwrap().quantity, Decimal::from_str("1.5").unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_quantity_auto_delete_when_zero() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity(
            "test-acc-1",
            "BTC",
            Decimal::from_str("1.0").unwrap(),
            Some(Decimal::from_str("40000").unwrap()),
        )
        .await?;

        // Remove all
        repo.remove_quantity("test-acc-1", "BTC", Decimal::from_str("1.0").unwrap())
            .await?;

        // Should be deleted
        let holding = repo.get("test-acc-1", "BTC").await?;
        assert!(holding.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_quantity_insufficient_balance() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity("test-acc-1", "BTC", Decimal::from_str("1.0").unwrap(), None)
            .await?;

        let result = repo
            .remove_quantity("test-acc-1", "BTC", Decimal::from_str("2.0").unwrap())
            .await;
        assert!(result.is_err());
        match result {
            Err(CryptofolioError::InsufficientBalance { .. }) => {}
            _ => panic!("Expected InsufficientBalance error"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_quantity_asset_not_found() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        let result = repo
            .remove_quantity("test-acc-1", "BTC", Decimal::from_str("1.0").unwrap())
            .await;
        assert!(result.is_err());
        match result {
            Err(CryptofolioError::AssetNotFound(_)) => {}
            _ => panic!("Expected AssetNotFound error"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_set_quantity() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        // Set initial quantity
        repo.set_quantity(
            "test-acc-1",
            "ETH",
            Decimal::from_str("10.0").unwrap(),
            Some(Decimal::from_str("3000").unwrap()),
        )
        .await?;

        let holding = repo.get("test-acc-1", "ETH").await?;
        assert!(holding.is_some());
        assert_eq!(
            holding.unwrap().quantity,
            Decimal::from_str("10.0").unwrap()
        );

        // Update quantity (replaces, doesn't add)
        repo.set_quantity(
            "test-acc-1",
            "ETH",
            Decimal::from_str("5.0").unwrap(),
            Some(Decimal::from_str("3500").unwrap()),
        )
        .await?;

        let holding = repo.get("test-acc-1", "ETH").await?;
        assert!(holding.is_some());
        let h = holding.unwrap();
        assert_eq!(h.quantity, Decimal::from_str("5.0").unwrap());
        assert_eq!(h.avg_cost_basis, Some(Decimal::from_str("3500").unwrap()));

        Ok(())
    }

    #[tokio::test]
    async fn test_set_quantity_zero_deletes() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity("test-acc-1", "BTC", Decimal::from_str("1.0").unwrap(), None)
            .await?;

        // Set to zero should delete
        repo.set_quantity("test-acc-1", "BTC", Decimal::ZERO, None)
            .await?;

        let holding = repo.get("test-acc-1", "BTC").await?;
        assert!(holding.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_all() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity("test-acc-1", "BTC", Decimal::from_str("1.0").unwrap(), None)
            .await?;
        repo.add_quantity(
            "test-acc-1",
            "ETH",
            Decimal::from_str("10.0").unwrap(),
            None,
        )
        .await?;
        repo.add_quantity("test-acc-2", "BTC", Decimal::from_str("0.5").unwrap(), None)
            .await?;

        let holdings = repo.list_all().await?;
        assert_eq!(holdings.len(), 3);

        // Should be ordered by asset
        assert_eq!(holdings[0].asset, "BTC");
        assert_eq!(holdings[1].asset, "BTC");
        assert_eq!(holdings[2].asset, "ETH");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_account() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity("test-acc-1", "BTC", Decimal::from_str("1.0").unwrap(), None)
            .await?;
        repo.add_quantity(
            "test-acc-1",
            "ETH",
            Decimal::from_str("10.0").unwrap(),
            None,
        )
        .await?;
        repo.add_quantity("test-acc-2", "BTC", Decimal::from_str("0.5").unwrap(), None)
            .await?;

        let holdings = repo.list_by_account("test-acc-1").await?;
        assert_eq!(holdings.len(), 2);

        let holdings = repo.list_by_account("test-acc-2").await?;
        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].asset, "BTC");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_case_insensitive() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity("test-acc-1", "btc", Decimal::from_str("1.0").unwrap(), None)
            .await?;

        // Should find with different case
        let holding = repo.get("test-acc-1", "BTC").await?;
        assert!(holding.is_some());

        let holding = repo.get("test-acc-1", "btc").await?;
        assert!(holding.is_some());

        let holding = repo.get("test-acc-1", "Btc").await?;
        assert!(holding.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity("test-acc-1", "BTC", Decimal::from_str("1.0").unwrap(), None)
            .await?;

        repo.delete("test-acc-1", "BTC").await?;

        let holding = repo.get("test-acc-1", "BTC").await?;
        assert!(holding.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_all_for_account() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        repo.add_quantity("test-acc-1", "BTC", Decimal::from_str("1.0").unwrap(), None)
            .await?;
        repo.add_quantity(
            "test-acc-1",
            "ETH",
            Decimal::from_str("10.0").unwrap(),
            None,
        )
        .await?;
        repo.add_quantity("test-acc-2", "BTC", Decimal::from_str("0.5").unwrap(), None)
            .await?;

        repo.delete_all_for_account("test-acc-1").await?;

        let holdings = repo.list_by_account("test-acc-1").await?;
        assert_eq!(holdings.len(), 0);

        let holdings = repo.list_by_account("test-acc-2").await?;
        assert_eq!(holdings.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_list() -> Result<()> {
        let pool = setup_test_db().await?;
        let repo = HoldingRepository::new(&pool);

        let holdings = repo.list_all().await?;
        assert_eq!(holdings.len(), 0);

        let holdings = repo.list_by_account("test-acc-1").await?;
        assert_eq!(holdings.len(), 0);

        let holding = repo.get("test-acc-1", "BTC").await?;
        assert!(holding.is_none());

        Ok(())
    }
}
