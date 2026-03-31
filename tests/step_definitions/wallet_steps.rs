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
        "addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qvwa",
        None,
    ).await.expect("Failed to add address");

    world.last_account_id = Some(wallet_name);
}

#[then(expr = "the wallet balance should be {string} BTC")]
async fn verify_btc_balance(_world: &mut CryptofolioWorld, _expected: String) {
    // TODO: Query wallet balance from database
    // For now, just pass - will implement with actual repository
}

#[then(expr = "the wallet balance should be {string} ETH")]
async fn verify_eth_balance(_world: &mut CryptofolioWorld, _expected: String) {
    // TODO: Query wallet balance from database
}

#[then(expr = "I should have {int} wallets")]
async fn verify_wallet_count(_world: &mut CryptofolioWorld, _count: u32) {
    // TODO: Query wallet count from database
}

#[then(expr = "the wallet should be for blockchain {string}")]
async fn verify_blockchain(_world: &mut CryptofolioWorld, _blockchain: String) {
    // TODO: Verify blockchain field
}
