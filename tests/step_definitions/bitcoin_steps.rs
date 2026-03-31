use cucumber::given;
use crate::support::world::CryptofolioWorld;

/// Bitcoin-specific step definitions

#[given(expr = "the blockchain shows {int} incoming and {int} outgoing Bitcoin transactions")]
async fn mock_bitcoin_transactions(world: &mut CryptofolioWorld, incoming: u32, outgoing: u32) {
    // Setup blockchain mock if not already done
    if world.blockchain_mock.is_none() {
        world.setup_blockchain_mock().await.expect("Failed to setup Bitcoin mock");
    }

    let address = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";
    let total_count = (incoming + outgoing) as usize;

    if let Some(mock) = &world.blockchain_mock {
        // Mock address info endpoint (required before transactions)
        mock.mock_address_info(address, 0.5, total_count as u64).await;

        // Mock transactions endpoint
        mock.mock_transactions(address, incoming as i64, outgoing as i64, total_count).await;
    }
}
