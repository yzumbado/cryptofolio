/// Bitcoin blockchain clients (RPC and public APIs)
use crate::error::{CryptofolioError, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
                / Decimal::from(100_000_000);
        let total_received =
            Decimal::from(data.chain_stats.funded_txo_sum) / Decimal::from(100_000_000);
        let total_sent = Decimal::from(data.chain_stats.spent_txo_sum) / Decimal::from(100_000_000);

        Ok(AddressInfo {
            address: address.to_string(),
            balance,
            total_received,
            total_sent,
            tx_count: data.chain_stats.tx_count,
        })
    }

    /// Get transactions for an address
    pub async fn get_transactions(&self, address: &str) -> Result<Vec<BitcoinTransaction>> {
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
                if vin
                    .prevout
                    .as_ref()
                    .and_then(|p| p.scriptpubkey_address.as_deref())
                    == Some(address)
                {
                    value_out += vin.prevout.as_ref().unwrap().value;
                }
            }

            let net_value = value_in - value_out;
            let is_incoming = net_value > 0;
            let value = Decimal::from(net_value.abs()) / Decimal::from(100_000_000);

            let fee = tx
                .fee
                .map(|f| Decimal::from(f) / Decimal::from(100_000_000));

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
