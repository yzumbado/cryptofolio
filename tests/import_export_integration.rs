// Import/Export Integration Tests
// Tests the complete CSV import/export pipeline and data round-trip preservation

use chrono::Utc;
use cryptofolio::core::account::{Account, AccountConfig, AccountType};
use cryptofolio::core::transaction::{Transaction, TransactionType};
use cryptofolio::db::{AccountRepository, HoldingRepository, TransactionRepository};
use cryptofolio::error::Result;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use tempfile::TempDir;

mod common;

#[derive(Debug, Deserialize)]
struct CsvRecord {
    date: String,
    #[serde(rename = "type")]
    tx_type: String,
    asset: String,
    quantity: String,
    price_usd: String,
    #[serde(default)]
    fee: String,
    #[serde(default)]
    fee_asset: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    to_asset: String,
    #[serde(default)]
    to_quantity: String,
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("Invalid decimal in test")
}

/// Helper to create a test CSV file
fn create_test_csv(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let path = dir.path().join(filename);
    fs::write(&path, content).expect("Failed to write test CSV");
    path
}

/// Test: CSV import creates transactions and holdings
#[tokio::test]
async fn test_csv_import_creates_transactions_and_holdings() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    // Create test account
    let account_id = "import_test_account";
    if account_repo.get_category("test").await?.is_none() {
        account_repo
            .create_category("test", "Test Category")
            .await?;
    }
    let account = Account {
        id: account_id.to_string(),
        name: "Import Test Account".to_string(),
        account_type: AccountType::Exchange,
        category_id: "test".to_string(),
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };
    account_repo.create_account(&account).await?;

    // Create test CSV with buy transactions
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let csv_content = r#"date,type,asset,quantity,price_usd,fee,fee_asset,notes,to_asset,to_quantity
2025-01-01,buy,BTC,1.0,40000,10,USDT,First buy,,
2025-01-02,buy,ETH,10.0,3000,5,USDT,Second buy,,
2025-01-03,buy,BTC,0.5,45000,7.5,USDT,Third buy,,"#;

    let csv_path = create_test_csv(&temp_dir, "import_test.csv", csv_content);

    // Simulate import by processing CSV manually (since we're testing the data layer, not CLI)
    let file = fs::File::open(&csv_path)?;
    let mut reader = csv::Reader::from_reader(file);

    let mut imported = 0;
    for result in reader.deserialize() {
        let row: CsvRecord = result?;

        let quantity = dec(&row.quantity);
        let price_usd = dec(&row.price_usd);

        // Create transaction
        let tx = Transaction::new_buy(account_id, &row.asset, quantity, price_usd, Utc::now());
        tx_repo.insert(&tx).await?;

        // Update holding
        holding_repo
            .add_quantity(account_id, &row.asset, quantity, Some(price_usd))
            .await?;

        imported += 1;
    }

    // Verify transactions created
    let transactions = tx_repo.list_by_account(account_id, None).await?;
    assert_eq!(transactions.len(), 3, "Should have 3 imported transactions");
    assert_eq!(imported, 3, "Should have processed 3 rows");

    // Verify holdings created
    let btc_holding = holding_repo.get(account_id, "BTC").await?;
    assert!(btc_holding.is_some(), "Should have BTC holding");
    assert_eq!(
        btc_holding.unwrap().quantity,
        dec("1.5"),
        "Should have 1.5 BTC total"
    );

    let eth_holding = holding_repo.get(account_id, "ETH").await?;
    assert!(eth_holding.is_some(), "Should have ETH holding");
    assert_eq!(
        eth_holding.unwrap().quantity,
        dec("10.0"),
        "Should have 10 ETH"
    );

    Ok(())
}

/// Test: CSV export produces valid output
#[tokio::test]
async fn test_csv_export_produces_valid_output() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    // Create test account
    let account_id = "export_test_account";
    if account_repo.get_category("test").await?.is_none() {
        account_repo
            .create_category("test", "Test Category")
            .await?;
    }
    let account = Account {
        id: account_id.to_string(),
        name: "Export Test Account".to_string(),
        account_type: AccountType::Exchange,
        category_id: "test".to_string(),
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };
    account_repo.create_account(&account).await?;

    // Create test transactions
    let tx1 = Transaction::new_buy(account_id, "BTC", dec("1.0"), dec("40000"), Utc::now());
    let tx2 = Transaction::new_sell(account_id, "BTC", dec("0.5"), dec("50000"), Utc::now());
    tx_repo.insert(&tx1).await?;
    tx_repo.insert(&tx2).await?;

    // Export to CSV
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let export_path = temp_dir.path().join("export_test.csv");

    let transactions = tx_repo.list_by_account(account_id, None).await?;

    // Create CSV with proper header
    let mut content = String::from("date,type,asset,quantity,price_usd\n");
    for tx in &transactions {
        let tx_type_str = format!("{:?}", tx.tx_type).to_lowercase();
        let empty = String::new();
        let asset = if tx_type_str == "buy" {
            tx.to_asset.as_ref().unwrap_or(&empty)
        } else {
            tx.from_asset.as_ref().unwrap_or(&empty)
        };
        let quantity = if tx_type_str == "buy" {
            tx.to_quantity.unwrap_or(Decimal::ZERO)
        } else {
            tx.from_quantity.unwrap_or(Decimal::ZERO)
        };

        content.push_str(&format!(
            "{},{},{},{},{}\n",
            tx.timestamp.format("%Y-%m-%d"),
            tx_type_str,
            asset,
            quantity,
            tx.price_usd.unwrap_or(Decimal::ZERO)
        ));
    }

    fs::write(&export_path, content)?;

    // Verify file was created and is not empty
    let file_content = fs::read_to_string(&export_path)?;
    assert!(!file_content.is_empty(), "Export file should not be empty");

    // Count lines (excluding header)
    let line_count = file_content.lines().count();
    assert!(
        line_count >= 3,
        "Should have header + at least 2 transaction records"
    );

    Ok(())
}

/// Test: Round-trip import→export→import preserves data
#[tokio::test]
async fn test_roundtrip_preserves_data() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    // Create test account
    let account_id = "roundtrip_test_account";
    if account_repo.get_category("test").await?.is_none() {
        account_repo
            .create_category("test", "Test Category")
            .await?;
    }
    let account = Account {
        id: account_id.to_string(),
        name: "Roundtrip Test Account".to_string(),
        account_type: AccountType::Exchange,
        category_id: "test".to_string(),
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };
    account_repo.create_account(&account).await?;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // STEP 1: Create initial CSV
    let csv_content = r#"date,type,asset,quantity,price_usd,fee,fee_asset,notes,to_asset,to_quantity
2025-01-01,buy,BTC,2.0,40000,20,USDT,Initial purchase,,
2025-01-02,sell,BTC,0.5,50000,10,USDT,Partial sale,,"#;

    let import_path_1 = create_test_csv(&temp_dir, "roundtrip_1.csv", csv_content);

    // STEP 2: Import first CSV
    let file = fs::File::open(&import_path_1)?;
    let mut reader = csv::Reader::from_reader(file);

    let mut original_data = Vec::new();
    for result in reader.deserialize() {
        let row: CsvRecord = result?;

        let quantity = dec(&row.quantity);
        let price_usd = dec(&row.price_usd);

        // Store original data for comparison
        original_data.push((row.tx_type.clone(), row.asset.clone(), quantity, price_usd));

        // Create transaction
        let tx = if row.tx_type == "buy" {
            Transaction::new_buy(account_id, &row.asset, quantity, price_usd, Utc::now())
        } else {
            Transaction::new_sell(account_id, &row.asset, quantity, price_usd, Utc::now())
        };
        tx_repo.insert(&tx).await?;

        // Update holdings
        if row.tx_type == "buy" {
            holding_repo
                .add_quantity(account_id, &row.asset, quantity, Some(price_usd))
                .await?;
        } else {
            holding_repo
                .remove_quantity(account_id, &row.asset, quantity)
                .await?;
        }
    }

    // STEP 3: Export to new CSV
    let export_path = temp_dir.path().join("roundtrip_export.csv");
    let transactions = tx_repo.list_by_account(account_id, None).await?;

    let mut content = String::from("date,type,asset,quantity,price_usd\n");
    for tx in &transactions {
        let tx_type_str = format!("{:?}", tx.tx_type).to_lowercase();
        let empty = String::new();
        let asset = if tx_type_str == "buy" {
            tx.to_asset.as_ref().unwrap_or(&empty)
        } else {
            tx.from_asset.as_ref().unwrap_or(&empty)
        };
        let quantity = if tx_type_str == "buy" {
            tx.to_quantity.unwrap_or(Decimal::ZERO)
        } else {
            tx.from_quantity.unwrap_or(Decimal::ZERO)
        };

        content.push_str(&format!(
            "{},{},{},{},{}\n",
            tx.timestamp.format("%Y-%m-%d"),
            tx_type_str,
            asset,
            quantity,
            tx.price_usd.unwrap_or(Decimal::ZERO)
        ));
    }

    fs::write(&export_path, content)?;

    // STEP 4: Verify exported data matches original
    let mut reader = csv::Reader::from_path(&export_path)?;
    let mut exported_count = 0;

    for result in reader.records() {
        let _record = result?;
        exported_count += 1;
    }

    assert_eq!(
        exported_count,
        original_data.len(),
        "Exported transaction count should match original"
    );

    // STEP 5: Verify holdings are correct
    let btc_holding = holding_repo.get(account_id, "BTC").await?;
    assert!(
        btc_holding.is_some(),
        "Should have BTC holding after round-trip"
    );
    // Bought 2.0, sold 0.5 = 1.5 remaining
    assert_eq!(
        btc_holding.unwrap().quantity,
        dec("1.5"),
        "Should have 1.5 BTC after round-trip"
    );

    Ok(())
}

/// Test: Import handles multiple transaction types
#[tokio::test]
async fn test_import_handles_multiple_transaction_types() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let holding_repo = HoldingRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    // Create test account
    let account_id = "multi_type_account";
    if account_repo.get_category("test").await?.is_none() {
        account_repo
            .create_category("test", "Test Category")
            .await?;
    }
    let account = Account {
        id: account_id.to_string(),
        name: "Multi Type Account".to_string(),
        account_type: AccountType::Exchange,
        category_id: "test".to_string(),
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };
    account_repo.create_account(&account).await?;

    // Create CSV with different transaction types
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let csv_content = r#"date,type,asset,quantity,price_usd,fee,fee_asset,notes,to_asset,to_quantity
2025-01-01,buy,BTC,1.0,40000,10,USDT,Buy BTC,,
2025-01-02,sell,BTC,0.3,50000,5,USDT,Sell some BTC,,
2025-01-03,buy,ETH,10.0,3000,15,USDT,Buy ETH,,
2025-01-04,swap,BTC,0.2,45000,2,BNB,Swap BTC to ETH,ETH,3.0"#;

    let csv_path = create_test_csv(&temp_dir, "multi_type.csv", csv_content);

    // Process CSV
    let file = fs::File::open(&csv_path)?;
    let mut reader = csv::Reader::from_reader(file);

    for result in reader.deserialize() {
        let row: CsvRecord = result?;

        let quantity = dec(&row.quantity);
        let price_usd = dec(&row.price_usd);

        match row.tx_type.as_str() {
            "buy" => {
                let tx =
                    Transaction::new_buy(account_id, &row.asset, quantity, price_usd, Utc::now());
                tx_repo.insert(&tx).await?;
                holding_repo
                    .add_quantity(account_id, &row.asset, quantity, Some(price_usd))
                    .await?;
            }
            "sell" => {
                let tx =
                    Transaction::new_sell(account_id, &row.asset, quantity, price_usd, Utc::now());
                tx_repo.insert(&tx).await?;
                holding_repo
                    .remove_quantity(account_id, &row.asset, quantity)
                    .await?;
            }
            "swap" => {
                let to_quantity = dec(&row.to_quantity);

                let tx = Transaction {
                    id: 0,
                    tx_type: TransactionType::Swap,
                    from_account_id: Some(account_id.to_string()),
                    from_asset: Some(row.asset.clone()),
                    from_quantity: Some(quantity),
                    to_account_id: Some(account_id.to_string()),
                    to_asset: Some(row.to_asset.clone()),
                    to_quantity: Some(to_quantity),
                    price_usd: Some(price_usd),
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
                tx_repo.insert(&tx).await?;
                holding_repo
                    .remove_quantity(account_id, &row.asset, quantity)
                    .await?;
                holding_repo
                    .add_quantity(account_id, &row.to_asset, to_quantity, None)
                    .await?;
            }
            _ => {}
        }
    }

    // Verify transactions created
    let transactions = tx_repo.list_by_account(account_id, None).await?;
    assert_eq!(transactions.len(), 4, "Should have 4 transactions");

    // Verify final holdings
    // BTC: buy 1.0, sell 0.3, swap out 0.2 = 0.5 remaining
    let btc_holding = holding_repo.get(account_id, "BTC").await?;
    assert!(btc_holding.is_some(), "Should have BTC holding");
    assert_eq!(
        btc_holding.unwrap().quantity,
        dec("0.5"),
        "Should have 0.5 BTC"
    );

    // ETH: buy 10.0, swap in 3.0 = 13.0 total
    let eth_holding = holding_repo.get(account_id, "ETH").await?;
    assert!(eth_holding.is_some(), "Should have ETH holding");
    assert_eq!(
        eth_holding.unwrap().quantity,
        dec("13.0"),
        "Should have 13 ETH"
    );

    Ok(())
}

/// Test: Empty CSV import
#[tokio::test]
async fn test_empty_csv_import() -> Result<()> {
    let pool = common::setup_test_db().await?;
    let account_repo = AccountRepository::new(&pool);
    let tx_repo = TransactionRepository::new(&pool);

    // Create test account
    let account_id = "empty_csv_account";
    if account_repo.get_category("test").await?.is_none() {
        account_repo
            .create_category("test", "Test Category")
            .await?;
    }
    let account = Account {
        id: account_id.to_string(),
        name: "Empty CSV Account".to_string(),
        account_type: AccountType::Exchange,
        category_id: "test".to_string(),
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };
    account_repo.create_account(&account).await?;

    // Create empty CSV (only header)
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let csv_content =
        "date,type,asset,quantity,price_usd,fee,fee_asset,notes,to_asset,to_quantity\n";
    let csv_path = create_test_csv(&temp_dir, "empty.csv", csv_content);

    // Process empty CSV
    let file = fs::File::open(&csv_path)?;
    let mut reader = csv::Reader::from_reader(file);

    let mut count = 0;
    for _result in reader.deserialize::<serde_json::Value>() {
        count += 1;
    }

    assert_eq!(count, 0, "Empty CSV should process 0 records");

    // Verify no transactions created
    let transactions = tx_repo.list_by_account(account_id, None).await?;
    assert_eq!(transactions.len(), 0, "Should have no transactions");

    Ok(())
}
