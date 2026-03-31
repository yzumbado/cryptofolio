/// Ethereum blockchain clients (Etherscan API)

use crate::error::{CryptofolioError, Result};
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
    pub value: Decimal,        // In ETH
    pub gas_used: u64,
    pub gas_price: Decimal,    // In Gwei
    pub timestamp: i64,
    pub block_number: u64,
    pub is_error: bool,
}

/// Address information from blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    pub address: String,
    pub balance: Decimal,      // In ETH
    pub tokens: Vec<ERC20Token>,
}

/// Etherscan API client
pub struct EtherscanClient {
    base_url: String,
    api_key: Option<String>,
}

impl EtherscanClient {
    /// Create a new Etherscan client
    pub fn new(testnet: bool, api_key: Option<String>) -> Self {
        let base_url = if testnet {
            "https://api-sepolia.etherscan.io/api".to_string()
        } else {
            "https://api.etherscan.io/api".to_string()
        };

        Self { base_url, api_key }
    }

    /// Create a client with custom base URL (for testing)
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            base_url,
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
            "{}?module=account&action=balance&address={}&tag=latest",
            self.base_url, address
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
            "{}?module=account&action=tokentx&address={}&startblock=0&endblock=99999999&sort=asc",
            self.base_url, address
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

            let entry = token_map
                .entry(tx.contract_address.clone())
                .or_insert((tx.token_symbol, tx.token_name, 0, decimals));

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
            .map(|(contract, (symbol, name, balance, decimals))| {
                let divisor = Decimal::from(10_i128.pow(decimals as u32));
                let token_balance = Decimal::from(balance) / divisor;

                ERC20Token {
                    symbol,
                    name,
                    contract_address: contract,
                    balance: token_balance,
                    decimals,
                }
            })
            .collect();

        // Sort by symbol for consistent output
        tokens.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        Ok(tokens)
    }

    /// Get transactions for an address
    pub async fn get_transactions(&self, address: &str) -> Result<Vec<EthereumTransaction>> {
        let mut url = format!(
            "{}?module=account&action=txlist&address={}&startblock=0&endblock=99999999&sort=asc",
            self.base_url, address
        );

        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let response = reqwest::get(&url)
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to fetch transactions: {}", e)))?;

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
    #[allow(dead_code)]  // Received from API but not used
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
        let client = EtherscanClient::new(false, None);
        assert!(client.base_url.contains("etherscan.io"));
        assert!(!client.base_url.contains("sepolia"));

        let testnet_client = EtherscanClient::new(true, None);
        assert!(testnet_client.base_url.contains("sepolia"));
    }

    #[test]
    fn test_client_with_api_key() {
        let client = EtherscanClient::new(false, Some("test_key".to_string()));
        assert_eq!(client.api_key, Some("test_key".to_string()));
    }
}
