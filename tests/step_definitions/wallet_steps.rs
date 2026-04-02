use crate::support::world::CryptofolioWorld;
use cucumber::{given, then};

/// Wallet-specific step definitions

#[given(expr = "the Bitcoin blockchain shows balance of {float} BTC")]
async fn mock_bitcoin_balance(world: &mut CryptofolioWorld, balance: f64) {
    // Setup blockchain mock if not already done
    if world.blockchain_mock.is_none() {
        world
            .setup_blockchain_mock()
            .await
            .expect("Failed to setup blockchain mock");
    }

    // Mock the address info for the test wallet
    let address = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";
    if let Some(mock) = &world.blockchain_mock {
        mock.mock_address_info(address, balance, 10).await;
        mock.mock_empty_transactions(address).await;
    }
}

#[given(expr = "I have added a Bitcoin wallet {string}")]
async fn add_bitcoin_wallet(world: &mut CryptofolioWorld, wallet_name: String) {
    use chrono::Utc;
    use cryptofolio::core::account::{Account, AccountConfig, AccountType};
    use cryptofolio::db::accounts::AccountRepository;

    let pool = world.pool();
    let repo = AccountRepository::new(pool);

    // Create account (use name as ID for simplicity in tests)
    let account = Account {
        id: wallet_name.clone(),
        name: wallet_name.clone(),
        category_id: "cold-storage".to_string(),
        account_type: AccountType::SoftwareWallet,
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };

    repo.create_account(&account)
        .await
        .expect("Failed to create account");

    // Add Bitcoin address
    repo.add_address(
        &wallet_name,
        "bitcoin",
        "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
        None,
    )
    .await
    .expect("Failed to add address");

    world.last_account_id = Some(wallet_name);
}

#[given(expr = "I have added an Ethereum wallet {string}")]
async fn add_ethereum_wallet(world: &mut CryptofolioWorld, wallet_name: String) {
    use chrono::Utc;
    use cryptofolio::core::account::{Account, AccountConfig, AccountType};
    use cryptofolio::db::accounts::AccountRepository;

    let pool = world.pool();
    let repo = AccountRepository::new(pool);

    // Create account (use name as ID for simplicity in tests)
    let account = Account {
        id: wallet_name.clone(),
        name: wallet_name.clone(),
        category_id: "cold-storage".to_string(),
        account_type: AccountType::SoftwareWallet,
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };

    repo.create_account(&account)
        .await
        .expect("Failed to create account");

    // Add Ethereum address
    repo.add_address(
        &wallet_name,
        "ethereum",
        "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
        None,
    )
    .await
    .expect("Failed to add address");

    world.last_account_id = Some(wallet_name);
}

#[given(expr = "I have added a Cardano wallet {string}")]
async fn add_cardano_wallet(world: &mut CryptofolioWorld, wallet_name: String) {
    use chrono::Utc;
    use cryptofolio::core::account::{Account, AccountConfig, AccountType};
    use cryptofolio::db::accounts::AccountRepository;

    let pool = world.pool();
    let repo = AccountRepository::new(pool);

    let account = Account {
        id: wallet_name.clone(),
        name: wallet_name.clone(),
        category_id: "cold-storage".to_string(),
        account_type: AccountType::SoftwareWallet,
        config: AccountConfig::default(),
        sync_enabled: false,
        created_at: Utc::now(),
    };

    repo.create_account(&account)
        .await
        .expect("Failed to create account");

    // Add Cardano address
    repo.add_address(
        &wallet_name,
        "cardano",
        "addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgse35a3x",
        None,
    ).await.expect("Failed to add address");

    world.last_account_id = Some(wallet_name);
}

#[then(expr = "the wallet balance should be {string} BTC")]
async fn verify_btc_balance(world: &mut CryptofolioWorld, expected: String) {
    use cryptofolio::db::holdings::HoldingRepository;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let pool = world.pool();
    let repo = HoldingRepository::new(pool);
    let all = repo.list_all().await.expect("Failed to query holdings");

    let btc_qty: Decimal = all
        .iter()
        .filter(|h| h.asset == "BTC")
        .map(|h| h.quantity)
        .sum();

    let expected_qty = Decimal::from_str(&expected).expect("Invalid decimal in test");
    assert_eq!(
        btc_qty, expected_qty,
        "Expected BTC balance {}, got {}",
        expected_qty, btc_qty
    );
}

#[then(expr = "the wallet balance should be {string} ETH")]
async fn verify_eth_balance(world: &mut CryptofolioWorld, expected: String) {
    use cryptofolio::db::holdings::HoldingRepository;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let pool = world.pool();
    let repo = HoldingRepository::new(pool);
    let all = repo.list_all().await.expect("Failed to query holdings");

    let eth_qty: Decimal = all
        .iter()
        .filter(|h| h.asset == "ETH")
        .map(|h| h.quantity)
        .sum();

    let expected_qty = Decimal::from_str(&expected).expect("Invalid decimal in test");
    assert_eq!(
        eth_qty, expected_qty,
        "Expected ETH balance {}, got {}",
        expected_qty, eth_qty
    );
}

#[then(expr = "I should have {int} wallets")]
async fn verify_wallet_count(world: &mut CryptofolioWorld, expected_count: u32) {
    use cryptofolio::core::account::AccountType;
    use cryptofolio::db::accounts::AccountRepository;

    let pool = world.pool();
    let repo = AccountRepository::new(pool);
    let accounts = repo.list_accounts().await.expect("Failed to list accounts");

    let wallet_count = accounts
        .iter()
        .filter(|a| {
            matches!(
                a.account_type,
                AccountType::SoftwareWallet | AccountType::HardwareWallet
            )
        })
        .count() as u32;

    assert_eq!(
        wallet_count, expected_count,
        "Expected {} wallets, found {}",
        expected_count, wallet_count
    );
}

#[then(expr = "the wallet should be for blockchain {string}")]
async fn verify_blockchain(world: &mut CryptofolioWorld, expected_blockchain: String) {
    use cryptofolio::db::accounts::AccountRepository;

    let pool = world.pool();
    let repo = AccountRepository::new(pool);
    let accounts = repo.list_accounts().await.expect("Failed to list accounts");

    for account in &accounts {
        let addresses = repo
            .list_addresses(&account.id)
            .await
            .expect("Failed to list addresses");
        let has_chain = addresses
            .iter()
            .any(|a| a.blockchain.eq_ignore_ascii_case(&expected_blockchain));
        if has_chain {
            return; // found at least one address on this chain
        }
    }

    panic!(
        "No wallet address found for blockchain '{}'",
        expected_blockchain
    );
}
