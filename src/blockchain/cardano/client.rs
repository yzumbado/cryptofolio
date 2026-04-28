/// Cardano blockchain client (Blockfrost API)
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::blockchain::trait_def::BlockchainClient;
use crate::blockchain::types::{
    AddressSummary, Chain, ChainExtras, HealthStatus, TransactionDirection, WalletBalance,
    WalletTransaction,
};
use crate::error::{CryptofolioError, Result};
use bech32::{Bech32, Hrp};
use blake2::digest::{FixedOutput, Update};
use blake2::Blake2b;
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
        // Get ADA balance and basic info (also returns stake_address)
        let address_data = self.get_address_data(address).await?;

        // Get native tokens
        let tokens = self.get_native_tokens(address).await?;

        // Get stake delegation info using the stake address from address data
        let (stake_address, stake_pool) = if let Some(ref sa) = address_data.stake_address {
            self.get_stake_info(sa).await?
        } else {
            (None, None)
        };

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
            data.amount
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
            stake_address: data.stake_address,
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

            let fingerprint = cip14_fingerprint(&policy_id, &asset_name);

            tokens.push(NativeToken {
                unit: amount.unit.clone(),
                quantity: amount.quantity,
                fingerprint,
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

    /// Get stake delegation info for a stake address.
    ///
    /// Calls:
    ///   GET /accounts/{stake_address}        → pool_id + controlled amount
    ///   GET /pools/{pool_id}/metadata        → ticker, name
    ///   GET /pools/{pool_id}                 → live_pledge, margin_cost
    async fn get_stake_info(
        &self,
        stake_address: &str,
    ) -> Result<(Option<String>, Option<StakePoolInfo>)> {
        let client = reqwest::Client::new();

        // 1. Fetch account info
        let account_url = format!("{}/accounts/{}", self.base_url, stake_address);
        let mut req = client.get(&account_url);
        if let Some(key) = &self.api_key {
            req = req.header("project_id", key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to fetch account: {}", e)))?;

        if !resp.status().is_success() {
            // Stake address not registered on-chain — valid for new wallets
            return Ok((Some(stake_address.to_string()), None));
        }

        let account: BlockfrostAccountResponse = resp.json().await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to parse account response: {}", e))
        })?;

        let pool_id = match account.pool_id {
            Some(pid) => pid,
            None => return Ok((Some(stake_address.to_string()), None)),
        };

        // 2. Fetch pool metadata (ticker, name)
        let meta_url = format!("{}/pools/{}/metadata", self.base_url, pool_id);
        let mut req = client.get(&meta_url);
        if let Some(key) = &self.api_key {
            req = req.header("project_id", key);
        }
        let meta_resp = req.send().await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to fetch pool metadata: {}", e))
        })?;

        let (ticker, name) = if meta_resp.status().is_success() {
            let meta: BlockfrostPoolMetaResponse = meta_resp.json().await.unwrap_or_default();
            (
                meta.ticker.unwrap_or_else(|| pool_id[..8].to_string()),
                meta.name.unwrap_or_default(),
            )
        } else {
            (pool_id[..8].to_string(), String::new())
        };

        // 3. Fetch pool stats (pledge, margin)
        let pool_url = format!("{}/pools/{}", self.base_url, pool_id);
        let mut req = client.get(&pool_url);
        if let Some(key) = &self.api_key {
            req = req.header("project_id", key);
        }
        let pool_resp = req
            .send()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to fetch pool stats: {}", e)))?;

        let (active_stake, live_pledge, margin_cost) = if pool_resp.status().is_success() {
            let stats: BlockfrostPoolResponse = pool_resp.json().await.unwrap_or_default();
            let active = Decimal::from_str(&stats.active_stake.unwrap_or_default())
                .unwrap_or(Decimal::ZERO)
                / Decimal::from(1_000_000);
            let pledge = Decimal::from_str(&stats.live_pledge.unwrap_or_default())
                .unwrap_or(Decimal::ZERO)
                / Decimal::from(1_000_000);
            (active, pledge, stats.margin_cost.unwrap_or(0.0))
        } else {
            (Decimal::ZERO, Decimal::ZERO, 0.0)
        };

        Ok((
            Some(stake_address.to_string()),
            Some(StakePoolInfo {
                pool_id,
                ticker,
                name,
                active_stake,
                live_pledge,
                margin_cost,
            }),
        ))
    }

    /// Get transactions for an address (raw Blockfrost types).
    /// Used internally and by the BlockchainClient trait impl.
    pub async fn fetch_transactions(&self, address: &str) -> Result<Vec<CardanoTransaction>> {
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
    stake_address: Option<String>,
}

// Blockfrost API response types
#[derive(Debug, Deserialize)]
struct BlockfrostAddressResponse {
    amount: Vec<AmountItem>,
    tx_count: u64,
    stake_address: Option<String>,
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
struct BlockfrostAccountResponse {
    pool_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct BlockfrostPoolMetaResponse {
    ticker: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct BlockfrostPoolResponse {
    active_stake: Option<String>,
    live_pledge: Option<String>,
    margin_cost: Option<f64>,
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

// ---------------------------------------------------------------------------
// BlockchainClient trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl BlockchainClient for BlockfrostClient {
    fn provider_name(&self) -> &str {
        "Blockfrost"
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        #[derive(serde::Deserialize)]
        struct LatestBlock {
            height: Option<u64>,
        }

        let client = reqwest::Client::new();
        let start = std::time::Instant::now();

        // 1. Confirm the API is healthy
        let health_url = format!("{}/health", self.base_url);
        let mut req = client.get(&health_url);
        if let Some(key) = &self.api_key {
            req = req.header("project_id", key);
        }
        let response = req.send().await.map_err(|e| {
            CryptofolioError::Network(format!("Blockfrost health check failed: {}", e))
        })?;
        let latency_ms = start.elapsed().as_millis() as u64;

        if !response.status().is_success() {
            return Ok(HealthStatus::unreachable(self.provider_name(), latency_ms));
        }

        // 2. Fetch latest block height
        let block_url = format!("{}/blocks/latest", self.base_url);
        let mut block_req = client.get(&block_url);
        if let Some(key) = &self.api_key {
            block_req = block_req.header("project_id", key);
        }
        let block_height = block_req
            .send()
            .await
            .ok()
            .and_then(|r| if r.status().is_success() { Some(r) } else { None });

        let block_height: Option<u64> = if let Some(resp) = block_height {
            resp.json::<LatestBlock>().await.ok().and_then(|b| b.height)
        } else {
            None
        };

        Ok(HealthStatus::reachable(self.provider_name(), block_height, latency_ms))
    }

    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary> {
        let info = self.get_address_info(address).await?;

        let mut balances = vec![WalletBalance {
            asset: "ADA".to_string(),
            asset_id: None,
            quantity: info.balance,
            decimals: 6,
        }];

        for token in &info.tokens {
            balances.push(WalletBalance {
                asset: token.display_name.clone(),
                asset_id: Some(token.unit.clone()),
                quantity: token.balance,
                decimals: token.decimals,
            });
        }

        let extras = Some(ChainExtras::Cardano {
            stake_address: info.stake_address.clone(),
            pool_id: info.stake_pool.as_ref().map(|p| p.pool_id.clone()),
            pool_ticker: info.stake_pool.as_ref().map(|p| p.ticker.clone()),
            pool_name: info.stake_pool.as_ref().map(|p| p.name.clone()),
            active_stake: info.stake_pool.as_ref().map(|p| p.active_stake),
            margin_cost: info.stake_pool.as_ref().map(|p| p.margin_cost),
        });

        Ok(AddressSummary {
            address: address.to_string(),
            chain: Chain::Cardano,
            balances,
            transaction_count: info.tx_count,
            extras,
        })
    }

    async fn get_transactions(
        &self,
        address: &str,
        since_block: Option<u64>,
    ) -> Result<Vec<WalletTransaction>> {
        let raw = self.fetch_transactions(address).await?;

        let txs = raw
            .into_iter()
            .filter(|tx| match since_block {
                Some(min) => tx.block_height >= min,
                None => true,
            })
            .map(|tx| {
                let timestamp = DateTime::from_timestamp(tx.block_time, 0)
                    .unwrap_or_else(Utc::now);

                WalletTransaction {
                    external_id: tx.hash,
                    // Cardano UTXO direction requires fetching UTXOs — marked Internal for now.
                    // Phase 3+ will resolve this via /txs/{hash}/utxos.
                    direction: TransactionDirection::Internal,
                    amount: tx.fees,
                    asset: "ADA".to_string(),
                    fee: Some(tx.fees),
                    fee_asset: Some("ADA".to_string()),
                    block_height: Some(tx.block_height),
                    timestamp,
                    counterparty: None,
                    memo: None,
                }
            })
            .collect();

        Ok(txs)
    }

    async fn get_chain_extras(&self, address: &str) -> Result<Option<ChainExtras>> {
        let info = self.get_address_info(address).await?;

        Ok(Some(ChainExtras::Cardano {
            stake_address: info.stake_address,
            pool_id: info.stake_pool.as_ref().map(|p| p.pool_id.clone()),
            pool_ticker: info.stake_pool.as_ref().map(|p| p.ticker.clone()),
            pool_name: info.stake_pool.as_ref().map(|p| p.name.clone()),
            active_stake: info.stake_pool.as_ref().map(|p| p.active_stake),
            margin_cost: info.stake_pool.as_ref().map(|p| p.margin_cost),
        }))
    }
}

/// Compute a CIP-14 asset fingerprint.
///
/// Algorithm:
/// 1. Decode policy_id and asset_name from hex.
/// 2. Concatenate the bytes.
/// 3. Hash with BLAKE2b-160 (20-byte digest).
/// 4. Bech32-encode with HRP `"asset"`.
fn cip14_fingerprint(policy_id_hex: &str, asset_name_hex: &str) -> String {
    // Decode hex fields (empty asset_name_hex is valid — "lovelace" has none)
    let policy_bytes = hex::decode(policy_id_hex).unwrap_or_default();
    let name_bytes = hex::decode(asset_name_hex).unwrap_or_default();

    // BLAKE2b with a 160-bit (20-byte) digest
    type Blake2b160 = Blake2b<blake2::digest::typenum::U20>;
    let mut hasher = Blake2b160::default();
    hasher.update(&policy_bytes);
    hasher.update(&name_bytes);
    let hash = hasher.finalize_fixed();

    // Bech32 encode with HRP "asset"
    let asset_hrp = Hrp::parse("asset").expect("static HRP is valid");
    bech32::encode::<Bech32>(asset_hrp, &hash).unwrap_or_else(|_| String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cip14_fingerprint_known_vector() {
        // CIP-14 test vector: empty asset name
        let policy = "7eae28af2208be856f7a119668ae52a49b73725e326dc16579dcc373";
        let fp = cip14_fingerprint(policy, "");
        assert_eq!(fp, "asset1rjklcrnsdzqp65wjgrg55sy9723kw09mlgvlc3");
    }

    #[test]
    fn test_cip14_fingerprint_with_asset_name() {
        let policy = "1e349c9bdea19fd6c147626a5260bc44b71635f398b67c59881df209";
        let fp = cip14_fingerprint(policy, "504154415445");
        assert_eq!(fp, "asset1hv4p5tv2a837mzqrst04d0dcptdjmluqvdx9k3");
    }

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
