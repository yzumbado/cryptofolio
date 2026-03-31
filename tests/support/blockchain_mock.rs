use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mock Blockstream API server for testing
pub struct BlockstreamMock {
    pub server: MockServer,
}

impl BlockstreamMock {
    /// Create a new mock Blockstream server
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        Self { server }
    }

    /// Get the mock server URL
    pub fn url(&self) -> String {
        self.server.uri()
    }

    /// Mock address info endpoint with specified balance
    pub async fn mock_address_info(&self, address: &str, balance_btc: f64, tx_count: u64) {
        let balance_sats = (balance_btc * 100_000_000.0) as i64;

        Mock::given(method("GET"))
            .and(path(format!("/address/{}", address)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "chain_stats": {
                    "funded_txo_sum": balance_sats,
                    "spent_txo_sum": 0,
                    "tx_count": tx_count
                }
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock address with no balance
    pub async fn mock_empty_address(&self, address: &str) {
        Mock::given(method("GET"))
            .and(path(format!("/address/{}", address)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "chain_stats": {
                    "funded_txo_sum": 0,
                    "spent_txo_sum": 0,
                    "tx_count": 0
                }
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock transactions endpoint with specified transaction list
    pub async fn mock_transactions(
        &self,
        address: &str,
        incoming: i64,
        outgoing: i64,
        count: usize,
    ) {
        let mut txs = Vec::new();

        for i in 0..count {
            txs.push(json!({
                "txid": format!("mock_tx_{}", i),
                "fee": 1500,
                "status": {
                    "confirmed": true,
                    "block_height": 800000 + i,
                    "block_time": 1709251200 + (i * 600) as i64
                },
                "vin": [],
                "vout": [{
                    "scriptpubkey_address": address,
                    "value": incoming
                }]
            }));
        }

        Mock::given(method("GET"))
            .and(path(format!("/address/{}/txs", address)))
            .respond_with(ResponseTemplate::new(200).set_body_json(txs))
            .mount(&self.server)
            .await;
    }

    /// Mock empty transaction list
    pub async fn mock_empty_transactions(&self, address: &str) {
        Mock::given(method("GET"))
            .and(path(format!("/address/{}/txs", address)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&self.server)
            .await;
    }

    /// Mock API error (e.g., rate limit)
    pub async fn mock_error(&self, address: &str, status_code: u16) {
        Mock::given(method("GET"))
            .and(path(format!("/address/{}", address)))
            .respond_with(ResponseTemplate::new(status_code))
            .mount(&self.server)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_creation() {
        let mock = BlockstreamMock::new().await;
        assert!(!mock.url().is_empty());
    }

    #[tokio::test]
    async fn test_mock_address_info() {
        let mock = BlockstreamMock::new().await;
        let address = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";

        mock.mock_address_info(address, 0.5, 10).await;

        let url = format!("{}/address/{}", mock.url(), address);
        let response = reqwest::get(&url).await.unwrap();
        assert_eq!(response.status(), 200);

        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["chain_stats"]["funded_txo_sum"], 50_000_000);
        assert_eq!(body["chain_stats"]["tx_count"], 10);
    }
}
