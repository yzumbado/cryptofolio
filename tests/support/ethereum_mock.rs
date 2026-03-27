use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, query_param};
use serde_json::json;

/// Mock Etherscan/Ethereum API server for testing
pub struct EthereumMock {
    pub server: MockServer,
}

impl EthereumMock {
    /// Create a new mock Ethereum server
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        Self { server }
    }

    /// Get the mock server URL
    pub fn url(&self) -> String {
        self.server.uri()
    }

    /// Mock ETH balance for an address
    pub async fn mock_balance(&self, address: &str, balance_eth: f64) {
        let balance_wei = format!("{}", (balance_eth * 1_000_000_000_000_000_000.0) as u128);

        // Etherscan API format: ?module=account&action=balance&address=...
        Mock::given(method("GET"))
            .and(path("/api"))
            .and(query_param("module", "account"))
            .and(query_param("action", "balance"))
            .and(query_param("address", address))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "1",
                "message": "OK",
                "result": balance_wei
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock ERC-20 token balance list for an address
    pub async fn mock_tokens(&self, address: &str, tokens: &[(&str, &str, f64, u8)]) {
        let mut token_list = Vec::new();

        for (idx, (symbol, name, balance, decimals)) in tokens.iter().enumerate() {
            let balance_raw = (balance * 10_f64.powi(*decimals as i32)) as u128;
            // Use index to generate unique contract addresses
            let contract_address = format!("0x{:040x}", idx + 1);

            // Create a token transaction where the address receives the tokens
            token_list.push(json!({
                "tokenSymbol": symbol,
                "tokenName": name,
                "tokenDecimal": decimals.to_string(),
                "value": balance_raw.to_string(),
                "contractAddress": contract_address,
                "from": "0x0000000000000000000000000000000000000000", // From zero address
                "to": address, // To the test address (receiving)
            }));
        }

        Mock::given(method("GET"))
            .and(path("/api"))
            .and(query_param("module", "account"))
            .and(query_param("action", "tokentx"))
            .and(query_param("address", address))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "1",
                "message": "OK",
                "result": token_list
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock single token balance
    pub async fn mock_single_token(&self, address: &str, symbol: &str, name: &str, balance: f64, decimals: u8) {
        self.mock_tokens(address, &[(symbol, name, balance, decimals)]).await;
    }

    /// Mock no tokens for an address
    pub async fn mock_no_tokens(&self, address: &str) {
        Mock::given(method("GET"))
            .and(path("/api"))
            .and(query_param("module", "account"))
            .and(query_param("action", "tokentx"))
            .and(query_param("address", address))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "1",
                "message": "OK",
                "result": []
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock transaction list for an address
    pub async fn mock_transactions(&self, address: &str, tx_count: usize) {
        let mut txs = Vec::new();

        for i in 0..tx_count {
            txs.push(json!({
                "hash": format!("0x{:064x}", i),
                "from": "0x0000000000000000000000000000000000000000",
                "to": address,
                "value": "1000000000000000000", // 1 ETH in wei
                "gas": "21000",
                "gasPrice": "20000000000", // 20 Gwei
                "gasUsed": "21000",
                "timeStamp": (1709251200 + i * 3600).to_string(),
                "isError": "0",
                "blockNumber": (18000000 + i).to_string()
            }));
        }

        Mock::given(method("GET"))
            .and(path("/api"))
            .and(query_param("module", "account"))
            .and(query_param("action", "txlist"))
            .and(query_param("address", address))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "1",
                "message": "OK",
                "result": txs
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock API error (e.g., rate limit)
    pub async fn mock_error(&self, status_code: u16) {
        Mock::given(method("GET"))
            .and(path("/api"))
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
        let mock = EthereumMock::new().await;
        assert!(!mock.url().is_empty());
    }

    #[tokio::test]
    async fn test_mock_balance() {
        let mock = EthereumMock::new().await;
        let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";

        mock.mock_balance(address, 1.5).await;

        let url = format!(
            "{}/api?module=account&action=balance&address={}",
            mock.url(),
            address
        );
        let response = reqwest::get(&url).await.unwrap();
        assert_eq!(response.status(), 200);

        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["status"], "1");
        // 1.5 ETH = 1,500,000,000,000,000,000 wei
        assert_eq!(body["result"], "1500000000000000000");
    }

    #[tokio::test]
    async fn test_mock_tokens() {
        let mock = EthereumMock::new().await;
        let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";

        mock.mock_tokens(address, &[
            ("USDT", "Tether USD", 1000.0, 6),
            ("USDC", "USD Coin", 500.0, 6),
        ]).await;

        let url = format!(
            "{}/api?module=account&action=tokentx&address={}",
            mock.url(),
            address
        );
        let response = reqwest::get(&url).await.unwrap();
        assert_eq!(response.status(), 200);

        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["result"].as_array().unwrap().len(), 2);
        assert_eq!(body["result"][0]["tokenSymbol"], "USDT");
        assert_eq!(body["result"][1]["tokenSymbol"], "USDC");
    }
}
