//! Bittensor (TAO) watch-only client, backed by the Taostats API.
//!
//! TAO on a coldkey lives in two places: a **free** (liquid) balance and a
//! **staked** balance (root + alpha-as-tao, valued in TAO). The tracker's other
//! chains only ever had a single native balance, so a naive sync would miss the
//! staked portion entirely — which is most of a staker's holdings. We therefore
//! report `free + staked` as the TAO balance and keep the breakdown in
//! `ChainExtras::Bittensor`.
//!
//! Data source: `GET /api/account/latest/v1?address=<coldkey>` returns
//! `balance_free`, `balance_staked` and `balance_total`, all in **rao**
//! (1 TAO = 1e9 rao). Requires a Taostats API key in the `Authorization`
//! header (no `Bearer` prefix). Watch-only: coldkey address in, balances out —
//! no keys, no signing.

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;

use crate::blockchain::trait_def::BlockchainClient;
use crate::blockchain::types::{
    AddressSummary, Chain, ChainExtras, HealthStatus, WalletBalance, WalletTransaction,
};
use crate::error::{CryptofolioError, Result};

const RAO_PER_TAO: i64 = 1_000_000_000;
const BASE_URL: &str = "https://api.taostats.io";

/// Taostats-backed Bittensor client.
pub struct TaostatsClient {
    base_url: String,
    api_key: Option<String>,
}

impl TaostatsClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            base_url: BASE_URL.to_string(),
            api_key,
        }
    }

    /// Override the base URL (for tests against a mock server).
    pub fn with_base_url(base_url: String, api_key: Option<String>) -> Self {
        Self { base_url, api_key }
    }

    /// Convert a rao string (integer, 9 decimals) to a TAO `Decimal`.
    fn rao_to_tao(rao: &str) -> Decimal {
        Decimal::from_str(rao).unwrap_or(Decimal::ZERO) / Decimal::from(RAO_PER_TAO)
    }

    async fn fetch_account(&self, address: &str) -> Result<AccountRecord> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            CryptofolioError::Config(
                "Taostats API key not configured (set bittensor.api_key or TAOSTATS_API_KEY)"
                    .into(),
            )
        })?;

        let url = format!(
            "{}/api/account/latest/v1?address={}",
            self.base_url, address
        );
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", api_key)
            .send()
            .await
            .map_err(|e| {
                CryptofolioError::Network(format!("Failed to fetch Taostats account: {e}"))
            })?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Taostats API error: {}",
                response.status()
            )));
        }

        let body: AccountResponse = response
            .json()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse Taostats: {e}")))?;

        body.data.into_iter().next().ok_or_else(|| {
            CryptofolioError::Network(format!("Taostats returned no account for {address}"))
        })
    }
}

#[async_trait]
impl BlockchainClient for TaostatsClient {
    fn provider_name(&self) -> &str {
        "Taostats"
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        let start = std::time::Instant::now();
        let api_key = match self.api_key.as_ref() {
            Some(k) => k,
            None => return Ok(HealthStatus::unreachable(self.provider_name(), 0)),
        };
        let url = format!("{}/api/status/v1", self.base_url);
        let client = reqwest::Client::new();
        let latency = |s: std::time::Instant| s.elapsed().as_millis() as u64;
        match client
            .get(&url)
            .header("Authorization", api_key)
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => Ok(HealthStatus::reachable(
                self.provider_name(),
                None,
                latency(start),
            )),
            _ => Ok(HealthStatus::unreachable(
                self.provider_name(),
                latency(start),
            )),
        }
    }

    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary> {
        let acct = self.fetch_account(address).await?;

        let free = Self::rao_to_tao(&acct.balance_free);
        let staked = Self::rao_to_tao(&acct.balance_staked);
        // Total TAO the coldkey controls = free + staked. Prefer the API's own
        // balance_total when present (it is free + staked), else sum them.
        let total = if acct.balance_total.is_empty() {
            free + staked
        } else {
            Self::rao_to_tao(&acct.balance_total)
        };

        Ok(AddressSummary {
            address: address.to_string(),
            chain: Chain::Bittensor,
            balances: vec![WalletBalance {
                asset: "TAO".to_string(),
                asset_id: None,
                quantity: total,
                decimals: 9,
            }],
            transaction_count: 0,
            extras: Some(ChainExtras::Bittensor { free, staked }),
        })
    }

    async fn get_transactions(
        &self,
        _address: &str,
        _since_block: Option<u64>,
    ) -> Result<Vec<WalletTransaction>> {
        // Transaction history (transfers / stake events) is a separate Taostats
        // endpoint; balances are the first deliverable. Return none for now so
        // the balance sync works without inventing transactions.
        Ok(Vec::new())
    }

    async fn get_chain_extras(&self, address: &str) -> Result<Option<ChainExtras>> {
        let acct = self.fetch_account(address).await?;
        Ok(Some(ChainExtras::Bittensor {
            free: Self::rao_to_tao(&acct.balance_free),
            staked: Self::rao_to_tao(&acct.balance_staked),
        }))
    }
}

// --- Taostats /api/account/latest/v1 response ------------------------------

#[derive(Debug, Deserialize)]
struct AccountResponse {
    data: Vec<AccountRecord>,
}

#[derive(Debug, Deserialize)]
struct AccountRecord {
    #[serde(default)]
    balance_free: String,
    #[serde(default)]
    balance_staked: String,
    #[serde(default)]
    balance_total: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rao_converts_to_tao() {
        // 31_471_386 rao = 0.031471386 TAO
        assert_eq!(
            TaostatsClient::rao_to_tao("31471386").to_string(),
            "0.031471386"
        );
        // 1 TAO
        assert_eq!(TaostatsClient::rao_to_tao("1000000000").to_string(), "1");
        // empty / junk -> zero
        assert_eq!(TaostatsClient::rao_to_tao(""), Decimal::ZERO);
    }

    #[test]
    fn provider_name_is_taostats() {
        let c = TaostatsClient::new(None);
        assert_eq!(c.provider_name(), "Taostats");
    }

    #[tokio::test]
    async fn missing_key_is_unreachable_not_panic() {
        let c = TaostatsClient::new(None);
        let h = c.health_check().await.expect("health check returns Ok");
        assert!(!h.reachable);
    }
}
