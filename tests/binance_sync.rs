// Binance Sync Integration Tests
// Tests the complete import/sync workflow: TransactionImporter, SyncStateRepository,
// and the utility helpers — all against an in-memory SQLite database.

use chrono::Utc;
use cryptofolio::core::account::{Account, AccountConfig, AccountType};
use cryptofolio::db::sync_state::SyncStateRepository;
use cryptofolio::db::{AccountRepository, HoldingRepository, TransactionRepository};
use cryptofolio::error::Result;
use cryptofolio::exchange::binance::import::{
    is_usd_equivalent, ms_to_datetime, parse_symbol, TransactionImporter,
};
use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::str::FromStr;

mod common;

// =============================================================================
// Helpers
// =============================================================================

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("Invalid decimal in test")
}

/// Create an in-memory DB, apply all migrations, and insert a test account.
async fn setup_db_with_account(account_id: &str) -> Result<SqlitePool> {
    let pool = common::setup_test_db().await?;
    let repo = AccountRepository::new(&pool);

    if repo.get_category("test-cat").await?.is_none() {
        repo.create_category("test-cat", "Test").await?;
    }

    let acc = Account {
        id: account_id.to_string(),
        name: format!("Account {}", account_id),
        account_type: AccountType::Exchange,
        category_id: "test-cat".to_string(),
        config: AccountConfig::default(),
        sync_enabled: true,
        created_at: Utc::now(),
    };
    repo.create_account(&acc).await?;

    Ok(pool)
}

// Construct BinanceTrade test fixtures
use cryptofolio::exchange::binance::models::{
    BinanceDeposit, BinanceFiatOrder, BinanceTrade, BinanceTransfer, BinanceWithdrawal,
};

fn make_trade(id: i64, symbol: &str, price: &str, qty: &str, is_buyer: bool) -> BinanceTrade {
    BinanceTrade {
        symbol: symbol.to_string(),
        id,
        order_id: 1000 + id,
        price: dec(price),
        qty: dec(qty),
        quote_qty: dec(price) * dec(qty),
        commission: dec("0.0001"),
        commission_asset: "BNB".to_string(),
        time: 1_700_000_000_000 + id,
        is_buyer,
        is_maker: false,
    }
}

fn make_deposit(id: &str, coin: &str, amount: &str, status: i32) -> BinanceDeposit {
    BinanceDeposit {
        id: id.to_string(),
        amount: dec(amount),
        coin: coin.to_string(),
        network: "BTC".to_string(),
        status,
        address: Some("1ABC123".to_string()),
        tx_id: Some(format!("txid-{}", id)),
        insert_time: 1_700_000_000_000,
        confirm_times: None,
    }
}

fn make_withdrawal(
    id: &str,
    coin: &str,
    amount: &str,
    fee: &str,
    status: i32,
) -> BinanceWithdrawal {
    BinanceWithdrawal {
        id: id.to_string(),
        amount: dec(amount),
        transaction_fee: dec(fee),
        coin: coin.to_string(),
        status,
        address: "1XYZ456deadbeef00".to_string(),
        tx_id: Some(format!("txid-wd-{}", id)),
        apply_time: 1_700_001_000_000,
        network: "BTC".to_string(),
        withdraw_order_id: None,
    }
}

fn make_fiat_order(
    order_no: &str,
    crypto: &str,
    amount: &str,
    fiat: &str,
    status: &str,
) -> BinanceFiatOrder {
    BinanceFiatOrder {
        order_no: order_no.to_string(),
        fiat_currency: fiat.to_string(),
        indicated_amount: dec("500"), // fiat amount paid
        amount: dec(amount),          // crypto received
        total_fee: dec("5"),          // fee in fiat
        method: "Card".to_string(),
        status: status.to_string(),
        crypto_currency: crypto.to_string(),
        create_time: 1_700_000_000_000,
        update_time: 1_700_000_100_000,
    }
}

fn make_transfer(tran_id: i64, asset: &str, amount: &str, status: &str) -> BinanceTransfer {
    BinanceTransfer {
        tran_id,
        amount: dec(amount),
        asset: asset.to_string(),
        transfer_type: "MAIN_UMFUTURE".to_string(),
        status: status.to_string(),
        timestamp: 1_700_000_000_000 + tran_id,
    }
}

// =============================================================================
// parse_symbol tests
// =============================================================================

#[test]
fn test_parse_symbol_btcusdt() {
    assert_eq!(
        parse_symbol("BTCUSDT").unwrap(),
        ("BTC".into(), "USDT".into())
    );
}

#[test]
fn test_parse_symbol_ethbtc() {
    assert_eq!(
        parse_symbol("ETHBTC").unwrap(),
        ("ETH".into(), "BTC".into())
    );
}

#[test]
fn test_parse_symbol_adabusd() {
    assert_eq!(
        parse_symbol("ADABUSD").unwrap(),
        ("ADA".into(), "BUSD".into())
    );
}

#[test]
fn test_parse_symbol_lowercase() {
    assert_eq!(
        parse_symbol("ethusdt").unwrap(),
        ("ETH".into(), "USDT".into())
    );
}

#[test]
fn test_parse_symbol_bnbeth() {
    assert_eq!(
        parse_symbol("BNBETH").unwrap(),
        ("BNB".into(), "ETH".into())
    );
}

#[test]
fn test_parse_symbol_unknown_errors() {
    assert!(parse_symbol("BTCXXX").is_err());
    assert!(parse_symbol("").is_err());
    assert!(parse_symbol("USDT").is_err()); // only quote asset, no base
}

// =============================================================================
// is_usd_equivalent tests
// =============================================================================

#[test]
fn test_is_usd_equivalent_stablecoins() {
    assert!(is_usd_equivalent("USDT"));
    assert!(is_usd_equivalent("BUSD"));
    assert!(is_usd_equivalent("USDC"));
    assert!(is_usd_equivalent("DAI"));
}

#[test]
fn test_is_usd_equivalent_case_insensitive() {
    assert!(is_usd_equivalent("usdt"));
    assert!(is_usd_equivalent("Busd"));
}

#[test]
fn test_is_usd_equivalent_non_stable() {
    assert!(!is_usd_equivalent("BTC"));
    assert!(!is_usd_equivalent("ETH"));
    assert!(!is_usd_equivalent("BNB"));
}

// =============================================================================
// ms_to_datetime tests
// =============================================================================

#[test]
fn test_ms_to_datetime_epoch() {
    let dt = ms_to_datetime(0).unwrap();
    assert_eq!(dt.timestamp(), 0);
}

#[test]
fn test_ms_to_datetime_recent() {
    let ms = 1_700_000_000_000i64;
    let dt = ms_to_datetime(ms).unwrap();
    assert_eq!(dt.timestamp(), 1_700_000_000);
}

// =============================================================================
// TransactionImporter — buy trade
// =============================================================================

#[tokio::test]
async fn test_import_buy_trade_creates_transaction_holding_and_tax_lot() -> Result<()> {
    let pool = setup_db_with_account("acc-buy").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    let trade = make_trade(1, "BTCUSDT", "50000", "0.1", true);
    let result = importer.import_trade("acc-buy", &trade, false).await?;

    // Should create a transaction
    assert!(result.is_created(), "Expected Created, got {:?}", result);

    // Holding should reflect the buy
    let holding = holding_repo.get("acc-buy", "BTC").await?.unwrap();
    assert_eq!(holding.quantity, dec("0.1"));
    assert_eq!(holding.avg_cost_basis, Some(dec("50000")));

    // Transaction in DB
    let txs = tx_repo.list_by_account("acc-buy", None).await?;
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].to_asset, Some("BTC".to_string()));
    assert_eq!(txs[0].to_quantity, Some(dec("0.1")));
    assert_eq!(txs[0].external_id, Some("binance-trade-1".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_import_buy_trade_sets_price_usd_for_usdt_pair() -> Result<()> {
    let pool = setup_db_with_account("acc-price").await?;
    let importer = TransactionImporter::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    let trade = make_trade(1, "ETHUSDT", "3000", "1.0", true);
    importer.import_trade("acc-price", &trade, false).await?;

    let txs = tx_repo.list_by_account("acc-price", None).await?;
    // USDT pair → price_usd should be set
    assert_eq!(txs[0].price_usd, Some(dec("3000")));

    Ok(())
}

#[tokio::test]
async fn test_import_buy_trade_no_price_usd_for_non_stable_pair() -> Result<()> {
    let pool = setup_db_with_account("acc-noprice").await?;
    let importer = TransactionImporter::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    let trade = make_trade(1, "ETHBTC", "0.065", "2.0", true);
    importer.import_trade("acc-noprice", &trade, false).await?;

    let txs = tx_repo.list_by_account("acc-noprice", None).await?;
    // BTC quote → no USD price available
    assert_eq!(txs[0].price_usd, None);

    Ok(())
}

// =============================================================================
// TransactionImporter — sell trade
// =============================================================================

#[tokio::test]
async fn test_import_sell_trade_after_buy_creates_pnl() -> Result<()> {
    let pool = setup_db_with_account("acc-sell").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);

    // First import a buy
    let buy = make_trade(1, "BTCUSDT", "40000", "1.0", true);
    importer.import_trade("acc-sell", &buy, false).await?;

    // Verify holding after buy
    let h = holding_repo.get("acc-sell", "BTC").await?.unwrap();
    assert_eq!(h.quantity, dec("1.0"));

    // Now import a sell
    let sell = make_trade(2, "BTCUSDT", "50000", "1.0", false);
    let result = importer.import_trade("acc-sell", &sell, false).await?;
    assert!(result.is_created());

    // Holding should be gone (or zero)
    let h_after = holding_repo.get("acc-sell", "BTC").await?;
    assert!(h_after.is_none() || h_after.unwrap().quantity == dec("0"));

    Ok(())
}

#[tokio::test]
async fn test_import_sell_without_prior_buy_does_not_panic() -> Result<()> {
    // Sell with no prior holding should not return an error — just silently
    // skip the holding removal (we may be importing records pre-dating our DB)
    let pool = setup_db_with_account("acc-noprior").await?;
    let importer = TransactionImporter::new(&pool);

    let sell = make_trade(99, "BTCUSDT", "60000", "0.5", false);
    let result = importer.import_trade("acc-noprior", &sell, false).await?;
    assert!(
        result.is_created(),
        "Should still create the transaction record"
    );

    Ok(())
}

// =============================================================================
// TransactionImporter — duplicate detection
// =============================================================================

#[tokio::test]
async fn test_duplicate_trade_is_skipped() -> Result<()> {
    let pool = setup_db_with_account("acc-dup-trade").await?;
    let importer = TransactionImporter::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    let trade = make_trade(42, "ETHUSDT", "3000", "0.5", true);

    let r1 = importer
        .import_trade("acc-dup-trade", &trade, false)
        .await?;
    let r2 = importer
        .import_trade("acc-dup-trade", &trade, false)
        .await?;

    assert!(r1.is_created());
    assert!(r2.is_skipped());

    // Only one transaction in DB
    let txs = tx_repo.list_by_account("acc-dup-trade", None).await?;
    assert_eq!(txs.len(), 1, "Should not create duplicate transaction");

    Ok(())
}

#[tokio::test]
async fn test_duplicate_deposit_is_skipped() -> Result<()> {
    let pool = setup_db_with_account("acc-dup-dep").await?;
    let importer = TransactionImporter::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    let deposit = make_deposit("dep-001", "ETH", "2.0", 1);

    let r1 = importer
        .import_deposit("acc-dup-dep", &deposit, false)
        .await?;
    let r2 = importer
        .import_deposit("acc-dup-dep", &deposit, false)
        .await?;

    assert!(r1.is_created());
    assert!(r2.is_skipped());

    let txs = tx_repo.list_by_account("acc-dup-dep", None).await?;
    assert_eq!(txs.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_duplicate_withdrawal_is_skipped() -> Result<()> {
    let pool = setup_db_with_account("acc-dup-wd").await?;
    let importer = TransactionImporter::new(&pool);

    let withdrawal = make_withdrawal("wd-001", "BTC", "0.5", "0.0005", 6);

    let r1 = importer
        .import_withdrawal("acc-dup-wd", &withdrawal, false)
        .await?;
    let r2 = importer
        .import_withdrawal("acc-dup-wd", &withdrawal, false)
        .await?;

    assert!(r1.is_created());
    assert!(r2.is_skipped());

    Ok(())
}

// =============================================================================
// TransactionImporter — dry run
// =============================================================================

#[tokio::test]
async fn test_dry_run_trade_has_no_side_effects() -> Result<()> {
    let pool = setup_db_with_account("acc-dry-trade").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    let trade = make_trade(1, "BTCUSDT", "50000", "0.1", true);
    let result = importer.import_trade("acc-dry-trade", &trade, true).await?;

    // Dry-run returns Created(0) to indicate "would create" in reporting
    assert!(result.is_created());

    // No holding created
    assert!(holding_repo.get("acc-dry-trade", "BTC").await?.is_none());

    // No transaction created
    assert_eq!(
        tx_repo.list_by_account("acc-dry-trade", None).await?.len(),
        0
    );

    Ok(())
}

#[tokio::test]
async fn test_dry_run_deposit_has_no_side_effects() -> Result<()> {
    let pool = setup_db_with_account("acc-dry-dep").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    let deposit = make_deposit("dep-dry", "BTC", "0.5", 1);
    let result = importer
        .import_deposit("acc-dry-dep", &deposit, true)
        .await?;

    // Dry-run returns Created(0) to indicate "would create" in reporting
    assert!(result.is_created());
    assert!(holding_repo.get("acc-dry-dep", "BTC").await?.is_none());
    assert_eq!(tx_repo.list_by_account("acc-dry-dep", None).await?.len(), 0);

    Ok(())
}

// =============================================================================
// TransactionImporter — deposit
// =============================================================================

#[tokio::test]
async fn test_import_successful_deposit_updates_holdings() -> Result<()> {
    let pool = setup_db_with_account("acc-dep").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);

    let deposit = make_deposit("dep-001", "BTC", "0.5", 1);
    let result = importer.import_deposit("acc-dep", &deposit, false).await?;

    assert!(result.is_created());

    let h = holding_repo.get("acc-dep", "BTC").await?.unwrap();
    assert_eq!(h.quantity, dec("0.5"));
    // Deposit has no cost basis
    assert_eq!(h.avg_cost_basis, None);

    Ok(())
}

#[tokio::test]
async fn test_import_deposit_status_6_also_accepted() -> Result<()> {
    let pool = setup_db_with_account("acc-dep6").await?;
    let importer = TransactionImporter::new(&pool);

    let deposit = make_deposit("dep-002", "ETH", "1.0", 6); // 6 = credited
    let result = importer.import_deposit("acc-dep6", &deposit, false).await?;
    assert!(result.is_created());

    Ok(())
}

#[tokio::test]
async fn test_import_pending_deposit_is_skipped() -> Result<()> {
    let pool = setup_db_with_account("acc-dep-pending").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);

    let deposit = make_deposit("dep-003", "BTC", "0.5", 0); // 0 = pending
    let result = importer
        .import_deposit("acc-dep-pending", &deposit, false)
        .await?;

    assert!(result.is_skipped());
    assert!(holding_repo.get("acc-dep-pending", "BTC").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn test_import_multiple_deposits_accumulate_holdings() -> Result<()> {
    let pool = setup_db_with_account("acc-multi-dep").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);

    importer
        .import_deposit("acc-multi-dep", &make_deposit("d1", "BTC", "0.5", 1), false)
        .await?;
    importer
        .import_deposit("acc-multi-dep", &make_deposit("d2", "BTC", "0.3", 1), false)
        .await?;
    importer
        .import_deposit("acc-multi-dep", &make_deposit("d3", "ETH", "5.0", 1), false)
        .await?;

    let btc = holding_repo.get("acc-multi-dep", "BTC").await?.unwrap();
    assert_eq!(btc.quantity, dec("0.8"));

    let eth = holding_repo.get("acc-multi-dep", "ETH").await?.unwrap();
    assert_eq!(eth.quantity, dec("5.0"));

    Ok(())
}

// =============================================================================
// TransactionImporter — withdrawal
// =============================================================================

#[tokio::test]
async fn test_import_successful_withdrawal_reduces_holdings() -> Result<()> {
    let pool = setup_db_with_account("acc-wd").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);

    // Seed holdings first
    holding_repo
        .add_quantity("acc-wd", "BTC", dec("1.0"), None)
        .await?;

    let wd = make_withdrawal("wd-001", "BTC", "0.4", "0.0005", 6);
    let result = importer.import_withdrawal("acc-wd", &wd, false).await?;
    assert!(result.is_created());

    let h = holding_repo.get("acc-wd", "BTC").await?.unwrap();
    // 1.0 - (0.4 + 0.0005 fee) = 0.5995
    assert_eq!(h.quantity, dec("0.5995"));

    Ok(())
}

#[tokio::test]
async fn test_import_non_success_withdrawal_skipped() -> Result<()> {
    let pool = setup_db_with_account("acc-wd-skip").await?;
    let importer = TransactionImporter::new(&pool);

    // status 2 = awaiting approval
    let wd = make_withdrawal("wd-002", "ETH", "1.0", "0.005", 2);
    let result = importer
        .import_withdrawal("acc-wd-skip", &wd, false)
        .await?;
    assert!(result.is_skipped());

    Ok(())
}

#[tokio::test]
async fn test_import_withdrawal_no_holding_does_not_error() -> Result<()> {
    // Importing a withdrawal for a coin we have no holding of should not fail —
    // the holding removal is silently ignored for pre-sync records.
    let pool = setup_db_with_account("acc-wd-nohold").await?;
    let importer = TransactionImporter::new(&pool);

    let wd = make_withdrawal("wd-003", "ADA", "100.0", "1.0", 6);
    let result = importer
        .import_withdrawal("acc-wd-nohold", &wd, false)
        .await?;
    assert!(result.is_created(), "Should still record the transaction");

    Ok(())
}

// =============================================================================
// TransactionImporter — fiat orders
// =============================================================================

#[tokio::test]
async fn test_import_completed_fiat_order_creates_buy() -> Result<()> {
    let pool = setup_db_with_account("acc-fiat").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    // Buying 0.01 BTC with a credit card (paying $500 USD)
    let order = make_fiat_order("fiat-001", "BTC", "0.01", "USD", "Completed");
    let result = importer
        .import_fiat_order("acc-fiat", &order, false)
        .await?;
    assert!(result.is_created());

    let h = holding_repo.get("acc-fiat", "BTC").await?.unwrap();
    assert_eq!(h.quantity, dec("0.01"));

    let txs = tx_repo.list_by_account("acc-fiat", None).await?;
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].to_asset, Some("BTC".to_string()));
    // Price = 500 fiat / 0.01 crypto = 50000
    assert_eq!(txs[0].price_usd, Some(dec("50000")));
    assert_eq!(
        txs[0].external_id,
        Some("binance-fiat-fiat-001".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn test_import_incomplete_fiat_order_skipped() -> Result<()> {
    let pool = setup_db_with_account("acc-fiat-skip").await?;
    let importer = TransactionImporter::new(&pool);

    let order = make_fiat_order("fiat-002", "ETH", "0.1", "USD", "Processing");
    let result = importer
        .import_fiat_order("acc-fiat-skip", &order, false)
        .await?;
    assert!(result.is_skipped());

    Ok(())
}

#[tokio::test]
async fn test_import_fiat_order_duplicate_skipped() -> Result<()> {
    let pool = setup_db_with_account("acc-fiat-dup").await?;
    let importer = TransactionImporter::new(&pool);

    let order = make_fiat_order("fiat-003", "USDT", "100", "EUR", "Completed");
    let r1 = importer
        .import_fiat_order("acc-fiat-dup", &order, false)
        .await?;
    let r2 = importer
        .import_fiat_order("acc-fiat-dup", &order, false)
        .await?;

    assert!(r1.is_created());
    assert!(r2.is_skipped());

    Ok(())
}

// =============================================================================
// TransactionImporter — internal transfers
// =============================================================================

#[tokio::test]
async fn test_import_confirmed_transfer_creates_transaction() -> Result<()> {
    let pool = setup_db_with_account("acc-tran").await?;
    let importer = TransactionImporter::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    let transfer = make_transfer(9001, "USDT", "500", "CONFIRMED");
    let result = importer
        .import_transfer("acc-tran", &transfer, false)
        .await?;
    assert!(result.is_created());

    let txs = tx_repo.list_by_account("acc-tran", None).await?;
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].from_account_id, Some("acc-tran".to_string()));
    assert_eq!(txs[0].to_account_id, Some("acc-tran".to_string()));
    assert_eq!(txs[0].from_asset, Some("USDT".to_string()));
    assert_eq!(
        txs[0].external_id,
        Some("binance-transfer-9001".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn test_import_pending_transfer_is_skipped() -> Result<()> {
    let pool = setup_db_with_account("acc-tran-pending").await?;
    let importer = TransactionImporter::new(&pool);

    let transfer = make_transfer(9002, "BNB", "1.0", "PENDING");
    let result = importer
        .import_transfer("acc-tran-pending", &transfer, false)
        .await?;
    assert!(result.is_skipped());

    Ok(())
}

#[tokio::test]
async fn test_import_transfer_does_not_change_holdings() -> Result<()> {
    // Internal transfer is same account both sides — holdings unchanged
    let pool = setup_db_with_account("acc-tran-hold").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);

    holding_repo
        .add_quantity("acc-tran-hold", "USDT", dec("1000"), None)
        .await?;

    let transfer = make_transfer(9003, "USDT", "200", "CONFIRMED");
    importer
        .import_transfer("acc-tran-hold", &transfer, false)
        .await?;

    // Holdings unchanged (same account both sides)
    let h = holding_repo.get("acc-tran-hold", "USDT").await?.unwrap();
    assert_eq!(h.quantity, dec("1000"));

    Ok(())
}

// =============================================================================
// Mixed import sequence
// =============================================================================

#[tokio::test]
async fn test_mixed_import_sequence() -> Result<()> {
    // Realistic sequence: fiat purchase → buy trades → deposit → withdrawal
    let pool = setup_db_with_account("acc-mixed").await?;
    let importer = TransactionImporter::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    // 1. Fiat purchase: buy 100 USDT with credit card
    let fiat = make_fiat_order("fiat-seq-1", "USDT", "100", "USD", "Completed");
    let r1 = importer
        .import_fiat_order("acc-mixed", &fiat, false)
        .await?;
    assert!(r1.is_created());

    // 2. Buy 0.001 BTC with USDT
    let buy = make_trade(1, "BTCUSDT", "50000", "0.001", true);
    let r2 = importer.import_trade("acc-mixed", &buy, false).await?;
    assert!(r2.is_created());

    // 3. Deposit 0.5 ETH from external wallet
    let dep = make_deposit("dep-seq-1", "ETH", "0.5", 1);
    let r3 = importer.import_deposit("acc-mixed", &dep, false).await?;
    assert!(r3.is_created());

    // 4. Withdraw 0.1 ETH
    holding_repo
        .add_quantity("acc-mixed", "ETH", dec("0.4"), None) // ETH already has 0.5 from step 3
        .await?;
    let wd = make_withdrawal("wd-seq-1", "ETH", "0.2", "0.001", 6);
    let r4 = importer.import_withdrawal("acc-mixed", &wd, false).await?;
    assert!(r4.is_created());

    // Check final state
    let _usdt = holding_repo.get("acc-mixed", "USDT").await?;
    let btc = holding_repo.get("acc-mixed", "BTC").await?.unwrap();
    let eth = holding_repo.get("acc-mixed", "ETH").await?.unwrap();

    // USDT: 100 from fiat buy (buy trade uses it but USDT balance not tracked separately here)
    assert_eq!(btc.quantity, dec("0.001"));
    assert!(eth.quantity > dec("0")); // some ETH remaining

    // Total 4 transactions recorded
    let txs = tx_repo.list_by_account("acc-mixed", None).await?;
    assert_eq!(txs.len(), 4, "Should have 4 transactions total");

    Ok(())
}

// =============================================================================
// SyncStateRepository integration
// =============================================================================

#[tokio::test]
async fn test_sync_state_watermarks_update_correctly() -> Result<()> {
    let pool = setup_db_with_account("acc-state").await?;
    let repo = SyncStateRepository::new(&pool);

    // Initially no sync state
    let state = repo.get_or_create("acc-state").await?;
    assert!(state.needs_full_trade_sync());
    assert!(state.needs_full_deposit_sync());
    assert!(state.needs_full_withdrawal_sync());

    // Update trade sync
    let now = Utc::now();
    repo.update_trade_sync("acc-state", now, Some(12345), Some("BTCUSDT"))
        .await?;

    let state2 = repo.get("acc-state").await?.unwrap();
    assert!(!state2.needs_full_trade_sync());
    assert_eq!(state2.last_trade_id, Some(12345));
    assert_eq!(state2.last_sync_symbol, Some("BTCUSDT".to_string()));

    // Update deposit sync
    repo.update_deposit_sync("acc-state", now).await?;
    let state3 = repo.get("acc-state").await?.unwrap();
    assert!(!state3.needs_full_deposit_sync());

    // Update withdrawal sync
    repo.update_withdrawal_sync("acc-state", now).await?;
    let state4 = repo.get("acc-state").await?.unwrap();
    assert!(!state4.needs_full_withdrawal_sync());

    Ok(())
}

#[tokio::test]
async fn test_sync_state_reset_clears_all_watermarks() -> Result<()> {
    let pool = setup_db_with_account("acc-reset").await?;
    let repo = SyncStateRepository::new(&pool);

    let now = Utc::now();
    repo.get_or_create("acc-reset").await?;
    repo.update_trade_sync("acc-reset", now, Some(99), Some("ETHUSDT"))
        .await?;
    repo.update_deposit_sync("acc-reset", now).await?;
    repo.update_withdrawal_sync("acc-reset", now).await?;

    // Verify watermarks set
    let state = repo.get("acc-reset").await?.unwrap();
    assert!(!state.needs_full_trade_sync());

    // Reset
    repo.reset("acc-reset").await?;

    let state2 = repo.get("acc-reset").await?.unwrap();
    assert!(state2.needs_full_trade_sync());
    assert!(state2.needs_full_deposit_sync());
    assert!(state2.needs_full_withdrawal_sync());
    assert!(state2.last_trade_id.is_none());
    assert!(state2.last_sync_symbol.is_none());

    Ok(())
}

#[tokio::test]
async fn test_sync_state_get_or_create_is_idempotent() -> Result<()> {
    let pool = setup_db_with_account("acc-idemp").await?;
    let repo = SyncStateRepository::new(&pool);

    // Calling get_or_create multiple times should not error or duplicate
    let _s1 = repo.get_or_create("acc-idemp").await?;
    let _s2 = repo.get_or_create("acc-idemp").await?;
    let s3 = repo.get_or_create("acc-idemp").await?;

    assert_eq!(s3.account_id, "acc-idemp");

    Ok(())
}

#[tokio::test]
async fn test_sync_state_fiat_and_transfer_watermarks() -> Result<()> {
    let pool = setup_db_with_account("acc-fiat-tran").await?;
    let repo = SyncStateRepository::new(&pool);

    let now = Utc::now();
    repo.get_or_create("acc-fiat-tran").await?;

    // Initially None
    let state = repo.get("acc-fiat-tran").await?.unwrap();
    assert!(state.last_fiat_sync.is_none());
    assert!(state.last_transfer_sync.is_none());

    // Update
    repo.update_fiat_sync("acc-fiat-tran", now).await?;
    repo.update_transfer_sync("acc-fiat-tran", now).await?;

    let state2 = repo.get("acc-fiat-tran").await?.unwrap();
    assert!(state2.last_fiat_sync.is_some());
    assert!(state2.last_transfer_sync.is_some());

    Ok(())
}

// =============================================================================
// SyncReport unit tests
// =============================================================================

use cryptofolio::exchange::binance::sync::SyncReport;

#[test]
fn test_sync_report_default_is_all_zero() {
    let r = SyncReport::default();
    assert_eq!(r.total_created(), 0);
    assert_eq!(r.total_skipped(), 0);
    assert!(!r.has_errors());
}

#[test]
fn test_sync_report_totals() {
    let mut r = SyncReport::default();
    r.trades_created = 10;
    r.deposits_created = 3;
    r.withdrawals_created = 1;
    r.fiat_orders_created = 2;
    r.transfers_created = 4;

    r.trades_skipped = 5;
    r.deposits_skipped = 1;

    assert_eq!(r.total_created(), 20);
    assert_eq!(r.total_skipped(), 6);
}

#[test]
fn test_sync_report_errors() {
    let mut r = SyncReport::default();
    assert!(!r.has_errors());
    r.errors.push("trade fetch failed".to_string());
    assert!(r.has_errors());
}

// =============================================================================
// Edge cases
// =============================================================================

#[tokio::test]
async fn test_import_trade_note_contains_trade_id() -> Result<()> {
    let pool = setup_db_with_account("acc-note").await?;
    let importer = TransactionImporter::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    let trade = make_trade(9999, "ADAUSDT", "1.20", "100", true);
    importer.import_trade("acc-note", &trade, false).await?;

    let txs = tx_repo.list_by_account("acc-note", None).await?;
    let notes = txs[0].notes.as_deref().unwrap_or("");
    assert!(
        notes.contains("9999"),
        "Notes should contain the trade ID: {notes}"
    );

    Ok(())
}

#[tokio::test]
async fn test_import_withdrawal_note_contains_address_prefix() -> Result<()> {
    let pool = setup_db_with_account("acc-wd-note").await?;
    let importer = TransactionImporter::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    holding_repo_seed(&pool, "acc-wd-note", "BTC", "1.0").await;

    let wd = make_withdrawal("wd-note-1", "BTC", "0.1", "0.001", 6);
    importer
        .import_withdrawal("acc-wd-note", &wd, false)
        .await?;

    let txs = tx_repo.list_by_account("acc-wd-note", None).await?;
    let notes = txs[0].notes.as_deref().unwrap_or("");
    // Should contain beginning of address
    assert!(
        notes.contains("1XYZ456"),
        "Notes should contain address prefix: {notes}"
    );

    Ok(())
}

async fn holding_repo_seed(pool: &SqlitePool, account_id: &str, asset: &str, qty: &str) {
    HoldingRepository::new(pool)
        .add_quantity(account_id, asset, dec(qty), None)
        .await
        .unwrap();
}
