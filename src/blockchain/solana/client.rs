/// Solana RPC client implementing the BlockchainClient trait.
///
/// Uses the Solana JSON-RPC API directly (no Solana SDK dependency).
/// Supports SOL balance, SPL token balances, stake accounts, and
/// transaction history via `getSignaturesForAddress`.
///
/// # No public RPC default
///
/// The public Solana RPC (`api.mainnet-beta.solana.com`) aggressively
/// rate-limits unauthenticated requests. Users must supply their own
/// endpoint (e.g. Helius free tier).
///
/// Configure in `config.toml`:
/// ```toml
/// [blockchain.solana]
/// rpc_url = "https://mainnet.helius-rpc.com/?api-key=<key>"
/// ```
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::blockchain::trait_def::BlockchainClient;
use crate::blockchain::types::{
    AddressSummary, Chain, ChainExtras, HealthStatus, SolanaStakeAccount, TransactionDirection,
    WalletBalance, WalletTransaction,
};
use crate::error::{CryptofolioError, Result};

// ---------------------------------------------------------------------------
// Jupiter token list
// ---------------------------------------------------------------------------

const JUPITER_TOKEN_LIST_URL: &str = "https://token.jup.ag/strict";

#[derive(Debug, Clone)]
struct TokenInfo {
    symbol: String,
    name: String,
    // decimals omitted: SPL balances arrive as uiAmount (already scaled by the RPC)
}

// ---------------------------------------------------------------------------
// SolanaRpcClient
// ---------------------------------------------------------------------------

pub struct SolanaRpcClient {
    rpc_url: String,
    http: reqwest::Client,
    /// mint pubkey → (symbol, name, decimals)
    token_list: Arc<RwLock<HashMap<String, TokenInfo>>>,
}

impl SolanaRpcClient {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            http: reqwest::Client::new(),
            token_list: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // -----------------------------------------------------------------------
    // RPC helpers
    // -----------------------------------------------------------------------

    async fn post_rpc(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let response = self
            .http
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| CryptofolioError::Network(format!("Solana RPC request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CryptofolioError::Network(format!(
                "Solana RPC HTTP error: {}",
                response.status()
            )));
        }

        let json: Value = response.json().await.map_err(|e| {
            CryptofolioError::Network(format!("Failed to parse Solana RPC response: {}", e))
        })?;

        if let Some(err) = json.get("error") {
            return Err(CryptofolioError::Network(format!(
                "Solana RPC error: {}",
                err
            )));
        }

        Ok(json["result"].clone())
    }

    // -----------------------------------------------------------------------
    // Jupiter token list cache
    // -----------------------------------------------------------------------

    async fn ensure_token_list(&self) -> Result<()> {
        {
            let list = self.token_list.read().await;
            if !list.is_empty() {
                return Ok(());
            }
        }

        #[derive(Deserialize)]
        #[allow(dead_code)] // decimals from API; reserved for future balance-scaling use
        struct JupiterToken {
            address: String,
            symbol: String,
            name: String,
            decimals: u8,
        }

        let response = self
            .http
            .get(JUPITER_TOKEN_LIST_URL)
            .send()
            .await
            .map_err(|e| {
                CryptofolioError::Network(format!("Failed to fetch Jupiter token list: {}", e))
            })?;

        if !response.status().is_success() {
            // Non-fatal: token metadata just won't resolve
            return Ok(());
        }

        let tokens: Vec<JupiterToken> = response.json().await.unwrap_or_default();

        let mut list = self.token_list.write().await;
        for token in tokens {
            list.insert(
                token.address,
                TokenInfo {
                    symbol: token.symbol,
                    name: token.name,
                },
            );
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Chain-specific fetchers
    // -----------------------------------------------------------------------

    async fn get_sol_balance(&self, address: &str) -> Result<Decimal> {
        let result = self
            .post_rpc("getBalance", json!([address, {"commitment": "finalized"}]))
            .await?;

        let lamports = result["value"].as_u64().unwrap_or(0);
        Ok(Decimal::from(lamports) / Decimal::from(1_000_000_000u64))
    }

    async fn get_spl_balances(&self, address: &str) -> Result<Vec<WalletBalance>> {
        let result = self
            .post_rpc(
                "getTokenAccountsByOwner",
                json!([
                    address,
                    {"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"},
                    {"encoding": "jsonParsed"}
                ]),
            )
            .await?;

        let _ = self.ensure_token_list().await;
        let token_list = self.token_list.read().await;

        let mut balances = Vec::new();

        if let Some(accounts) = result["value"].as_array() {
            for account in accounts {
                let info = &account["account"]["data"]["parsed"]["info"];
                let mint = info["mint"].as_str().unwrap_or("").to_string();
                let ui_amount = info["tokenAmount"]["uiAmount"].as_f64().unwrap_or(0.0);
                let decimals = info["tokenAmount"]["decimals"].as_u64().unwrap_or(0) as u8;

                if ui_amount == 0.0 {
                    continue;
                }

                let (symbol, _name) = if let Some(info) = token_list.get(&mint) {
                    (info.symbol.clone(), info.name.clone())
                } else {
                    // Unknown token — use truncated mint as symbol
                    let short = if mint.len() > 8 { &mint[..8] } else { &mint };
                    (short.to_string(), mint.clone())
                };

                let quantity = Decimal::try_from(ui_amount).unwrap_or(Decimal::ZERO);

                balances.push(WalletBalance {
                    asset: symbol,
                    asset_id: Some(mint),
                    quantity,
                    decimals,
                });
            }
        }

        Ok(balances)
    }

    async fn get_stake_accounts(&self, address: &str) -> Result<Vec<SolanaStakeAccount>> {
        let result = self
            .post_rpc(
                "getProgramAccounts",
                json!([
                    "Stake11111111111111111111111111111111111111111",
                    {
                        "filters": [{"memcmp": {"offset": 44, "bytes": address}}],
                        "encoding": "jsonParsed"
                    }
                ]),
            )
            .await?;

        let mut accounts = Vec::new();

        if let Some(arr) = result.as_array() {
            for item in arr {
                let pubkey = item["pubkey"].as_str().unwrap_or("").to_string();
                let lamports = item["account"]["lamports"].as_u64().unwrap_or(0);
                let validator = item["account"]["data"]["parsed"]["info"]["stake"]["delegation"]
                    ["voter"]
                    .as_str()
                    .map(|s| s.to_string());
                let activation_epoch = item["account"]["data"]["parsed"]["info"]["stake"]
                    ["delegation"]["activationEpoch"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok());

                accounts.push(SolanaStakeAccount {
                    pubkey,
                    lamports,
                    validator_vote_account: validator,
                    activation_epoch,
                });
            }
        }

        Ok(accounts)
    }

    async fn get_signatures(&self, address: &str, since_slot: Option<u64>) -> Result<Vec<String>> {
        let params = json!([address, {"limit": 1000}]);

        // Solana uses slot for pagination, not block height — use as before cursor
        // since_slot maps to `minContextSlot` in newer RPC versions;
        // we implement it as a post-filter on block_time for simplicity.
        let _ = since_slot; // used below in get_transactions filter

        let result = self.post_rpc("getSignaturesForAddress", params).await?;

        let signatures = result
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item["signature"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(signatures)
    }
}

// ---------------------------------------------------------------------------
// BlockchainClient implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl BlockchainClient for SolanaRpcClient {
    fn provider_name(&self) -> &str {
        "Solana RPC"
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        let start = std::time::Instant::now();

        let result = self.post_rpc("getSlot", json!([])).await;
        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(slot) => Ok(HealthStatus::reachable(
                self.provider_name(),
                slot.as_u64(),
                latency_ms,
            )),
            Err(_) => Ok(HealthStatus::unreachable(self.provider_name(), latency_ms)),
        }
    }

    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary> {
        let sol_balance = self.get_sol_balance(address).await?;
        let spl_balances = self.get_spl_balances(address).await?;
        let stake_accounts = self.get_stake_accounts(address).await.unwrap_or_default();

        let mut balances = vec![WalletBalance {
            asset: "SOL".to_string(),
            asset_id: None,
            quantity: sol_balance,
            decimals: 9,
        }];
        balances.extend(spl_balances);

        let extras = if stake_accounts.is_empty() {
            None
        } else {
            Some(ChainExtras::Solana { stake_accounts })
        };

        // Transaction count requires a separate RPC call — use 0 as a sentinel
        // (full count available via get_transactions if needed).
        Ok(AddressSummary {
            address: address.to_string(),
            chain: Chain::Solana,
            balances,
            transaction_count: 0,
            extras,
        })
    }

    async fn get_transactions(
        &self,
        address: &str,
        since_block: Option<u64>,
    ) -> Result<Vec<WalletTransaction>> {
        let signatures = self.get_signatures(address, since_block).await?;

        let mut txs = Vec::new();

        for sig in signatures {
            let result = self
                .post_rpc(
                    "getTransaction",
                    json!([sig, {"encoding": "jsonParsed", "commitment": "finalized", "maxSupportedTransactionVersion": 0}]),
                )
                .await;

            let tx_data = match result {
                Ok(v) if !v.is_null() => v,
                _ => continue,
            };

            let slot = tx_data["slot"].as_u64();

            // Apply since_block filter (slot ≈ block height on Solana)
            if let (Some(min), Some(s)) = (since_block, slot) {
                if s < min {
                    continue;
                }
            }

            let block_time = tx_data["blockTime"].as_i64();
            let timestamp = block_time
                .and_then(|t| DateTime::from_timestamp(t, 0))
                .unwrap_or_else(Utc::now);

            // Determine direction from pre/post balances
            let pre_balances = tx_data["meta"]["preBalances"].as_array();
            let post_balances = tx_data["meta"]["postBalances"].as_array();
            let account_keys = tx_data["transaction"]["message"]["accountKeys"].as_array();

            let direction = if let (Some(pre), Some(post), Some(keys)) =
                (pre_balances, post_balances, account_keys)
            {
                // Find the index of our address in account keys
                let idx = keys.iter().position(|k| {
                    k.as_str()
                        .or_else(|| k["pubkey"].as_str())
                        .map(|s| s.eq_ignore_ascii_case(address))
                        .unwrap_or(false)
                });

                if let Some(i) = idx {
                    let pre_lamports = pre.get(i).and_then(|v| v.as_i64()).unwrap_or(0);
                    let post_lamports = post.get(i).and_then(|v| v.as_i64()).unwrap_or(0);
                    let delta = post_lamports - pre_lamports;
                    if delta > 0 {
                        TransactionDirection::Incoming
                    } else if delta < 0 {
                        TransactionDirection::Outgoing
                    } else {
                        TransactionDirection::Internal
                    }
                } else {
                    TransactionDirection::Internal
                }
            } else {
                TransactionDirection::Internal
            };

            let fee_lamports = tx_data["meta"]["fee"].as_u64().unwrap_or(0);
            let fee = Some(Decimal::from(fee_lamports) / Decimal::from(1_000_000_000u64));

            // Net SOL amount from pre/post balance delta of our address
            let amount = {
                let pre_balances = tx_data["meta"]["preBalances"].as_array();
                let post_balances = tx_data["meta"]["postBalances"].as_array();
                let account_keys = tx_data["transaction"]["message"]["accountKeys"].as_array();

                if let (Some(pre), Some(post), Some(keys)) =
                    (pre_balances, post_balances, account_keys)
                {
                    let idx = keys.iter().position(|k| {
                        k.as_str()
                            .or_else(|| k["pubkey"].as_str())
                            .map(|s| s.eq_ignore_ascii_case(address))
                            .unwrap_or(false)
                    });
                    if let Some(i) = idx {
                        let pre_l = pre.get(i).and_then(|v| v.as_i64()).unwrap_or(0);
                        let post_l = post.get(i).and_then(|v| v.as_i64()).unwrap_or(0);
                        let delta = (post_l - pre_l).abs();
                        Decimal::from(delta as u64) / Decimal::from(1_000_000_000u64)
                    } else {
                        Decimal::ZERO
                    }
                } else {
                    Decimal::ZERO
                }
            };

            txs.push(WalletTransaction {
                external_id: sig,
                direction,
                amount,
                asset: "SOL".to_string(),
                fee,
                fee_asset: Some("SOL".to_string()),
                block_height: slot,
                timestamp,
                counterparty: None,
                memo: None,
            });
        }

        Ok(txs)
    }

    async fn get_chain_extras(&self, address: &str) -> Result<Option<ChainExtras>> {
        let stake_accounts = self.get_stake_accounts(address).await.unwrap_or_default();
        if stake_accounts.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ChainExtras::Solana { stake_accounts }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SolanaRpcClient::new("https://mainnet.helius-rpc.com".to_string());
        assert_eq!(client.provider_name(), "Solana RPC");
        assert_eq!(client.rpc_url, "https://mainnet.helius-rpc.com");
    }
}
