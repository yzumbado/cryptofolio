// P&L Integration Tests
// Tests the complete profit and loss workflow from transactions to tax lots to realized P&L

use chrono::Utc;
use cryptofolio::core::account::{Account, AccountConfig, AccountType};
use cryptofolio::core::pnl::{CostBasisMethod, PnLCalculator};
use cryptofolio::core::transaction::Transaction;
use cryptofolio::db::{AccountRepository, RealizedPnLRepository, TaxLotRepository, TransactionRepository};
use cryptofolio::error::Result;
use rust_decimal::Decimal;
use std::str::FromStr;

mod common;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("Invalid decimal in test")
}

/// Test: Complete FIFO workflow - buy multiple lots, sell, verify P&L
#[tokio::test]
async fn test_fifo_buy_sell_workflow() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);
    let tax_lot_repo = TaxLotRepository::new(&pool);
    let pnl_repo = RealizedPnLRepository::new(&pool);
    let pnl_calc = PnLCalculator::new(&pool);

    // Create test account
    let account_id = "test_fifo_account";
    if account_repo.get_category("test").await?.is_none() {
        account_repo.create_category("test", "Test Category").await?;
    }
    let account = Account {
        id: account_id.to_string(),
        name: "Test Account".to_string(),
        account_type: AccountType::Exchange,
        category_id: "test".to_string(),
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };
    account_repo.create_account(&account).await?;

    // Transaction 1: Buy 1.0 BTC @ $40,000
    let tx1 = Transaction::new_buy(account_id, "BTC", dec("1.0"), dec("40000"), Utc::now());
    let tx1_id = tx_repo.insert(&tx1).await?;
    pnl_calc.process_acquisition(
        tx1_id,
        account_id,
        "BTC",
        dec("1.0"),
        dec("40000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Transaction 2: Buy 1.0 BTC @ $50,000
    let tx2 = Transaction::new_buy(account_id, "BTC", dec("1.0"), dec("50000"), Utc::now());
    let tx2_id = tx_repo.insert(&tx2).await?;
    pnl_calc.process_acquisition(
        tx2_id,
        account_id,
        "BTC",
        dec("1.0"),
        dec("50000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Verify 2 tax lots created
    let lots = tax_lot_repo.list_by_account_asset(account_id, "BTC").await?;
    assert_eq!(lots.len(), 2, "Should have 2 tax lots");
    assert_eq!(lots[0].quantity, dec("1.0"));
    assert_eq!(lots[1].quantity, dec("1.0"));

    // Transaction 3: Sell 1.5 BTC @ $60,000 (FIFO: should match first lot fully + half of second)
    let tx3 = Transaction::new_sell(account_id, "BTC", dec("1.5"), dec("60000"), Utc::now());
    let tx3_id = tx_repo.insert(&tx3).await?;
    let pnls = pnl_calc.process_disposal(
        tx3_id,
        account_id,
        "BTC",
        dec("1.5"),
        dec("60000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Verify P&L records
    assert_eq!(pnls.len(), 2, "Should create 2 P&L records (matching 2 lots)");

    // First match: 1.0 BTC from first lot @ $40k, sold @ $60k = $20k gain
    assert_eq!(pnls[0].quantity, dec("1.0"));
    assert_eq!(pnls[0].cost_basis, dec("40000"));
    assert_eq!(pnls[0].proceeds, dec("60000"));
    assert_eq!(pnls[0].realized_gain, dec("20000"));

    // Second match: 0.5 BTC from second lot @ $50k, sold @ $60k = $5k gain
    assert_eq!(pnls[1].quantity, dec("0.5"));
    assert_eq!(pnls[1].cost_basis, dec("25000"));
    assert_eq!(pnls[1].proceeds, dec("30000"));
    assert_eq!(pnls[1].realized_gain, dec("5000"));

    // Total realized gain = $25,000
    let total_gain: Decimal = pnls.iter().map(|p| p.realized_gain).sum();
    assert_eq!(total_gain, dec("25000"));

    // Verify remaining tax lots
    let lots_after = tax_lot_repo.list_by_account_asset(account_id, "BTC").await?;
    let remaining_qty: Decimal = lots_after.iter()
        .filter(|lot| !lot.fully_disposed)
        .map(|lot| lot.remaining_quantity)
        .sum();
    assert_eq!(remaining_qty, dec("0.5"), "Should have 0.5 BTC remaining");

    // Verify P&L summary
    let summary = pnl_repo.get_summary(Some(account_id), None, None).await?;
    assert_eq!(summary, dec("25000"), "Total realized P&L should be $25,000");

    Ok(())
}

/// Test: LIFO vs FIFO produces different results
#[tokio::test]
async fn test_lifo_vs_fifo_different_results() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);
    let pnl_calc = PnLCalculator::new(&pool);

    // Create account
    let account_id = "test_lifo_account";
    if account_repo.get_category("test").await?.is_none() {
        account_repo.create_category("test", "Test Category").await?;
    }
    let account = Account {
        id: account_id.to_string(),
        name: "Test Account".to_string(),
        account_type: AccountType::Exchange,
        category_id: "test".to_string(),
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };
    account_repo.create_account(&account).await?;

    // Buy 1.0 BTC @ $40,000
    let tx1 = Transaction::new_buy(account_id, "BTC", dec("1.0"), dec("40000"), Utc::now());
    let tx1_id = tx_repo.insert(&tx1).await?;
    pnl_calc.process_acquisition(
        tx1_id,
        account_id,
        "BTC",
        dec("1.0"),
        dec("40000"),
        Utc::now(),
        CostBasisMethod::Lifo, // Using LIFO
    ).await?;

    // Buy 1.0 BTC @ $50,000
    let tx2 = Transaction::new_buy(account_id, "BTC", dec("1.0"), dec("50000"), Utc::now());
    let tx2_id = tx_repo.insert(&tx2).await?;
    pnl_calc.process_acquisition(
        tx2_id,
        account_id,
        "BTC",
        dec("1.0"),
        dec("50000"),
        Utc::now(),
        CostBasisMethod::Lifo,
    ).await?;

    // Sell 1.5 BTC @ $60,000 (LIFO: should match SECOND lot first)
    let tx3 = Transaction::new_sell(account_id, "BTC", dec("1.5"), dec("60000"), Utc::now());
    let tx3_id = tx_repo.insert(&tx3).await?;
    let pnls_lifo = pnl_calc.process_disposal(
        tx3_id,
        account_id,
        "BTC",
        dec("1.5"),
        dec("60000"),
        Utc::now(),
        CostBasisMethod::Lifo,
    ).await?;

    // LIFO should match:
    // - 1.0 BTC from SECOND lot @ $50k = $10k gain
    // - 0.5 BTC from FIRST lot @ $40k = $10k gain
    // Total: $20k (different from FIFO's $25k!)
    let total_lifo: Decimal = pnls_lifo.iter().map(|p| p.realized_gain).sum();
    assert_eq!(total_lifo, dec("20000"), "LIFO should produce $20k gain");

    // Compare to FIFO: same scenario would produce $25k
    // This test proves FIFO and LIFO produce different results
    assert_ne!(total_lifo, dec("25000"), "LIFO and FIFO must produce different results");

    Ok(())
}

/// Test: Unrealized P&L calculation
#[tokio::test]
async fn test_unrealized_pnl_calculation() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);
    let pnl_calc = PnLCalculator::new(&pool);

    // Create account
    let account_id = "test_unrealized_account";
    if account_repo.get_category("test").await?.is_none() {
        account_repo.create_category("test", "Test Category").await?;
    }
    let account = Account {
        id: account_id.to_string(),
        name: "Test Account".to_string(),
        account_type: AccountType::Exchange,
        category_id: "test".to_string(),
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };
    account_repo.create_account(&account).await?;

    // Buy 1.0 BTC @ $40,000
    let tx1 = Transaction::new_buy(account_id, "BTC", dec("1.0"), dec("40000"), Utc::now());
    let tx1_id = tx_repo.insert(&tx1).await?;
    pnl_calc.process_acquisition(
        tx1_id,
        account_id,
        "BTC",
        dec("1.0"),
        dec("40000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Buy 0.5 BTC @ $50,000
    let tx2 = Transaction::new_buy(account_id, "BTC", dec("0.5"), dec("50000"), Utc::now());
    let tx2_id = tx_repo.insert(&tx2).await?;
    pnl_calc.process_acquisition(
        tx2_id,
        account_id,
        "BTC",
        dec("0.5"),
        dec("50000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Calculate unrealized P&L at $60,000/BTC
    // Expected: (60000 - 40000) * 1.0 + (60000 - 50000) * 0.5 = $25,000
    let unrealized = pnl_calc.calculate_unrealized_pnl(
        account_id,
        "BTC",
        dec("60000"),
    ).await?;

    assert_eq!(unrealized, dec("25000"), "Unrealized P&L should be $25,000");

    // Calculate at a loss ($35,000/BTC)
    // Expected: (35000 - 40000) * 1.0 + (35000 - 50000) * 0.5 = -$12,500
    let unrealized_loss = pnl_calc.calculate_unrealized_pnl(
        account_id,
        "BTC",
        dec("35000"),
    ).await?;

    assert_eq!(unrealized_loss, dec("-12500"), "Unrealized loss should be -$12,500");

    Ok(())
}

/// Test: Error handling - insufficient tax lots
#[tokio::test]
async fn test_insufficient_tax_lots_error() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);
    let pnl_calc = PnLCalculator::new(&pool);

    // Create account
    let account_id = "test_error_account";
    if account_repo.get_category("test").await?.is_none() {
        account_repo.create_category("test", "Test Category").await?;
    }
    let account = Account {
        id: account_id.to_string(),
        name: "Test Account".to_string(),
        account_type: AccountType::Exchange,
        category_id: "test".to_string(),
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };
    account_repo.create_account(&account).await?;

    // Buy only 1.0 BTC
    let tx1 = Transaction::new_buy(account_id, "BTC", dec("1.0"), dec("40000"), Utc::now());
    let tx1_id = tx_repo.insert(&tx1).await?;
    pnl_calc.process_acquisition(
        tx1_id,
        account_id,
        "BTC",
        dec("1.0"),
        dec("40000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Try to sell 2.0 BTC (should fail)
    let tx2 = Transaction::new_sell(account_id, "BTC", dec("2.0"), dec("60000"), Utc::now());
    let tx2_id = tx_repo.insert(&tx2).await?;
    let result = pnl_calc.process_disposal(
        tx2_id,
        account_id,
        "BTC",
        dec("2.0"),
        dec("60000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await;

    assert!(result.is_err(), "Should fail with insufficient tax lots");

    // Verify original tax lot is still available (not consumed by failed disposal)
    let available = pnl_calc.get_available_quantity(account_id, "BTC").await?;
    assert_eq!(available, dec("1.0"), "Original 1.0 BTC should still be available");

    Ok(())
}

/// Test: Multi-account isolation
#[tokio::test]
async fn test_multi_account_pnl_isolation() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);
    let pnl_repo = RealizedPnLRepository::new(&pool);
    let pnl_calc = PnLCalculator::new(&pool);

    // Create category
    if account_repo.get_category("test").await?.is_none() {
        account_repo.create_category("test", "Test Category").await?;
    }

    // Create two accounts
    for account_id in ["account_a", "account_b"] {
        let account = Account {
            id: account_id.to_string(),
            name: format!("Test {}", account_id),
            account_type: AccountType::Exchange,
            category_id: "test".to_string(),
            config: AccountConfig::default(),
            sync_enabled: false,
            created_at: Utc::now(),
        };
        account_repo.create_account(&account).await?;
    }

    // Account A: Buy 1 BTC @ $40k, sell @ $60k = $20k gain
    let tx_a1 = Transaction::new_buy("account_a", "BTC", dec("1.0"), dec("40000"), Utc::now());
    let tx_a1_id = tx_repo.insert(&tx_a1).await?;
    pnl_calc.process_acquisition(tx_a1_id, "account_a", "BTC", dec("1.0"), dec("40000"), Utc::now(), CostBasisMethod::Fifo).await?;

    let tx_a2 = Transaction::new_sell("account_a", "BTC", dec("1.0"), dec("60000"), Utc::now());
    let tx_a2_id = tx_repo.insert(&tx_a2).await?;
    pnl_calc.process_disposal(tx_a2_id, "account_a", "BTC", dec("1.0"), dec("60000"), Utc::now(), CostBasisMethod::Fifo).await?;

    // Account B: Buy 1 BTC @ $50k, sell @ $60k = $10k gain
    let tx_b1 = Transaction::new_buy("account_b", "BTC", dec("1.0"), dec("50000"), Utc::now());
    let tx_b1_id = tx_repo.insert(&tx_b1).await?;
    pnl_calc.process_acquisition(tx_b1_id, "account_b", "BTC", dec("1.0"), dec("50000"), Utc::now(), CostBasisMethod::Fifo).await?;

    let tx_b2 = Transaction::new_sell("account_b", "BTC", dec("1.0"), dec("60000"), Utc::now());
    let tx_b2_id = tx_repo.insert(&tx_b2).await?;
    pnl_calc.process_disposal(tx_b2_id, "account_b", "BTC", dec("1.0"), dec("60000"), Utc::now(), CostBasisMethod::Fifo).await?;

    // Verify account A's P&L
    let pnl_a = pnl_repo.get_summary(Some("account_a"), None, None).await?;
    assert_eq!(pnl_a, dec("20000"), "Account A should have $20k realized gain");

    // Verify account B's P&L
    let pnl_b = pnl_repo.get_summary(Some("account_b"), None, None).await?;
    assert_eq!(pnl_b, dec("10000"), "Account B should have $10k realized gain");

    // Verify accounts are isolated
    assert_ne!(pnl_a, pnl_b, "Different accounts should have different P&L");

    Ok(())
}
