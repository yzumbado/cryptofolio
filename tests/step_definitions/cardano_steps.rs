use cucumber::given;
use crate::support::world::CryptofolioWorld;

/// Cardano-specific step definitions

#[given(expr = "the blockchain shows {int} incoming and {int} outgoing Cardano transactions")]
async fn mock_cardano_transactions(world: &mut CryptofolioWorld, incoming: u32, outgoing: u32) {
    // Setup blockchain mock if not already done
    if world.cardano_mock.is_none() {
        world.setup_cardano_mock().await.expect("Failed to setup Cardano mock");
    }

    let address = "addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qvwa";
    if let Some(mock) = &world.cardano_mock {
        // Mock ADA balance
        mock.mock_balance(address, 100.0).await;
        // Mock no tokens
        mock.mock_no_tokens(address).await;
        // Mock transactions
        mock.mock_transactions(address, (incoming + outgoing) as usize).await;
    }
}

#[given(expr = "the Cardano blockchain shows balance of {float} ADA")]
async fn mock_cardano_balance(world: &mut CryptofolioWorld, balance: f64) {
    // Setup blockchain mock if not already done
    if world.cardano_mock.is_none() {
        world.setup_cardano_mock().await.expect("Failed to setup Cardano mock");
    }

    // Mock the address info for the test wallet
    let address = "addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qvwa";
    if let Some(mock) = &world.cardano_mock {
        mock.mock_balance(address, balance).await;
        mock.mock_no_tokens(address).await;
    }
}

#[given(expr = "the Cardano blockchain shows {int} native tokens")]
async fn mock_native_tokens(world: &mut CryptofolioWorld, token_count: u32) {
    // Setup blockchain mock if not already done
    if world.cardano_mock.is_none() {
        world.setup_cardano_mock().await.expect("Failed to setup Cardano mock");
    }

    let address = "addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qvwa";
    if let Some(mock) = &world.cardano_mock {
        // Always mock ADA balance
        mock.mock_balance(address, 100.0).await;

        if token_count > 0 {
            // Mock common Cardano native tokens
            let mut tokens = vec![];
            if token_count >= 1 {
                tokens.push(("HOSKY", "1000000.0", 0)); // 0 decimals
            }
            if token_count >= 2 {
                tokens.push(("MIN", "50.0", 0));
            }
            if token_count >= 3 {
                tokens.push(("SUNDAE", "100.0", 6));
            }
            mock.mock_tokens(address, &tokens).await;
        } else {
            mock.mock_no_tokens(address).await;
        }
    }
}

#[given(expr = "the wallet has {float} {word} native tokens")]
async fn mock_specific_cardano_token(world: &mut CryptofolioWorld, amount: f64, symbol: String) {
    // Setup blockchain mock if not already done
    if world.cardano_mock.is_none() {
        world.setup_cardano_mock().await.expect("Failed to setup Cardano mock");
    }

    let decimals = match symbol.as_str() {
        "HOSKY" => 0,
        "MIN" => 0,
        "SUNDAE" => 6,
        "WMT" => 6,
        _ => 0, // Default to 0 decimals for Cardano native tokens
    };

    // Accumulate tokens instead of mocking immediately
    world.accumulated_cardano_tokens.push((symbol.clone(), format!("{}", amount), decimals));
}

#[given(expr = "the wallet is delegated to stake pool {string}")]
async fn mock_stake_delegation(world: &mut CryptofolioWorld, pool_ticker: String) {
    // Setup blockchain mock if not already done
    if world.cardano_mock.is_none() {
        world.setup_cardano_mock().await.expect("Failed to setup Cardano mock");
    }

    let address = "addr1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh0tcp5dc2ukmuqjjw0apg6k8xfn63t8y9p2l3w8w5z2x7jn8sqf3qvwa";
    if let Some(mock) = &world.cardano_mock {
        // Mock ADA balance
        mock.mock_balance(address, 1000.0).await;
        // Mock no tokens
        mock.mock_no_tokens(address).await;
        // Mock stake delegation
        mock.mock_delegation(address, &pool_ticker).await;
    }
}
