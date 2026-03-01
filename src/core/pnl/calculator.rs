use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::core::pnl::{CostBasisMethod, DisposalMatch, RealizedPnL, TaxLot};
use crate::db::{RealizedPnLRepository, TaxLotRepository};
use crate::error::{CryptofolioError, Result};

/// P&L Calculator - handles tax lot creation and disposal matching
pub struct PnLCalculator<'a> {
    tax_lot_repo: TaxLotRepository<'a>,
    pnl_repo: RealizedPnLRepository<'a>,
}

impl<'a> PnLCalculator<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            tax_lot_repo: TaxLotRepository::new(pool),
            pnl_repo: RealizedPnLRepository::new(pool),
        }
    }

    /// Process an acquisition (buy) - creates a new tax lot
    pub async fn process_acquisition(
        &self,
        tx_id: i64,
        account_id: &str,
        asset: &str,
        quantity: Decimal,
        price_per_unit: Decimal,
        timestamp: DateTime<Utc>,
        method: CostBasisMethod,
    ) -> Result<i64> {
        let tax_lot = TaxLot {
            id: 0, // Will be assigned by database
            account_id: account_id.to_string(),
            asset: asset.to_uppercase(),
            quantity,
            remaining_quantity: quantity,
            acquisition_price: price_per_unit,
            acquisition_date: timestamp,
            acquisition_tx_id: Some(tx_id),
            cost_basis_method: method,
            fully_disposed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.tax_lot_repo.create(&tax_lot).await
    }

    /// Process a disposal (sell) - matches against tax lots and records realized P&L
    pub async fn process_disposal(
        &self,
        tx_id: i64,
        account_id: &str,
        asset: &str,
        quantity: Decimal,
        proceeds_per_unit: Decimal,
        timestamp: DateTime<Utc>,
        method: CostBasisMethod,
    ) -> Result<Vec<RealizedPnL>> {
        // Match the disposal to tax lots using FIFO/LIFO
        let matches = self
            .match_disposal(account_id, asset, quantity, method)
            .await?;

        // Create realized P&L records for each match
        let mut realized_pnls = Vec::new();

        for disposal_match in matches {
            let proceeds = disposal_match.quantity * proceeds_per_unit;
            let realized_gain = proceeds - disposal_match.cost_basis;

            // Calculate holding period in days
            let holding_period_days = (timestamp - disposal_match.acquisition_date).num_days();

            let pnl = RealizedPnL {
                id: 0, // Will be assigned by database
                account_id: account_id.to_string(),
                asset: asset.to_uppercase(),
                disposal_date: timestamp,
                disposal_tx_id: Some(tx_id),
                quantity: disposal_match.quantity,
                proceeds,
                cost_basis: disposal_match.cost_basis,
                realized_gain,
                holding_period_days: Some(holding_period_days),
                tax_lot_id: Some(disposal_match.tax_lot_id),
                cost_basis_method: method,
                created_at: Utc::now(),
            };

            let pnl_id = self.pnl_repo.create(&pnl).await?;

            // Add the ID to our result
            let mut pnl_with_id = pnl;
            pnl_with_id.id = pnl_id;
            realized_pnls.push(pnl_with_id);
        }

        Ok(realized_pnls)
    }

    /// Internal: Match a disposal to tax lots using FIFO/LIFO
    async fn match_disposal(
        &self,
        account_id: &str,
        asset: &str,
        quantity: Decimal,
        method: CostBasisMethod,
    ) -> Result<Vec<DisposalMatch>> {
        // Get available lots ordered by the cost basis method
        let lots = self
            .tax_lot_repo
            .get_available_lots(account_id, asset, method)
            .await?;

        // IMPORTANT: Validate we have enough quantity BEFORE updating any lots
        let total_available: Decimal = lots.iter().map(|lot| lot.remaining_quantity).sum();
        if quantity > total_available {
            return Err(CryptofolioError::InsufficientTaxLots {
                asset: asset.to_string(),
                required: quantity,
                available: total_available,
            });
        }

        let mut matches = Vec::new();
        let mut remaining = quantity;

        for lot in &lots {
            if remaining <= Decimal::ZERO {
                break;
            }

            // Match as much as possible from this lot
            let match_qty = remaining.min(lot.remaining_quantity);
            let match_cost = match_qty * lot.acquisition_price;

            matches.push(DisposalMatch {
                tax_lot_id: lot.id,
                quantity: match_qty,
                cost_basis: match_cost,
                acquisition_date: lot.acquisition_date,
            });

            // Update the lot's remaining quantity
            let new_remaining = lot.remaining_quantity - match_qty;
            self.tax_lot_repo
                .update_remaining(lot.id, new_remaining)
                .await?;

            // Mark as fully disposed if nothing left
            if new_remaining == Decimal::ZERO {
                self.tax_lot_repo.mark_disposed(lot.id).await?;
            }

            remaining -= match_qty;
        }

        Ok(matches)
    }

    /// Get total available quantity for an asset (sum of all tax lot remaining quantities)
    pub async fn get_available_quantity(
        &self,
        account_id: &str,
        asset: &str,
    ) -> Result<Decimal> {
        let lots = self
            .tax_lot_repo
            .list_by_account_asset(account_id, asset)
            .await?;

        let total: Decimal = lots
            .iter()
            .filter(|lot| !lot.fully_disposed)
            .map(|lot| lot.remaining_quantity)
            .sum();

        Ok(total)
    }

    /// Calculate unrealized P&L for a specific asset
    pub async fn calculate_unrealized_pnl(
        &self,
        account_id: &str,
        asset: &str,
        current_price: Decimal,
    ) -> Result<Decimal> {
        let lots = self
            .tax_lot_repo
            .list_by_account_asset(account_id, asset)
            .await?;

        let unrealized: Decimal = lots
            .iter()
            .filter(|lot| !lot.fully_disposed && lot.remaining_quantity > Decimal::ZERO)
            .map(|lot| {
                (current_price - lot.acquisition_price) * lot.remaining_quantity
            })
            .sum();

        Ok(unrealized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_memory_pool, AccountRepository};
    use crate::core::account::{Account, AccountType};
    use std::str::FromStr;

    /// Helper to create a Decimal from a string (for test readability)
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).expect("Invalid decimal in test")
    }

    /// Helper to create a test account in the database
    async fn create_test_account(pool: &sqlx::SqlitePool, account_id: &str) {
        let account_repo = AccountRepository::new(pool);

        // Create a test category if it doesn't exist
        if account_repo.get_category("test").await.unwrap().is_none() {
            account_repo.create_category("test", "Test Category").await.unwrap();
        }

        // Create the test account
        let account = Account {
            id: account_id.to_string(),
            name: format!("Test Account {}", account_id),
            account_type: AccountType::Exchange,
            category_id: "test".to_string(),
            config: crate::core::account::AccountConfig::default(),
            sync_enabled: false,
            created_at: Utc::now(),
        };
        account_repo.create_account(&account).await.unwrap();
    }

    /// Helper to create a minimal transaction record (to satisfy foreign key constraints)
    async fn create_test_transaction(pool: &sqlx::SqlitePool, tx_id: i64, account_id: &str, asset: &str) {
        // Insert a minimal transaction record to satisfy foreign key constraints
        sqlx::query(
            r#"INSERT INTO transactions (id, timestamp, tx_type, to_account_id, to_asset, to_quantity)
               VALUES (?, CURRENT_TIMESTAMP, 'buy', ?, ?, '0')"#
        )
        .bind(tx_id)
        .bind(account_id)
        .bind(asset.to_uppercase())
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_fifo_matching() {
        // Setup: Create in-memory database and calculator
        let pool = init_memory_pool().await.unwrap();
        create_test_account(&pool, "test_account").await;
        let calculator = PnLCalculator::new(&pool);

        // SCENARIO: Buy BTC twice at different prices, then sell 1.5 BTC
        // Expected: FIFO should match the first purchase entirely, then half of the second

        // Transaction 1: Buy 1.0 BTC @ $40,000
        create_test_transaction(&pool, 1, "test_account", "BTC").await;
        calculator
            .process_acquisition(
                1,
                "test_account",
                "BTC",
                dec("1.0"),
                dec("40000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Transaction 2: Buy 1.0 BTC @ $50,000
        create_test_transaction(&pool, 2, "test_account", "BTC").await;
        calculator
            .process_acquisition(
                2,
                "test_account",
                "BTC",
                dec("1.0"),
                dec("50000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Transaction 3: Sell 1.5 BTC @ $60,000 (should match first lot + 0.5 of second lot)
        create_test_transaction(&pool, 3, "test_account", "BTC").await;
        let pnls = calculator
            .process_disposal(
                3,
                "test_account",
                "BTC",
                dec("1.5"),
                dec("60000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Verify we got 2 P&L records (matched 2 different lots)
        assert_eq!(pnls.len(), 2, "Should match against 2 different tax lots");

        // FIRST MATCH: 1.0 BTC from first lot (acquired @ $40k, sold @ $60k)
        let first_match = &pnls[0];
        assert_eq!(first_match.quantity, dec("1.0"), "First match quantity");
        assert_eq!(
            first_match.cost_basis,
            dec("40000"),
            "First match cost basis = 1.0 * $40,000"
        );
        assert_eq!(
            first_match.proceeds,
            dec("60000"),
            "First match proceeds = 1.0 * $60,000"
        );
        assert_eq!(
            first_match.realized_gain,
            dec("20000"),
            "First match gain = $60,000 - $40,000"
        );

        // SECOND MATCH: 0.5 BTC from second lot (acquired @ $50k, sold @ $60k)
        let second_match = &pnls[1];
        assert_eq!(second_match.quantity, dec("0.5"), "Second match quantity");
        assert_eq!(
            second_match.cost_basis,
            dec("25000"),
            "Second match cost basis = 0.5 * $50,000"
        );
        assert_eq!(
            second_match.proceeds,
            dec("30000"),
            "Second match proceeds = 0.5 * $60,000"
        );
        assert_eq!(
            second_match.realized_gain,
            dec("5000"),
            "Second match gain = $30,000 - $25,000"
        );

        // TOTAL: Verify combined realized gain
        let total_gain: Decimal = pnls.iter().map(|p| p.realized_gain).sum();
        assert_eq!(
            total_gain,
            dec("25000"),
            "Total realized gain = $20,000 + $5,000"
        );

        // VERIFY: Check remaining tax lot quantities
        let available = calculator
            .get_available_quantity("test_account", "BTC")
            .await
            .unwrap();
        assert_eq!(
            available,
            dec("0.5"),
            "Should have 0.5 BTC remaining (2.0 bought - 1.5 sold)"
        );
    }

    #[tokio::test]
    async fn test_lifo_matching() {
        // Setup: Create in-memory database and calculator
        let pool = init_memory_pool().await.unwrap();
        create_test_account(&pool, "test_account").await;
        let calculator = PnLCalculator::new(&pool);

        // SCENARIO: Buy BTC twice at different prices, then sell 1.5 BTC using LIFO
        // Expected: LIFO should match the SECOND purchase first, then half of the first

        // Transaction 1: Buy 1.0 BTC @ $40,000
        create_test_transaction(&pool, 1, "test_account", "BTC").await;
        calculator
            .process_acquisition(
                1,
                "test_account",
                "BTC",
                dec("1.0"),
                dec("40000"),
                Utc::now(),
                CostBasisMethod::Lifo,
            )
            .await
            .unwrap();

        // Transaction 2: Buy 1.0 BTC @ $50,000 (more recent, will be matched first in LIFO)
        create_test_transaction(&pool, 2, "test_account", "BTC").await;
        calculator
            .process_acquisition(
                2,
                "test_account",
                "BTC",
                dec("1.0"),
                dec("50000"),
                Utc::now(),
                CostBasisMethod::Lifo,
            )
            .await
            .unwrap();

        // Transaction 3: Sell 1.5 BTC @ $60,000 (LIFO: match second lot first)
        create_test_transaction(&pool, 3, "test_account", "BTC").await;
        let pnls = calculator
            .process_disposal(
                3,
                "test_account",
                "BTC",
                dec("1.5"),
                dec("60000"),
                Utc::now(),
                CostBasisMethod::Lifo,
            )
            .await
            .unwrap();

        // Verify we got 2 P&L records
        assert_eq!(pnls.len(), 2, "Should match against 2 different tax lots");

        // FIRST MATCH (LIFO): 1.0 BTC from SECOND lot (acquired @ $50k, sold @ $60k)
        let first_match = &pnls[0];
        assert_eq!(
            first_match.quantity,
            dec("1.0"),
            "First match quantity (from second lot)"
        );
        assert_eq!(
            first_match.cost_basis,
            dec("50000"),
            "First match cost basis = 1.0 * $50,000"
        );
        assert_eq!(
            first_match.proceeds,
            dec("60000"),
            "First match proceeds = 1.0 * $60,000"
        );
        assert_eq!(
            first_match.realized_gain,
            dec("10000"),
            "First match gain = $60,000 - $50,000"
        );

        // SECOND MATCH (LIFO): 0.5 BTC from FIRST lot (acquired @ $40k, sold @ $60k)
        let second_match = &pnls[1];
        assert_eq!(
            second_match.quantity,
            dec("0.5"),
            "Second match quantity (from first lot)"
        );
        assert_eq!(
            second_match.cost_basis,
            dec("20000"),
            "Second match cost basis = 0.5 * $40,000"
        );
        assert_eq!(
            second_match.proceeds,
            dec("30000"),
            "Second match proceeds = 0.5 * $60,000"
        );
        assert_eq!(
            second_match.realized_gain,
            dec("10000"),
            "Second match gain = $30,000 - $20,000"
        );

        // TOTAL: Verify combined realized gain (different from FIFO!)
        let total_gain: Decimal = pnls.iter().map(|p| p.realized_gain).sum();
        assert_eq!(
            total_gain,
            dec("20000"),
            "Total realized gain = $10,000 + $10,000 (different from FIFO's $25,000)"
        );

        // VERIFY: Check remaining tax lot quantities
        let available = calculator
            .get_available_quantity("test_account", "BTC")
            .await
            .unwrap();
        assert_eq!(
            available,
            dec("0.5"),
            "Should have 0.5 BTC remaining (2.0 bought - 1.5 sold)"
        );
    }

    #[tokio::test]
    async fn test_insufficient_tax_lots() {
        // Setup: Create in-memory database and calculator
        let pool = init_memory_pool().await.unwrap();
        create_test_account(&pool, "test_account").await;
        let calculator = PnLCalculator::new(&pool);

        // SCENARIO: Try to sell more BTC than we own
        // Expected: Should return InsufficientTaxLots error

        // Transaction 1: Buy 1.0 BTC @ $40,000
        create_test_transaction(&pool, 1, "test_account", "BTC").await;
        calculator
            .process_acquisition(
                1,
                "test_account",
                "BTC",
                dec("1.0"),
                dec("40000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Transaction 2: Try to sell 2.0 BTC (should fail - only have 1.0)
        create_test_transaction(&pool, 2, "test_account", "BTC").await;
        let result = calculator
            .process_disposal(
                2,
                "test_account",
                "BTC",
                dec("2.0"),
                dec("60000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await;

        // Verify error
        assert!(result.is_err(), "Should fail when selling more than available");

        match result.unwrap_err() {
            CryptofolioError::InsufficientTaxLots {
                asset,
                required,
                available,
            } => {
                assert_eq!(asset, "BTC", "Error should specify BTC");
                assert_eq!(required, dec("2.0"), "Error should show required amount");
                assert_eq!(available, dec("1.0"), "Error should show available amount");
            }
            other => panic!("Expected InsufficientTaxLots error, got: {:?}", other),
        }

        // VERIFY: Original 1.0 BTC should still be available (transaction should have rolled back)
        let available = calculator
            .get_available_quantity("test_account", "BTC")
            .await
            .unwrap();
        assert_eq!(
            available,
            dec("1.0"),
            "Original purchase should still be available"
        );
    }

    #[tokio::test]
    async fn test_partial_lot_disposal() {
        // Setup: Create in-memory database and calculator
        let pool = init_memory_pool().await.unwrap();
        create_test_account(&pool, "test_account").await;
        let calculator = PnLCalculator::new(&pool);

        // SCENARIO: Sell a small portion of a lot, verify the remainder is tracked
        // Expected: First sale should consume part of the lot, second sale should consume the rest

        // Transaction 1: Buy 1.0 BTC @ $40,000
        create_test_transaction(&pool, 1, "test_account", "BTC").await;
        calculator
            .process_acquisition(
                1,
                "test_account",
                "BTC",
                dec("1.0"),
                dec("40000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Transaction 2: Sell 0.3 BTC @ $50,000
        create_test_transaction(&pool, 2, "test_account", "BTC").await;
        let pnls1 = calculator
            .process_disposal(
                2,
                "test_account",
                "BTC",
                dec("0.3"),
                dec("50000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        assert_eq!(pnls1.len(), 1, "Should match 1 lot");
        assert_eq!(pnls1[0].quantity, dec("0.3"));
        assert_eq!(pnls1[0].cost_basis, dec("12000")); // 0.3 * 40000
        assert_eq!(pnls1[0].proceeds, dec("15000")); // 0.3 * 50000
        assert_eq!(pnls1[0].realized_gain, dec("3000")); // 15000 - 12000

        // Verify 0.7 BTC remaining
        let available_after_first = calculator
            .get_available_quantity("test_account", "BTC")
            .await
            .unwrap();
        assert_eq!(available_after_first, dec("0.7"), "Should have 0.7 BTC left");

        // Transaction 3: Sell remaining 0.7 BTC @ $55,000
        create_test_transaction(&pool, 3, "test_account", "BTC").await;
        let pnls2 = calculator
            .process_disposal(
                3,
                "test_account",
                "BTC",
                dec("0.7"),
                dec("55000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        assert_eq!(pnls2.len(), 1, "Should match the same lot");
        assert_eq!(pnls2[0].quantity, dec("0.7"));
        assert_eq!(pnls2[0].cost_basis, dec("28000")); // 0.7 * 40000
        assert_eq!(pnls2[0].proceeds, dec("38500")); // 0.7 * 55000
        assert_eq!(pnls2[0].realized_gain, dec("10500")); // 38500 - 28000

        // Verify no BTC remaining
        let available_final = calculator
            .get_available_quantity("test_account", "BTC")
            .await
            .unwrap();
        assert_eq!(available_final, dec("0.0"), "Should have 0 BTC left");
    }

    #[tokio::test]
    async fn test_unrealized_pnl_calculation() {
        // Setup: Create in-memory database and calculator
        let pool = init_memory_pool().await.unwrap();
        create_test_account(&pool, "test_account").await;
        let calculator = PnLCalculator::new(&pool);

        // SCENARIO: Buy BTC at two different prices, calculate unrealized P&L at current price
        // Expected: Unrealized P&L = (current_price - acquisition_price) * remaining_quantity

        // Transaction 1: Buy 1.0 BTC @ $40,000
        create_test_transaction(&pool, 1, "test_account", "BTC").await;
        calculator
            .process_acquisition(
                1,
                "test_account",
                "BTC",
                dec("1.0"),
                dec("40000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Transaction 2: Buy 0.5 BTC @ $50,000
        create_test_transaction(&pool, 2, "test_account", "BTC").await;
        calculator
            .process_acquisition(
                2,
                "test_account",
                "BTC",
                dec("0.5"),
                dec("50000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // TEST CASE 1: Price went UP to $60,000 (profit)
        let unrealized_profit = calculator
            .calculate_unrealized_pnl("test_account", "BTC", dec("60000"))
            .await
            .unwrap();

        // Expected unrealized P&L:
        // Lot 1: (60000 - 40000) * 1.0 = 20000
        // Lot 2: (60000 - 50000) * 0.5 = 5000
        // Total: 25000
        assert_eq!(
            unrealized_profit,
            dec("25000"),
            "Unrealized profit when price rises to $60k"
        );

        // TEST CASE 2: Price went DOWN to $35,000 (loss)
        let unrealized_loss = calculator
            .calculate_unrealized_pnl("test_account", "BTC", dec("35000"))
            .await
            .unwrap();

        // Expected unrealized P&L:
        // Lot 1: (35000 - 40000) * 1.0 = -5000
        // Lot 2: (35000 - 50000) * 0.5 = -7500
        // Total: -12500
        assert_eq!(
            unrealized_loss,
            dec("-12500"),
            "Unrealized loss when price drops to $35k"
        );

        // TEST CASE 3: Price at breakeven (weighted average cost)
        // Weighted avg: (40000 * 1.0 + 50000 * 0.5) / 1.5 = 43333.33...
        // At exactly $43,333.33, unrealized should be close to zero
        let unrealized_breakeven = calculator
            .calculate_unrealized_pnl("test_account", "BTC", dec("43333.33"))
            .await
            .unwrap();

        // Allow small rounding difference
        assert!(
            unrealized_breakeven.abs() < dec("1.0"),
            "Unrealized P&L should be near zero at weighted average cost, got: {}",
            unrealized_breakeven
        );

        // TEST CASE 4: After partial disposal, verify unrealized P&L only on remaining holdings
        // Sell 0.8 BTC @ $55,000 (will consume first lot partially)
        create_test_transaction(&pool, 3, "test_account", "BTC").await;
        calculator
            .process_disposal(
                3,
                "test_account",
                "BTC",
                dec("0.8"),
                dec("55000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Remaining holdings:
        // Lot 1: 0.2 BTC @ $40,000 (1.0 - 0.8 sold)
        // Lot 2: 0.5 BTC @ $50,000 (untouched)

        let unrealized_after_sale = calculator
            .calculate_unrealized_pnl("test_account", "BTC", dec("60000"))
            .await
            .unwrap();

        // Expected unrealized P&L on remaining:
        // Lot 1: (60000 - 40000) * 0.2 = 4000
        // Lot 2: (60000 - 50000) * 0.5 = 5000
        // Total: 9000
        assert_eq!(
            unrealized_after_sale,
            dec("9000"),
            "Unrealized P&L should only apply to remaining holdings"
        );
    }

    #[tokio::test]
    async fn test_multiple_accounts_isolated() {
        // Setup: Create in-memory database and calculator
        let pool = init_memory_pool().await.unwrap();
        create_test_account(&pool, "account_a").await;
        create_test_account(&pool, "account_b").await;
        let calculator = PnLCalculator::new(&pool);

        // SCENARIO: Verify that tax lots are isolated per account
        // Expected: Operations on account A shouldn't affect account B

        // Account A: Buy 1.0 BTC @ $40,000
        create_test_transaction(&pool, 1, "account_a", "BTC").await;
        calculator
            .process_acquisition(
                1,
                "account_a",
                "BTC",
                dec("1.0"),
                dec("40000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Account B: Buy 1.0 BTC @ $50,000
        create_test_transaction(&pool, 2, "account_b", "BTC").await;
        calculator
            .process_acquisition(
                2,
                "account_b",
                "BTC",
                dec("1.0"),
                dec("50000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Verify each account has correct balance
        let balance_a = calculator
            .get_available_quantity("account_a", "BTC")
            .await
            .unwrap();
        let balance_b = calculator
            .get_available_quantity("account_b", "BTC")
            .await
            .unwrap();

        assert_eq!(balance_a, dec("1.0"), "Account A should have 1.0 BTC");
        assert_eq!(balance_b, dec("1.0"), "Account B should have 1.0 BTC");

        // Sell from Account A
        create_test_transaction(&pool, 3, "account_a", "BTC").await;
        let pnls_a = calculator
            .process_disposal(
                3,
                "account_a",
                "BTC",
                dec("0.5"),
                dec("60000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Verify Account A's P&L uses its own cost basis ($40k)
        assert_eq!(pnls_a[0].cost_basis, dec("20000")); // 0.5 * 40000
        assert_eq!(pnls_a[0].realized_gain, dec("10000")); // (60000 - 40000) * 0.5

        // Verify Account B is unaffected
        let balance_b_after = calculator
            .get_available_quantity("account_b", "BTC")
            .await
            .unwrap();
        assert_eq!(
            balance_b_after,
            dec("1.0"),
            "Account B should still have 1.0 BTC"
        );

        // Sell from Account B
        create_test_transaction(&pool, 4, "account_b", "BTC").await;
        let pnls_b = calculator
            .process_disposal(
                4,
                "account_b",
                "BTC",
                dec("0.5"),
                dec("60000"),
                Utc::now(),
                CostBasisMethod::Fifo,
            )
            .await
            .unwrap();

        // Verify Account B's P&L uses its own cost basis ($50k)
        assert_eq!(pnls_b[0].cost_basis, dec("25000")); // 0.5 * 50000
        assert_eq!(pnls_b[0].realized_gain, dec("5000")); // (60000 - 50000) * 0.5

        // Different accounts should have different P&L even at same sale price
        assert_ne!(
            pnls_a[0].realized_gain,
            pnls_b[0].realized_gain,
            "Different accounts should have different gains"
        );
    }
}
