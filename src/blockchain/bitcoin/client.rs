use crate::blockchain::trait_def::BlockchainClient;
use crate::blockchain::types::{
    AddressSummary, Chain, HealthStatus, TransactionDirection, WalletBalance, WalletTransaction,
};
use crate::error::{CryptofolioError, Result};
/// Bitcoin blockchain clients (RPC and public APIs)
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

const SATS_PER_BTC: u64 = 100_000_000;

/// Bitcoin transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    pub txid: String,
    pub block_height: Option<u64>,
    pub timestamp: Option<i64>,
    pub value: Decimal, // In BTC
    pub fee: Option<Decimal>,
    pub is_incoming: bool,
}

/// Address information from blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    pub address: String,
    pub balance: Decimal, // In BTC
    pub total_received: Decimal,
    pub total_sent: Decimal,
    pub tx_count: u64,
}

/// Blockstream API client (public, no auth needed)
pub struct BlockstreamClient {
    base_url: String,
}

impl BlockstreamClient {
    /// Create a new Blockstream client
    pub fn new(testnet: bool) -> Self {
        let base_url = if testnet {
            "https://blockstream.info/testnet/api".to_string()
        } else {
            "https://blockstream.info/api".to_string()
        };

        Self { base_url }
    }

    /// Create a client with custom base URL (for testing)
    pub fn with_base_url(base_url: String) -> Self {
        Self { base_url }
    }

    /// Get address information
    pub async fn get_address_info(&self, address: &str) -> Result<AddressInfo> {
        let url = format!("{}/address/{}", self.base_url, address);

        let response = reqwest::get(&url).await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to fetch address info: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Blockstream API error: {}",
                response.status()
            )));
        }

        let data: BlockstreamAddressResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse response: {}", e)))?;

        // Convert satoshis to BTC
        let balance =
            Decimal::from(data.chain_stats.funded_txo_sum - data.chain_stats.spent_txo_sum)
                / Decimal::from(SATS_PER_BTC);
        let total_received =
            Decimal::from(data.chain_stats.funded_txo_sum) / Decimal::from(SATS_PER_BTC);
        let total_sent =
            Decimal::from(data.chain_stats.spent_txo_sum) / Decimal::from(SATS_PER_BTC);

        Ok(AddressInfo {
            address: address.to_string(),
            balance,
            total_received,
            total_sent,
            tx_count: data.chain_stats.tx_count,
        })
    }

    /// Get transactions for an address (raw Blockstream types).
    /// Used internally and by the BlockchainClient trait impl.
    pub async fn fetch_transactions(&self, address: &str) -> Result<Vec<BitcoinTransaction>> {
        let url = format!("{}/address/{}/txs", self.base_url, address);

        let response = reqwest::get(&url).await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to fetch transactions: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Blockstream API error: {}",
                response.status()
            )));
        }

        let txs: Vec<BlockstreamTransaction> = response.json().await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to parse transactions: {}", e))
        })?;

        // Convert to our format
        let mut result = Vec::new();
        for tx in txs {
            // Calculate value for this address
            let mut value_in = 0i64;
            let mut value_out = 0i64;

            for vout in &tx.vout {
                if vout.scriptpubkey_address.as_deref() == Some(address) {
                    value_in += vout.value;
                }
            }

            for vin in &tx.vin {
                if let Some(prevout) = &vin.prevout {
                    if prevout.scriptpubkey_address.as_deref() == Some(address) {
                        value_out += prevout.value;
                    }
                }
            }

            let net_value = value_in - value_out;
            let is_incoming = net_value > 0;
            let value = Decimal::from(net_value.abs()) / Decimal::from(SATS_PER_BTC);

            let fee = tx
                .fee
                .map(|f| Decimal::from(f) / Decimal::from(SATS_PER_BTC));

            result.push(BitcoinTransaction {
                txid: tx.txid,
                block_height: tx.status.block_height,
                timestamp: tx.status.block_time,
                value,
                fee,
                is_incoming,
            });
        }

        Ok(result)
    }
}

// Blockstream API response types
#[derive(Debug, Deserialize)]
struct BlockstreamAddressResponse {
    chain_stats: ChainStats,
}

#[derive(Debug, Deserialize)]
struct ChainStats {
    funded_txo_sum: i64,
    spent_txo_sum: i64,
    tx_count: u64,
}

#[derive(Debug, Deserialize)]
struct BlockstreamTransaction {
    txid: String,
    fee: Option<i64>,
    status: TxStatus,
    vin: Vec<TxInput>,
    vout: Vec<TxOutput>,
}

#[derive(Debug, Deserialize)]
struct TxStatus {
    #[allow(dead_code)]
    confirmed: bool,
    block_height: Option<u64>,
    block_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TxInput {
    prevout: Option<TxOutput>,
}

#[derive(Debug, Deserialize)]
struct TxOutput {
    scriptpubkey_address: Option<String>,
    value: i64,
}

// ---------------------------------------------------------------------------
// BlockchainClient trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl BlockchainClient for BlockstreamClient {
    fn provider_name(&self) -> &str {
        "Blockstream"
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        let url = format!("{}/blocks/tip/height", self.base_url);
        let start = std::time::Instant::now();

        let response = reqwest::get(&url).await.map_err(|e| {
            CryptofolioError::Network(format!("Blockstream health check failed: {}", e))
        })?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if !response.status().is_success() {
            return Ok(HealthStatus::unreachable(self.provider_name(), latency_ms));
        }

        let height = response
            .text()
            .await
            .unwrap_or_default()
            .trim()
            .parse::<u64>()
            .unwrap_or(0);

        Ok(HealthStatus::reachable(
            self.provider_name(),
            Some(height),
            latency_ms,
        ))
    }

    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary> {
        let info = self.get_address_info(address).await?;

        Ok(AddressSummary {
            address: address.to_string(),
            chain: Chain::Bitcoin,
            balances: vec![WalletBalance {
                asset: "BTC".to_string(),
                asset_id: None,
                quantity: info.balance,
                decimals: 8,
            }],
            transaction_count: info.tx_count,
            extras: None,
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
                Some(min) => tx.block_height.is_some_and(|h| h >= min),
                None => true,
            })
            .map(|tx| {
                let timestamp = tx
                    .timestamp
                    .and_then(|ts| DateTime::from_timestamp(ts, 0))
                    .unwrap_or_else(Utc::now);

                WalletTransaction {
                    external_id: tx.txid,
                    direction: if tx.is_incoming {
                        TransactionDirection::Incoming
                    } else {
                        TransactionDirection::Outgoing
                    },
                    amount: tx.value,
                    asset: "BTC".to_string(),
                    fee: tx.fee,
                    fee_asset: tx.fee.map(|_| "BTC".to_string()),
                    block_height: tx.block_height,
                    timestamp,
                    counterparty: None,
                    memo: None,
                }
            })
            .collect();

        Ok(txs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockstream_client_creation() {
        let client = BlockstreamClient::new(false);
        assert!(client.base_url.contains("blockstream.info"));
        assert!(!client.base_url.contains("testnet"));

        let testnet_client = BlockstreamClient::new(true);
        assert!(testnet_client.base_url.contains("testnet"));
    }
}
