// Transaction Workflow Integration Tests
// Tests the complete transaction lifecycle and interactions with holdings and P&L

use chrono::Utc;
use cryptofolio::core::account::{Account, AccountConfig, AccountType};
use cryptofolio::core::pnl::{CostBasisMethod, PnLCalculator};
use cryptofolio::core::transaction::{Transaction, TransactionType};
use cryptofolio::db::{AccountRepository, HoldingRepository, TransactionRepository};
use cryptofolio::error::Result;
use rust_decimal::Decimal;
use std::str::FromStr;

mod common;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("Invalid decimal in test")
}

/// Test: Buy transaction creates holding and tax lot
#[tokio::test]
async fn test_buy_creates_holding_and_tax_lot() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);
    let pnl_calc = PnLCalculator::new(&pool);

    // Create account
    let account_id = "buy_test_account";
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

    // Initially no holdings
    let holdings_before = holding_repo.list_by_account(account_id).await?;
    assert_eq!(holdings_before.len(), 0, "Should have no holdings initially");

    // Buy 1.5 BTC @ $45,000
    let tx = Transaction::new_buy(account_id, "BTC", dec("1.5"), dec("45000"), Utc::now());
    let tx_id = tx_repo.insert(&tx).await?;

    // Create holding
    holding_repo.add_quantity(account_id, "BTC", dec("1.5"), Some(dec("45000"))).await?;

    // Create tax lot
    pnl_calc.process_acquisition(
        tx_id,
        account_id,
        "BTC",
        dec("1.5"),
        dec("45000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Verify holding created
    let holdings_after = holding_repo.list_by_account(account_id).await?;
    assert_eq!(holdings_after.len(), 1, "Should have 1 holding");
    assert_eq!(holdings_after[0].asset, "BTC");
    assert_eq!(holdings_after[0].quantity, dec("1.5"));

    // Verify tax lot created
    let available_qty = pnl_calc.get_available_quantity(account_id, "BTC").await?;
    assert_eq!(available_qty, dec("1.5"), "Should have 1.5 BTC in tax lots");

    // Verify transaction recorded
    let transactions = tx_repo.list_by_account(account_id, None).await?;
    assert_eq!(transactions.len(), 1, "Should have 1 transaction");
    assert_eq!(transactions[0].tx_type, TransactionType::Buy);

    Ok(())
}

/// Test: Sell transaction removes holding and creates P&L
#[tokio::test]
async fn test_sell_removes_holding_and_creates_pnl() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);
    let pnl_calc = PnLCalculator::new(&pool);

    // Create account
    let account_id = "sell_test_account";
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

    // Setup: Buy 2.0 BTC @ $40,000
    let buy_tx = Transaction::new_buy(account_id, "BTC", dec("2.0"), dec("40000"), Utc::now());
    let buy_tx_id = tx_repo.insert(&buy_tx).await?;
    holding_repo.add_quantity(account_id, "BTC", dec("2.0"), Some(dec("40000"))).await?;
    pnl_calc.process_acquisition(
        buy_tx_id,
        account_id,
        "BTC",
        dec("2.0"),
        dec("40000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Verify initial state
    let holding_before = holding_repo.get(account_id, "BTC").await?;
    assert!(holding_before.is_some(), "Should have BTC holding");
    assert_eq!(holding_before.unwrap().quantity, dec("2.0"));

    // Sell 1.0 BTC @ $50,000
    let sell_tx = Transaction::new_sell(account_id, "BTC", dec("1.0"), dec("50000"), Utc::now());
    let sell_tx_id = tx_repo.insert(&sell_tx).await?;

    // Process disposal first (before removing holding)
    let pnls = pnl_calc.process_disposal(
        sell_tx_id,
        account_id,
        "BTC",
        dec("1.0"),
        dec("50000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Remove holding
    holding_repo.remove_quantity(account_id, "BTC", dec("1.0")).await?;

    // Verify holding reduced
    let holding_after = holding_repo.get(account_id, "BTC").await?;
    assert!(holding_after.is_some(), "Should still have BTC holding");
    assert_eq!(holding_after.unwrap().quantity, dec("1.0"), "Should have 1.0 BTC remaining");

    // Verify P&L created
    assert_eq!(pnls.len(), 1, "Should create 1 P&L record");
    assert_eq!(pnls[0].realized_gain, dec("10000"), "Should have $10k gain");

    // Verify transactions recorded
    let transactions = tx_repo.list_by_account(account_id, None).await?;
    assert_eq!(transactions.len(), 2, "Should have 2 transactions (buy + sell)");

    Ok(())
}

/// Test: Swap transaction handles both sides
#[tokio::test]
async fn test_swap_handles_both_sides() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);
    let pnl_calc = PnLCalculator::new(&pool);

    // Create account
    let account_id = "swap_test_account";
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

    // Setup: Buy 1.0 BTC @ $40,000
    let buy_tx = Transaction::new_buy(account_id, "BTC", dec("1.0"), dec("40000"), Utc::now());
    let buy_tx_id = tx_repo.insert(&buy_tx).await?;
    holding_repo.add_quantity(account_id, "BTC", dec("1.0"), Some(dec("40000"))).await?;
    pnl_calc.process_acquisition(
        buy_tx_id,
        account_id,
        "BTC",
        dec("1.0"),
        dec("40000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Swap: 1.0 BTC → 15 ETH (BTC price = $50,000, ETH price = $3,333.33)
    let mut swap_tx = Transaction {
        id: 0,
        tx_type: TransactionType::Swap,
        from_account_id: Some(account_id.to_string()),
        from_asset: Some("BTC".to_string()),
        from_quantity: Some(dec("1.0")),
        to_account_id: Some(account_id.to_string()),
        to_asset: Some("ETH".to_string()),
        to_quantity: Some(dec("15")),
        price_usd: Some(dec("50000")), // BTC price
        price_currency: None,
        price_amount: None,
        exchange_rate: None,
        exchange_rate_pair: None,
        fee: None,
        fee_asset: None,
        external_id: None,
        notes: None,
        timestamp: Utc::now(),
        created_at: Utc::now(),
    };
    let swap_tx_id = tx_repo.insert(&swap_tx).await?;

    // Process swap (sell BTC side)
    let pnls = pnl_calc.process_disposal(
        swap_tx_id,
        account_id,
        "BTC",
        dec("1.0"),
        dec("50000"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Process swap (buy ETH side)
    pnl_calc.process_acquisition(
        swap_tx_id,
        account_id,
        "ETH",
        dec("15"),
        dec("3333.33"),
        Utc::now(),
        CostBasisMethod::Fifo,
    ).await?;

    // Update holdings
    holding_repo.remove_quantity(account_id, "BTC", dec("1.0")).await?;
    holding_repo.add_quantity(account_id, "ETH", dec("15"), Some(dec("3333.33"))).await?;

    // Verify BTC holding removed
    let btc_holding = holding_repo.get(account_id, "BTC").await?;
    assert!(btc_holding.is_none() || btc_holding.unwrap().quantity == dec("0"), "BTC should be sold");

    // Verify ETH holding created
    let eth_holding = holding_repo.get(account_id, "ETH").await?;
    assert!(eth_holding.is_some(), "Should have ETH holding");
    assert_eq!(eth_holding.unwrap().quantity, dec("15"), "Should have 15 ETH");

    // Verify P&L from BTC sale
    assert_eq!(pnls.len(), 1, "Should create P&L for BTC sale");
    assert_eq!(pnls[0].realized_gain, dec("10000"), "Should have $10k gain from BTC");

    // Verify ETH tax lot created
    let eth_available = pnl_calc.get_available_quantity(account_id, "ETH").await?;
    assert_eq!(eth_available, dec("15"), "Should have 15 ETH in tax lots");

    Ok(())
}

/// Test: Transfer between accounts preserves holdings
#[tokio::test]
async fn test_transfer_preserves_holdings() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    // Create category
    if account_repo.get_category("test").await?.is_none() {
        account_repo.create_category("test", "Test Category").await?;
    }

    // Create two accounts
    let account_a_id = "transfer_account_a";
    let account_b_id = "transfer_account_b";

    for account_id in [account_a_id, account_b_id] {
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

    // Setup: Account A has 2.0 BTC
    holding_repo.add_quantity(account_a_id, "BTC", dec("2.0"), Some(dec("40000"))).await?;

    // Transfer 1.0 BTC from Account A to Account B
    let transfer_tx = Transaction {
        id: 0,
        tx_type: TransactionType::TransferOut,
        from_account_id: Some(account_a_id.to_string()),
        from_asset: Some("BTC".to_string()),
        from_quantity: Some(dec("1.0")),
        to_account_id: Some(account_b_id.to_string()),
        to_asset: Some("BTC".to_string()),
        to_quantity: Some(dec("1.0")),
        price_usd: None,
        price_currency: None,
        price_amount: None,
        exchange_rate: None,
        exchange_rate_pair: None,
        fee: None,
        fee_asset: None,
        external_id: None,
        notes: None,
        timestamp: Utc::now(),
        created_at: Utc::now(),
    };
    tx_repo.insert(&transfer_tx).await?;

    // Update holdings
    holding_repo.remove_quantity(account_a_id, "BTC", dec("1.0")).await?;
    holding_repo.add_quantity(account_b_id, "BTC", dec("1.0"), None).await?;

    // Verify Account A holding reduced
    let holding_a = holding_repo.get(account_a_id, "BTC").await?;
    assert!(holding_a.is_some(), "Account A should still have BTC");
    let holding_a_qty = holding_a.as_ref().unwrap().quantity;
    assert_eq!(holding_a_qty, dec("1.0"), "Account A should have 1.0 BTC");

    // Verify Account B holding created
    let holding_b = holding_repo.get(account_b_id, "BTC").await?;
    assert!(holding_b.is_some(), "Account B should have BTC");
    let holding_b_qty = holding_b.as_ref().unwrap().quantity;
    assert_eq!(holding_b_qty, dec("1.0"), "Account B should have 1.0 BTC");

    // Verify total holdings preserved
    let total_btc = holding_a_qty + holding_b_qty;
    assert_eq!(total_btc, dec("2.0"), "Total BTC should be preserved");

    Ok(())
}

/// Test: Multiple transactions on same asset accumulate correctly
#[tokio::test]
async fn test_multiple_transactions_accumulate() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);
    let pnl_calc = PnLCalculator::new(&pool);

    // Create account
    let account_id = "accumulate_test_account";
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

    // Buy #1: 1.0 BTC @ $40,000
    let tx1 = Transaction::new_buy(account_id, "BTC", dec("1.0"), dec("40000"), Utc::now());
    let tx1_id = tx_repo.insert(&tx1).await?;
    holding_repo.add_quantity(account_id, "BTC", dec("1.0"), Some(dec("40000"))).await?;
    pnl_calc.process_acquisition(tx1_id, account_id, "BTC", dec("1.0"), dec("40000"), Utc::now(), CostBasisMethod::Fifo).await?;

    // Buy #2: 0.5 BTC @ $45,000
    let tx2 = Transaction::new_buy(account_id, "BTC", dec("0.5"), dec("45000"), Utc::now());
    let tx2_id = tx_repo.insert(&tx2).await?;
    holding_repo.add_quantity(account_id, "BTC", dec("0.5"), Some(dec("45000"))).await?;
    pnl_calc.process_acquisition(tx2_id, account_id, "BTC", dec("0.5"), dec("45000"), Utc::now(), CostBasisMethod::Fifo).await?;

    // Buy #3: 0.25 BTC @ $50,000
    let tx3 = Transaction::new_buy(account_id, "BTC", dec("0.25"), dec("50000"), Utc::now());
    let tx3_id = tx_repo.insert(&tx3).await?;
    holding_repo.add_quantity(account_id, "BTC", dec("0.25"), Some(dec("50000"))).await?;
    pnl_calc.process_acquisition(tx3_id, account_id, "BTC", dec("0.25"), dec("50000"), Utc::now(), CostBasisMethod::Fifo).await?;

    // Verify total holding
    let holding = holding_repo.get(account_id, "BTC").await?;
    assert!(holding.is_some(), "Should have BTC holding");
    assert_eq!(holding.unwrap().quantity, dec("1.75"), "Should have 1.75 BTC total");

    // Verify total available in tax lots
    let available = pnl_calc.get_available_quantity(account_id, "BTC").await?;
    assert_eq!(available, dec("1.75"), "Should have 1.75 BTC in tax lots");

    // Verify transaction count
    let transactions = tx_repo.list_by_account(account_id, None).await?;
    assert_eq!(transactions.len(), 3, "Should have 3 transactions");

    Ok(())
}
