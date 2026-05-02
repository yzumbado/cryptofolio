use crate::blockchain::trait_def::BlockchainClient;
use crate::blockchain::types::{
    AddressSummary, Chain, HealthStatus, TransactionDirection, WalletBalance, WalletTransaction,
};
use crate::error::{CryptofolioError, Result};
/// Ethereum blockchain clients (Etherscan API)
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// ERC-20 Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ERC20Token {
    pub symbol: String,
    pub name: String,
    pub contract_address: String,
    pub balance: Decimal,
    pub decimals: u8,
}

/// Ethereum transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumTransaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value: Decimal, // In ETH
    pub gas_used: u64,
    pub gas_price: Decimal, // In Gwei
    pub timestamp: i64,
    pub block_number: u64,
    pub is_error: bool,
}

/// Address information from blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    pub address: String,
    pub balance: Decimal, // In ETH
    pub tokens: Vec<ERC20Token>,
}

/// Etherscan API client
pub struct EtherscanClient {
    base_url: String,
    chain_id: u64,
    api_key: Option<String>,
}

impl EtherscanClient {
    /// Create a new Etherscan V2 client
    /// V2 API: https://api.etherscan.io/v2/api?chainid=<id>&...
    pub fn new(testnet: bool, api_key: Option<String>) -> Self {
        // Etherscan V2 unified endpoint — chain selected via chainid param
        let base_url = "https://api.etherscan.io/v2/api".to_string();
        let chain_id = if testnet {
            11155111 // Sepolia
        } else {
            1 // Ethereum Mainnet
        };

        Self {
            base_url,
            chain_id,
            api_key,
        }
    }

    /// Create a client with custom base URL (for testing)
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            base_url,
            chain_id: 1,
            api_key: None,
        }
    }

    /// Get address balance and token information
    pub async fn get_address_info(&self, address: &str) -> Result<AddressInfo> {
        // Get ETH balance
        let balance = self.get_eth_balance(address).await?;

        // Get ERC-20 tokens
        let tokens = self.get_erc20_tokens(address).await?;

        Ok(AddressInfo {
            address: address.to_string(),
            balance,
            tokens,
        })
    }

    /// Get ETH balance for an address
    async fn get_eth_balance(&self, address: &str) -> Result<Decimal> {
        let mut url = format!(
            "{}?chainid={}&module=account&action=balance&address={}&tag=latest",
            self.base_url, self.chain_id, address
        );

        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let response = reqwest::get(&url)
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to fetch balance: {}", e)))?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Etherscan API error: {}",
                response.status()
            )));
        }

        let data: EtherscanResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        if data.status != "1" {
            return Err(CryptofolioError::Network(format!(
                "Etherscan API error: {}",
                data.message
            )));
        }

        // Convert wei to ETH
        let wei = Decimal::from_str(&data.result)
            .map_err(|e| CryptofolioError::Other(format!("Invalid balance format: {}", e)))?;
        let eth = wei / Decimal::from(1_000_000_000_000_000_000u64);

        Ok(eth)
    }

    /// Get ERC-20 tokens for an address
    async fn get_erc20_tokens(&self, address: &str) -> Result<Vec<ERC20Token>> {
        let mut url = format!(
            "{}?chainid={}&module=account&action=tokentx&address={}&startblock=0&endblock=99999999&sort=asc",
            self.base_url, self.chain_id, address
        );

        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let response = reqwest::get(&url)
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to fetch tokens: {}", e)))?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Etherscan API error: {}",
                response.status()
            )));
        }

        let data: EtherscanTokenResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        if data.status != "1" {
            // No transactions or no tokens is valid - return empty list
            if data.message.contains("No transactions found") {
                return Ok(Vec::new());
            }
            return Err(CryptofolioError::Network(format!(
                "Etherscan API error: {}",
                data.message
            )));
        }

        // Aggregate token balances from transactions
        let mut token_map: std::collections::HashMap<String, (String, String, i128, u8)> =
            std::collections::HashMap::new();

        for tx in data.result {
            let decimals = tx.token_decimal.parse::<u8>().unwrap_or(18);
            let value = i128::from_str(&tx.value).unwrap_or(0);

            let entry = token_map.entry(tx.contract_address.clone()).or_insert((
                tx.token_symbol,
                tx.token_name,
                0,
                decimals,
            ));

            // Add to balance if receiving, subtract if sending
            if tx.to.eq_ignore_ascii_case(address) {
                entry.2 += value;
            } else if tx.from.eq_ignore_ascii_case(address) {
                entry.2 -= value;
            }
        }

        // Convert to token list (filter out zero balances)
        let mut tokens: Vec<ERC20Token> = token_map
            .into_iter()
            .filter(|(_, (_, _, balance, _))| *balance > 0)
            .filter_map(|(contract, (symbol, name, balance, decimals))| {
                // Use from_str to avoid panicking on very large token values
                let raw = Decimal::from_str(&balance.to_string()).ok()?;
                let divisor = Decimal::from_str(&format!("1{}", "0".repeat(decimals as usize)))
                    .unwrap_or(Decimal::ONE);
                let token_balance = raw / divisor;

                Some(ERC20Token {
                    symbol,
                    name,
                    contract_address: contract,
                    balance: token_balance,
                    decimals,
                })
            })
            .collect();

        // Sort by symbol for consistent output
        tokens.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        Ok(tokens)
    }

    /// Get transactions for an address, optionally starting from `start_block`.
    async fn fetch_transactions_since(
        &self,
        address: &str,
        start_block: u64,
    ) -> Result<Vec<EthereumTransaction>> {
        let mut url = format!(
            "{}?chainid={}&module=account&action=txlist&address={}&startblock={}&endblock=99999999&sort=asc",
            self.base_url, self.chain_id, address, start_block
        );

        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let response = reqwest::get(&url).await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to fetch transactions: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Etherscan API error: {}",
                response.status()
            )));
        }

        let data: EtherscanTxResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        if data.status != "1" {
            if data.message.contains("No transactions found") {
                return Ok(Vec::new());
            }
            return Err(CryptofolioError::Network(format!(
                "Etherscan API error: {}",
                data.message
            )));
        }

        let mut result = Vec::new();
        for tx in data.result {
            let value_wei = Decimal::from_str(&tx.value).unwrap_or(Decimal::ZERO);
            let value_eth = value_wei / Decimal::from(1_000_000_000_000_000_000u64);

            let gas_price_wei = Decimal::from_str(&tx.gas_price).unwrap_or(Decimal::ZERO);
            let gas_price_gwei = gas_price_wei / Decimal::from(1_000_000_000u64);

            result.push(EthereumTransaction {
                hash: tx.hash,
                from: tx.from,
                to: tx.to,
                value: value_eth,
                gas_used: tx.gas_used.parse().unwrap_or(0),
                gas_price: gas_price_gwei,
                timestamp: tx.time_stamp.parse().unwrap_or(0),
                block_number: tx.block_number.parse().unwrap_or(0),
                is_error: tx.is_error != "0",
            });
        }

        Ok(result)
    }

    /// Get transactions for an address (raw Etherscan types).
    /// Used internally and by the BlockchainClient trait impl.
    pub async fn fetch_transactions(&self, address: &str) -> Result<Vec<EthereumTransaction>> {
        let mut url = format!(
            "{}?chainid={}&module=account&action=txlist&address={}&startblock=0&endblock=99999999&sort=asc",
            self.base_url, self.chain_id, address
        );

        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let response = reqwest::get(&url).await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to fetch transactions: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Etherscan API error: {}",
                response.status()
            )));
        }

        let data: EtherscanTxResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        if data.status != "1" {
            // No transactions is valid
            if data.message.contains("No transactions found") {
                return Ok(Vec::new());
            }
            return Err(CryptofolioError::Network(format!(
                "Etherscan API error: {}",
                data.message
            )));
        }

        // Convert to our format
        let mut result = Vec::new();
        for tx in data.result {
            let value_wei = Decimal::from_str(&tx.value).unwrap_or(Decimal::ZERO);
            let value_eth = value_wei / Decimal::from(1_000_000_000_000_000_000u64);

            let gas_price_wei = Decimal::from_str(&tx.gas_price).unwrap_or(Decimal::ZERO);
            let gas_price_gwei = gas_price_wei / Decimal::from(1_000_000_000u64);

            result.push(EthereumTransaction {
                hash: tx.hash,
                from: tx.from,
                to: tx.to,
                value: value_eth,
                gas_used: tx.gas_used.parse().unwrap_or(0),
                gas_price: gas_price_gwei,
                timestamp: tx.time_stamp.parse().unwrap_or(0),
                block_number: tx.block_number.parse().unwrap_or(0),
                is_error: tx.is_error != "0",
            });
        }

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// BlockchainClient trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl BlockchainClient for EtherscanClient {
    fn provider_name(&self) -> &str {
        "Etherscan"
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        let mut url = format!(
            "{}?chainid={}&module=proxy&action=eth_blockNumber",
            self.base_url, self.chain_id
        );
        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let start = std::time::Instant::now();
        let response = reqwest::get(&url).await.map_err(|e| {
            CryptofolioError::Network(format!("Etherscan health check failed: {}", e))
        })?;
        let latency_ms = start.elapsed().as_millis() as u64;

        if !response.status().is_success() {
            return Ok(HealthStatus::unreachable(self.provider_name(), latency_ms));
        }

        #[derive(serde::Deserialize)]
        struct BlockNumberResponse {
            result: String,
        }

        let data: BlockNumberResponse = response.json().await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to parse health response: {}", e))
        })?;

        let height = u64::from_str_radix(data.result.trim_start_matches("0x"), 16).unwrap_or(0);

        Ok(HealthStatus::reachable(
            self.provider_name(),
            Some(height),
            latency_ms,
        ))
    }

    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary> {
        let info = self.get_address_info(address).await?;

        let mut balances = vec![WalletBalance {
            asset: "ETH".to_string(),
            asset_id: None,
            quantity: info.balance,
            decimals: 18,
        }];

        for token in &info.tokens {
            balances.push(WalletBalance {
                asset: token.symbol.clone(),
                asset_id: Some(token.contract_address.clone()),
                quantity: token.balance,
                decimals: token.decimals,
            });
        }

        Ok(AddressSummary {
            address: address.to_string(),
            chain: Chain::Ethereum,
            balances,
            transaction_count: info.tokens.len() as u64,
            extras: None,
        })
    }

    async fn get_transactions(
        &self,
        address: &str,
        since_block: Option<u64>,
    ) -> Result<Vec<WalletTransaction>> {
        let raw = self
            .fetch_transactions_since(address, since_block.unwrap_or(0))
            .await?;

        let txs = raw
            .into_iter()
            .filter(|tx| !tx.is_error)
            .map(|tx| {
                let timestamp = DateTime::from_timestamp(tx.timestamp, 0).unwrap_or_else(Utc::now);

                let direction = if tx.to.eq_ignore_ascii_case(address) {
                    TransactionDirection::Incoming
                } else if tx.from.eq_ignore_ascii_case(address) {
                    TransactionDirection::Outgoing
                } else {
                    TransactionDirection::Internal
                };

                // Gas fee in ETH: gas_used * gas_price_gwei / 1e9
                let fee = Some(
                    Decimal::from(tx.gas_used) * tx.gas_price / Decimal::from(1_000_000_000u64),
                );

                WalletTransaction {
                    external_id: tx.hash,
                    direction,
                    amount: tx.value,
                    asset: "ETH".to_string(),
                    fee,
                    fee_asset: Some("ETH".to_string()),
                    block_height: Some(tx.block_number),
                    timestamp,
                    counterparty: Some(match direction {
                        TransactionDirection::Incoming => tx.from.clone(),
                        _ => tx.to.clone(),
                    }),
                    memo: None,
                }
            })
            .collect();

        Ok(txs)
    }
}

// Etherscan API response types
#[derive(Debug, Deserialize)]
struct EtherscanResponse {
    status: String,
    message: String,
    result: String,
}

#[derive(Debug, Deserialize)]
struct EtherscanTokenResponse {
    status: String,
    message: String,
    result: Vec<TokenTransaction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenTransaction {
    contract_address: String,
    token_symbol: String,
    token_name: String,
    token_decimal: String,
    value: String,
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct EtherscanTxResponse {
    status: String,
    message: String,
    result: Vec<EthTransaction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthTransaction {
    hash: String,
    from: String,
    to: String,
    value: String,
    #[allow(dead_code)] // Received from API but not used
    gas: String,
    gas_price: String,
    gas_used: String,
    time_stamp: String,
    is_error: String,
    block_number: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etherscan_client_creation() {
        // V2 API uses a single unified endpoint; chain is selected via chain_id
        let client = EtherscanClient::new(false, None);
        assert!(client.base_url.contains("etherscan.io"));
        assert_eq!(client.chain_id, 1); // Ethereum mainnet

        let testnet_client = EtherscanClient::new(true, None);
        assert!(testnet_client.base_url.contains("etherscan.io"));
        assert_eq!(testnet_client.chain_id, 11155111); // Sepolia
    }

    #[test]
    fn test_client_with_api_key() {
        let client = EtherscanClient::new(false, Some("test_key".to_string()));
        assert_eq!(client.api_key, Some("test_key".to_string()));
    }
}
