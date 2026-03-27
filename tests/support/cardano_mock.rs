use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use serde_json::json;

/// Mock Blockfrost/Cardano API server for testing
pub struct CardanoMock {
    pub server: MockServer,
}

impl CardanoMock {
    /// Create a new mock Cardano server
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        Self { server }
    }

    /// Get the mock server URL
    pub fn url(&self) -> String {
        format!("{}/api/v0", self.server.uri())
    }

    /// Mock ADA balance for an address
    pub async fn mock_balance(&self, address: &str, balance_ada: f64) {
        let balance_lovelace = (balance_ada * 1_000_000.0) as i64;

        // Blockfrost API format: GET /addresses/{address}
        Mock::given(method("GET"))
            .and(path(format!("/api/v0/addresses/{}", address)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "address": address,
                "amount": [
                    {
                        "unit": "lovelace",
                        "quantity": balance_lovelace.to_string()
                    }
                ],
                "tx_count": 10
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock native token balance list for an address
    pub async fn mock_tokens(&self, address: &str, tokens: &[(&str, &str, u8)]) {
        let mut amount_list = vec![
            json!({
                "unit": "lovelace",
                "quantity": "100000000"  // 100 ADA
            })
        ];

        for (idx, (symbol, quantity, _decimals)) in tokens.iter().enumerate() {
            // Generate fake policy ID + asset name
            let policy_id = format!("{:0<56}", idx + 1);
            let asset_name_hex = hex::encode(symbol);
            let unit = format!("{}{}", policy_id, asset_name_hex);

            amount_list.push(json!({
                "unit": unit,
                "quantity": quantity
            }));

            // Mock token metadata endpoint
            Mock::given(method("GET"))
                .and(path(format!("/api/v0/assets/{}", unit)))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "asset_name": symbol,
                    "onchain_metadata": {
                        "name": symbol,
                        "decimals": _decimals
                    }
                })))
                .mount(&self.server)
                .await;
        }

        // Mock the /addresses/{address}/total endpoint
        Mock::given(method("GET"))
            .and(path(format!("/api/v0/addresses/{}/total", address)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "address": address,
                "amount": amount_list,
                "tx_count": 10
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock no tokens for an address
    pub async fn mock_no_tokens(&self, address: &str) {
        Mock::given(method("GET"))
            .and(path(format!("/api/v0/addresses/{}/total", address)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "address": address,
                "amount": [
                    {
                        "unit": "lovelace",
                        "quantity": "100000000"
                    }
                ],
                "tx_count": 10
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock transaction list for an address
    pub async fn mock_transactions(&self, address: &str, tx_count: usize) {
        let mut txs = Vec::new();

        for i in 0..tx_count {
            let tx_hash = format!("{:064x}", i);
            txs.push(json!({
                "tx_hash": tx_hash,
                "block": format!("{:064x}", i + 1000000),
                "block_height": 8000000 + i,
                "block_time": 1709251200 + (i * 20) as i64,
                "slot": 100000000 + (i * 20),
                "index": i
            }));

            // Mock individual transaction detail
            Mock::given(method("GET"))
                .and(path(format!("/api/v0/txs/{}", tx_hash)))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "hash": tx_hash,
                    "block": format!("{:064x}", i + 1000000),
                    "block_height": 8000000 + i,
                    "block_time": 1709251200 + (i * 20) as i64,
                    "slot": 100000000 + (i * 20),
                    "index": i,
                    "fees": "170000",       // 0.17 ADA
                    "deposit": "0",
                    "size": 350
                })))
                .mount(&self.server)
                .await;
        }

        Mock::given(method("GET"))
            .and(path(format!("/api/v0/addresses/{}/transactions", address)))
            .respond_with(ResponseTemplate::new(200).set_body_json(txs))
            .mount(&self.server)
            .await;
    }

    /// Mock stake pool delegation for an address
    pub async fn mock_delegation(&self, address: &str, pool_ticker: &str) {
        // Mock stake address endpoint
        let stake_address = format!("stake1u{}", &address[6..]);

        Mock::given(method("GET"))
            .and(path(format!("/api/v0/accounts/{}", stake_address)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "stake_address": stake_address,
                "active": true,
                "pool_id": "pool1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq",
                "controlled_amount": "1000000000",  // 1000 ADA
                "rewards_sum": "50000000",           // 50 ADA in rewards
                "withdrawals_sum": "0"
            })))
            .mount(&self.server)
            .await;

        // Mock pool info endpoint
        Mock::given(method("GET"))
            .and(path("/api/v0/pools/pool1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "pool_id": "pool1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq",
                "hex": "0000000000000000000000000000000000000000000000000000000000000000",
                "vrf_key": "vrf_vk1...",
                "blocks_minted": 1000,
                "live_stake": "50000000000000",
                "live_size": 0.001,
                "live_saturation": 0.5,
                "live_delegators": 1000,
                "active_stake": "48000000000000",
                "active_size": 0.0009,
                "declared_pledge": "1000000000000",
                "live_pledge": "1000000000000",
                "margin_cost": 0.03,
                "fixed_cost": "340000000",
                "reward_account": "stake1u...",
                "owners": ["stake1u..."],
                "registration": ["pool_registration"],
                "retirement": []
            })))
            .mount(&self.server)
            .await;

        // Mock pool metadata endpoint
        Mock::given(method("GET"))
            .and(path("/api/v0/pools/pool1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq/metadata"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "pool_id": "pool1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq",
                "hex": "0000000000000000000000000000000000000000000000000000000000000000",
                "url": "https://example.com/pool.json",
                "hash": "0000000000000000000000000000000000000000000000000000000000000000",
                "ticker": pool_ticker,
                "name": format!("{} Stake Pool", pool_ticker),
                "description": format!("Test stake pool {}", pool_ticker),
                "homepage": "https://example.com"
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock API error (e.g., rate limit)
    pub async fn mock_error(&self, status_code: u16) {
        Mock::given(method("GET"))
            .and(path("/api/v0"))
            .respond_with(ResponseTemplate::new(status_code))
            .mount(&self.server)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cardano_mock_creation() {
        let mock = CardanoMock::new().await;
        assert!(mock.url().contains("/api/v0"));
    }
}
