use cucumber::given;
use crate::support::world::CryptofolioWorld;

/// Ethereum-specific step definitions

#[given(expr = "the Ethereum blockchain shows balance of {float} ETH")]
async fn mock_ethereum_balance(world: &mut CryptofolioWorld, balance: f64) {
    // Setup blockchain mock if not already done
    if world.ethereum_mock.is_none() {
        world.setup_ethereum_mock().await.expect("Failed to setup Ethereum mock");
    }

    // Mock the address info for the test wallet
    let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
    if let Some(mock) = &world.ethereum_mock {
        mock.mock_balance(address, balance).await;
        mock.mock_no_tokens(address).await;
    }
}

#[given(expr = "the Ethereum blockchain shows {int} ERC-20 tokens")]
async fn mock_erc20_tokens(world: &mut CryptofolioWorld, token_count: u32) {
    // Setup blockchain mock if not already done
    if world.ethereum_mock.is_none() {
        world.setup_ethereum_mock().await.expect("Failed to setup Ethereum mock");
    }

    let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
    if let Some(mock) = &world.ethereum_mock {
        // Always mock ETH balance (required for get_address_info)
        mock.mock_balance(address, 1.0).await;

        if token_count > 0 {
            // Mock common tokens: USDT, USDC, DAI
            let mut tokens = vec![];
            if token_count >= 1 {
                tokens.push(("USDT", "Tether USD", 1000.0, 6)); // 6 decimals
            }
            if token_count >= 2 {
                tokens.push(("USDC", "USD Coin", 500.0, 6));
            }
            if token_count >= 3 {
                tokens.push(("DAI", "Dai Stablecoin", 250.0, 18));
            }
            mock.mock_tokens(address, &tokens).await;
        } else {
            mock.mock_no_tokens(address).await;
        }
    }
}

#[given(expr = "the wallet has {float} {word} ERC-20 tokens")]
async fn mock_specific_ethereum_token(world: &mut CryptofolioWorld, amount: f64, symbol: String) {
    // Setup blockchain mock if not already done
    if world.ethereum_mock.is_none() {
        world.setup_ethereum_mock().await.expect("Failed to setup Ethereum mock");
    }

    let decimals = match symbol.as_str() {
        "USDT" | "USDC" => 6,
        "DAI" | "WETH" => 18,
        _ => 18, // Default to 18 decimals
    };

    let name = match symbol.as_str() {
        "USDT" => "Tether USD",
        "USDC" => "USD Coin",
        "DAI" => "Dai Stablecoin",
        "WETH" => "Wrapped Ether",
        _ => &symbol,
    };

    // Accumulate tokens instead of mocking immediately
    world.accumulated_tokens.push((symbol.clone(), name.to_string(), amount, decimals));
}

#[given(expr = "the blockchain shows {int} incoming and {int} outgoing transactions")]
async fn mock_eth_transactions(world: &mut CryptofolioWorld, incoming: u32, outgoing: u32) {
    // Setup blockchain mock if not already done
    if world.ethereum_mock.is_none() {
        world.setup_ethereum_mock().await.expect("Failed to setup Ethereum mock");
    }

    let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
    if let Some(mock) = &world.ethereum_mock {
        // Mock ETH balance
        mock.mock_balance(address, 2.5).await;
        // Mock no tokens (this scenario is about transactions, not tokens)
        mock.mock_no_tokens(address).await;
        // Mock transactions
        mock.mock_transactions(address, (incoming + outgoing) as usize).await;
    }
}
