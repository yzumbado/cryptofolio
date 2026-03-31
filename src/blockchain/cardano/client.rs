/// Cardano blockchain client (Blockfrost API)
use crate::error::{CryptofolioError, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Cardano native token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeToken {
    pub unit: String,         // Policy ID + asset name
    pub quantity: String,     // Raw quantity as string
    pub fingerprint: String,  // Asset fingerprint
    pub policy_id: String,    // Policy ID
    pub asset_name: String,   // Asset name (hex encoded)
    pub display_name: String, // Human-readable name
    pub decimals: u8,         // Decimal places (from metadata)
    pub balance: Decimal,     // Human-readable balance
}

/// Cardano transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardanoTransaction {
    pub hash: String,
    pub block: String,
    pub block_height: u64,
    pub block_time: i64,
    pub slot: u64,
    pub index: u32,
    pub fees: Decimal,    // In ADA
    pub deposit: Decimal, // In ADA
    pub size: u32,
}

/// Stake pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakePoolInfo {
    pub pool_id: String,
    pub ticker: String,
    pub name: String,
    pub active_stake: Decimal, // In ADA
    pub live_pledge: Decimal,  // In ADA
    pub margin_cost: f64,      // As percentage
}

/// Address information from blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    pub address: String,
    pub balance: Decimal, // In ADA
    pub tokens: Vec<NativeToken>,
    pub stake_address: Option<String>,
    pub stake_pool: Option<StakePoolInfo>,
    pub tx_count: u64,
}

/// Blockfrost API client
pub struct BlockfrostClient {
    base_url: String,
    api_key: Option<String>,
}

impl BlockfrostClient {
    /// Create a new Blockfrost client
    ///
    /// Networks:
    /// - Mainnet: https://cardano-mainnet.blockfrost.io/api/v0
    /// - Preprod: https://cardano-preprod.blockfrost.io/api/v0
    /// - Preview: https://cardano-preview.blockfrost.io/api/v0
    pub fn new(testnet: bool, api_key: Option<String>) -> Self {
        let base_url = if testnet {
            "https://cardano-preprod.blockfrost.io/api/v0".to_string()
        } else {
            "https://cardano-mainnet.blockfrost.io/api/v0".to_string()
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
        // Get ADA balance and basic info
        let address_data = self.get_address_data(address).await?;

        // Get native tokens
        let tokens = self.get_native_tokens(address).await?;

        // Get stake address and delegation info
        let (stake_address, stake_pool) = self.get_stake_info(address).await?;

        Ok(AddressInfo {
            address: address.to_string(),
            balance: address_data.balance,
            tokens,
            stake_address,
            stake_pool,
            tx_count: address_data.tx_count,
        })
    }

    /// Get address data (balance, tx count)
    async fn get_address_data(&self, address: &str) -> Result<AddressData> {
        let url = format!("{}/addresses/{}", self.base_url, address);

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("project_id", key);
        }

        let response = request.send().await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to fetch address data: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Blockfrost API error: {}",
                response.status()
            )));
        }

        let data: BlockfrostAddressResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        // Convert lovelace to ADA (1 ADA = 1,000,000 lovelace)
        let balance = Decimal::from_str(
            data
                .amount
                .iter()
                .find(|a| a.unit == "lovelace")
                .map(|a| a.quantity.as_str())
                .unwrap_or("0"),
        )
        .unwrap_or(Decimal::ZERO)
            / Decimal::from(1_000_000);

        Ok(AddressData {
            balance,
            tx_count: data.tx_count,
        })
    }

    /// Get native tokens for an address
    async fn get_native_tokens(&self, address: &str) -> Result<Vec<NativeToken>> {
        let url = format!("{}/addresses/{}/total", self.base_url, address);

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("project_id", key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to fetch tokens: {}", e)))?;

        if !response.status().is_success() {
            // No tokens is valid - return empty list
            return Ok(Vec::new());
        }

        let data: BlockfrostAddressResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        // Filter out lovelace (ADA) and extract native tokens
        let mut tokens = Vec::new();
        for amount in data.amount {
            if amount.unit == "lovelace" {
                continue; // Skip ADA
            }

            // Parse unit (format: policyId + assetName)
            let (policy_id, asset_name) = if amount.unit.len() > 56 {
                (amount.unit[..56].to_string(), amount.unit[56..].to_string())
            } else {
                (amount.unit.clone(), String::new())
            };

            // Try to get token metadata for display name and decimals
            let (display_name, decimals) = self
                .get_token_metadata(&amount.unit)
                .await
                .unwrap_or_else(|_| (asset_name.clone(), 0));

            // Calculate human-readable balance
            let quantity = Decimal::from_str(&amount.quantity).unwrap_or(Decimal::ZERO);
            let divisor = if decimals > 0 {
                Decimal::from(10_i128.pow(decimals as u32))
            } else {
                Decimal::ONE
            };
            let balance = quantity / divisor;

            tokens.push(NativeToken {
                unit: amount.unit.clone(),
                quantity: amount.quantity,
                fingerprint: String::new(), // TODO: Calculate asset fingerprint
                policy_id,
                asset_name: asset_name.clone(),
                display_name,
                decimals,
                balance,
            });
        }

        // Sort by display name for consistent output
        tokens.sort_by(|a, b| a.display_name.cmp(&b.display_name));

        Ok(tokens)
    }

    /// Get token metadata (name, decimals)
    async fn get_token_metadata(&self, unit: &str) -> Result<(String, u8)> {
        let url = format!("{}/assets/{}", self.base_url, unit);

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("project_id", key);
        }

        let response = request.send().await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to fetch token metadata: {}", e))
        })?;

        if !response.status().is_success() {
            // No metadata - use asset name
            return Ok((String::new(), 0));
        }

        let data: BlockfrostAssetResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        let display_name = data
            .onchain_metadata
            .as_ref()
            .and_then(|m| m.name.clone())
            .or(data.asset_name)
            .unwrap_or_default();

        let decimals = data
            .onchain_metadata
            .as_ref()
            .and_then(|m| m.decimals)
            .unwrap_or(0);

        Ok((display_name, decimals))
    }

    /// Get stake address and delegation info
    async fn get_stake_info(
        &self,
        _address: &str,
    ) -> Result<(Option<String>, Option<StakePoolInfo>)> {
        // TODO: Implement stake address lookup and pool delegation info
        // This requires additional Blockfrost API calls
        Ok((None, None))
    }

    /// Get transactions for an address
    pub async fn get_transactions(&self, address: &str) -> Result<Vec<CardanoTransaction>> {
        let url = format!("{}/addresses/{}/transactions", self.base_url, address);

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("project_id", key);
        }

        let response = request.send().await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to fetch transactions: {}", e))
        })?;

        if !response.status().is_success() {
            // No transactions is valid
            return Ok(Vec::new());
        }

        let data: Vec<BlockfrostTxResponse> = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        // Convert to our format
        let mut result = Vec::new();
        for tx in data {
            // Get detailed transaction info
            if let Ok(tx_detail) = self.get_transaction_detail(&tx.tx_hash).await {
                result.push(tx_detail);
            }
        }

        Ok(result)
    }

    /// Get detailed transaction information
    async fn get_transaction_detail(&self, tx_hash: &str) -> Result<CardanoTransaction> {
        let url = format!("{}/txs/{}", self.base_url, tx_hash);

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if let Some(key) = &self.api_key {
            request = request.header("project_id", key);
        }

        let response = request.send().await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to fetch transaction detail: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Blockfrost API error: {}",
                response.status()
            )));
        }

        let data: BlockfrostTxDetailResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        // Convert fees and deposit from lovelace to ADA
        let fees =
            Decimal::from_str(&data.fees).unwrap_or(Decimal::ZERO) / Decimal::from(1_000_000);
        let deposit =
            Decimal::from_str(&data.deposit).unwrap_or(Decimal::ZERO) / Decimal::from(1_000_000);

        Ok(CardanoTransaction {
            hash: data.hash,
            block: data.block,
            block_height: data.block_height,
            block_time: data.block_time,
            slot: data.slot,
            index: data.index,
            fees,
            deposit,
            size: data.size,
        })
    }
}

// Internal helper struct
struct AddressData {
    balance: Decimal,
    tx_count: u64,
}

// Blockfrost API response types
#[derive(Debug, Deserialize)]
struct BlockfrostAddressResponse {
    amount: Vec<AmountItem>,
    tx_count: u64,
}

#[derive(Debug, Deserialize)]
struct AmountItem {
    unit: String,
    quantity: String,
}

#[derive(Debug, Deserialize)]
struct BlockfrostAssetResponse {
    asset_name: Option<String>,
    onchain_metadata: Option<OnchainMetadata>,
}

#[derive(Debug, Deserialize)]
struct OnchainMetadata {
    name: Option<String>,
    decimals: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct BlockfrostTxResponse {
    tx_hash: String,
}

#[derive(Debug, Deserialize)]
struct BlockfrostTxDetailResponse {
    hash: String,
    block: String,
    block_height: u64,
    block_time: i64,
    slot: u64,
    index: u32,
    fees: String,
    deposit: String,
    size: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockfrost_client_creation() {
        let client = BlockfrostClient::new(false, None);
        assert!(client.base_url.contains("mainnet"));

        let testnet_client = BlockfrostClient::new(true, None);
        assert!(testnet_client.base_url.contains("preprod"));
    }

    #[test]
    fn test_client_with_api_key() {
        let client = BlockfrostClient::new(false, Some("test_key".to_string()));
        assert_eq!(client.api_key, Some("test_key".to_string()));
    }
}
