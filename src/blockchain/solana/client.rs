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
    AddressSummary, Chain, ChainExtras, DatedReward, DatedRewardBatch, HealthStatus,
    RewardDecodeSkip, SolanaStakeAccount, TransactionDirection, WalletBalance, WalletTransaction,
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

/// Resolve a known DePIN reward-token mint to (symbol, name).
///
/// These tokens are not in Jupiter's *strict* token list, so the sync would
/// otherwise label them with a truncated mint. Mapped by canonical mint:
///   - GEOD   (Geodnet)  — GPS/RTK DePIN miner rewards
///   - WINGS  (Wingbits) — ADS-B flight-data DePIN, a Token-2022 mint
fn known_depin_token(mint: &str) -> Option<(&'static str, &'static str)> {
    match mint {
        "7JA5eZdCzztSfQbJvS8aVVxMFfd81Rs9VvwnocV1mKHu" => Some(("GEOD", "Geodnet Token")),
        "WingsAYbfs4qnEgcw8jpSvetqp8XHM3GkKvow54WLcd" => Some(("WINGS", "Wingbits")),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// SolanaRpcClient
// ---------------------------------------------------------------------------

/// A wallet's SPL token account for a known DePIN reward mint.
#[derive(Debug, Clone)]
struct DepinTokenAccount {
    /// Token-account pubkey — `getSignaturesForAddress` is paged on THIS, not
    /// on the owner (owner-level history misses distributor rewards).
    pubkey: String,
    symbol: String,
    decimals: u8,
}

/// One entry from `getSignaturesForAddress`.
#[derive(Debug, Deserialize)]
struct SignatureEntry {
    signature: String,
    #[serde(default)]
    slot: u64,
    /// Present (non-null) when the transaction failed.
    #[serde(default)]
    err: Option<Value>,
}

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

        // Retry up to 3 times on 429 rate-limit with exponential backoff.
        let mut delay_ms = 1_000u64;
        for attempt in 0..3 {
            let response = self
                .http
                .post(&self.rpc_url)
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    CryptofolioError::Network(format!("Solana RPC request failed: {}", e))
                })?;

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2;
                    continue;
                }
                return Err(CryptofolioError::Network(
                    "Solana RPC rate limited (429)".to_string(),
                ));
            }

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

            return Ok(json["result"].clone());
        }

        Err(CryptofolioError::Network(
            "Solana RPC: exceeded retry limit".to_string(),
        ))
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
        // Query BOTH token programs: the legacy SPL Token program and Token-2022
        // (Token Extensions). DePIN reward tokens such as Wingbits (WINGS) are
        // minted under Token-2022, so querying only the legacy program silently
        // misses them. Program ids:
        //   legacy  : TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        //   2022    : TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
        const TOKEN_PROGRAMS: [&str; 2] = [
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
        ];

        let _ = self.ensure_token_list().await;
        let token_list = self.token_list.read().await;

        let mut balances = Vec::new();

        for program_id in TOKEN_PROGRAMS {
            let result = self
                .post_rpc(
                    "getTokenAccountsByOwner",
                    json!([
                        address,
                        {"programId": program_id},
                        {"encoding": "jsonParsed"}
                    ]),
                )
                .await?;

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
                    } else if let Some((sym, name)) = known_depin_token(&mint) {
                        // DePIN reward tokens are absent from Jupiter's strict
                        // list; resolve the ones we know by mint so they label
                        // correctly instead of showing a truncated mint.
                        (sym.to_string(), name.to_string())
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
        }

        Ok(balances)
    }

    async fn get_stake_accounts(&self, address: &str) -> Result<Vec<SolanaStakeAccount>> {
        let result = self
            .post_rpc(
                "getProgramAccounts",
                json!([
                    // Canonical Solana Stake program id. (A previous value had
                    // extra trailing 1s and was rejected with INVALID_PARAMS, so
                    // stake accounts were silently never fetched — staked SOL was
                    // invisible to the balance.)
                    "Stake11111111111111111111111111111111111111",
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

    // -----------------------------------------------------------------------
    // Dated DePIN reward history
    // -----------------------------------------------------------------------

    /// Resolve the wallet's token accounts for the known DePIN reward mints.
    ///
    /// Queries BOTH token programs (legacy + Token-2022), exactly like
    /// `get_spl_balances`. Zero-balance accounts are KEPT: the reward tokens may
    /// since have been moved out, but the dated income still happened. This uses
    /// `getTokenAccountsByOwner` rather than deriving the ATA with PDA math so we
    /// reuse the existing plumbing (and it also finds non-canonical accounts).
    async fn depin_token_accounts(&self, owner: &str) -> Result<Vec<DepinTokenAccount>> {
        const TOKEN_PROGRAMS: [&str; 2] = [
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
        ];

        let mut accounts = Vec::new();

        for program_id in TOKEN_PROGRAMS {
            let result = self
                .post_rpc(
                    "getTokenAccountsByOwner",
                    json!([
                        owner,
                        {"programId": program_id},
                        {"encoding": "jsonParsed"}
                    ]),
                )
                .await?;

            let entries = match result["value"].as_array() {
                Some(entries) => entries,
                None => continue,
            };

            for entry in entries {
                let info = &entry["account"]["data"]["parsed"]["info"];
                let mint = info["mint"].as_str().unwrap_or("");
                let (symbol, _name) = match known_depin_token(mint) {
                    Some(known) => known,
                    None => continue, // not a DePIN reward mint we track
                };
                let pubkey = entry["pubkey"].as_str().unwrap_or("").to_string();
                if pubkey.is_empty() {
                    continue;
                }
                let decimals = info["tokenAmount"]["decimals"].as_u64().unwrap_or(0) as u8;
                accounts.push(DepinTokenAccount {
                    pubkey,
                    symbol: symbol.to_string(),
                    decimals,
                });
            }
        }

        Ok(accounts)
    }

    /// Signature entries (with slot + err) for an account. Richer than
    /// `get_signatures` because reward decoding needs the block time (read from
    /// the transaction) and must skip failed transactions.
    async fn get_signature_entries(&self, address: &str) -> Result<Vec<SignatureEntry>> {
        let result = self
            .post_rpc("getSignaturesForAddress", json!([address, {"limit": 1000}]))
            .await?;

        serde_json::from_value(result).map_err(|e| {
            CryptofolioError::Network(format!(
                "Failed to decode getSignaturesForAddress response: {}",
                e
            ))
        })
    }

    /// List dated incoming SPL reward transfers for the wallet's known DePIN
    /// token accounts (procedure P5 in `docs/MINING_ASSET_ACCOUNTING.md`).
    ///
    /// Every INCOMING transfer to a tracked token account is treated as a reward
    /// (the P5 assumption: the miner payout account only receives distributions).
    /// A DEX purchase landing in the same token account would be misclassified;
    /// filtering by sender is out of scope until a distributor allow-list exists.
    /// Only the most recent `limit` signatures per token account are scanned.
    ///
    /// Failure behaviour: an RPC error propagates (never swallowed). A single
    /// undecodable transaction is SKIPPED into `DatedRewardBatch::skipped` with
    /// a reason, so one bad row cannot drop the others.
    async fn scan_dated_rewards(
        &self,
        address: &str,
        since_block: Option<u64>,
    ) -> Result<DatedRewardBatch> {
        let mut batch = DatedRewardBatch::default();

        for account in self.depin_token_accounts(address).await? {
            let entries = self.get_signature_entries(&account.pubkey).await?;

            for entry in entries {
                // A failed transaction never moved tokens — nothing to book.
                if entry.err.is_some() {
                    continue;
                }
                // `getSignaturesForAddress` returns slots; mirror the
                // `since_block` semantics of `get_transactions`.
                if let Some(min) = since_block {
                    if entry.slot < min {
                        continue;
                    }
                }

                let signature = entry.signature;
                let tx = self
                    .post_rpc(
                        "getTransaction",
                        json!([
                            signature,
                            {
                                "encoding": "jsonParsed",
                                "commitment": "finalized",
                                "maxSupportedTransactionVersion": 0
                            }
                        ]),
                    )
                    .await?;

                if tx.is_null() {
                    // Pruned/not-yet-available: report, never drop silently.
                    batch.skipped.push(RewardDecodeSkip {
                        signature,
                        reason: "transaction not available from RPC".to_string(),
                    });
                    continue;
                }

                match decode_incoming_reward(
                    &tx,
                    &signature,
                    &account.pubkey,
                    &account.symbol,
                    account.decimals,
                ) {
                    Ok(Some(reward)) => batch.rewards.push(reward),
                    // No incoming transfer to this token account — the tx is
                    // simply not a reward (e.g. an outgoing transfer).
                    Ok(None) => {}
                    Err(reason) => batch.skipped.push(RewardDecodeSkip { signature, reason }),
                }
            }
        }

        Ok(batch)
    }
}

/// Scale a raw SPL amount (integer base units) by the mint's decimals.
fn scale_amount(raw: u64, decimals: u8) -> std::result::Result<Decimal, String> {
    let factor = 10u64
        .checked_pow(u32::from(decimals))
        .ok_or_else(|| format!("implausible token decimals: {decimals}"))?;
    Ok(Decimal::from(raw) / Decimal::from(factor))
}

/// Add the raw base-unit amounts of SPL `transfer*` instructions that credit
/// `token_account`. Unreadable parsed instructions are ignored, never treated as
/// a transfer.
fn scan_instructions(instructions: &[Value], token_account: &str, total: &mut u64) {
    for ix in instructions {
        let parsed = &ix["parsed"];
        let kind = match parsed["type"].as_str() {
            Some(kind) => kind,
            None => continue,
        };
        // transfer / transferChecked / transferCheckedWithFee (Token-2022)
        if !kind.starts_with("transfer") {
            continue;
        }
        let info = &parsed["info"];
        if info["destination"].as_str() != Some(token_account) {
            continue;
        }
        let raw = info["tokenAmount"]["amount"]
            .as_str()
            .or_else(|| info["amount"].as_str())
            .and_then(|s| s.parse::<u64>().ok());
        if let Some(raw) = raw {
            *total = total.saturating_add(raw);
        }
    }
}

/// Decode one transaction into a dated reward credit for `token_account`.
///
/// * `Ok(Some(reward))` — at least one incoming transfer to the account.
/// * `Ok(None)` — the transaction contains no such transfer (not a reward).
/// * `Err(reason)` — the transaction shape could not be decoded.
fn decode_incoming_reward(
    tx: &Value,
    signature: &str,
    token_account: &str,
    symbol: &str,
    decimals: u8,
) -> std::result::Result<Option<DatedReward>, String> {
    let message = tx
        .get("transaction")
        .and_then(|t| t.get("message"))
        .ok_or_else(|| "missing transaction.message".to_string())?;
    let instructions = message
        .get("instructions")
        .and_then(|i| i.as_array())
        .ok_or_else(|| "missing transaction.message.instructions".to_string())?;

    let mut raw_total: u64 = 0;
    scan_instructions(instructions, token_account, &mut raw_total);

    // Rewards are often distributed through a CPI, so the transfer instruction
    // lives in `meta.innerInstructions`, not the outer message.
    if let Some(inner_groups) = tx["meta"]["innerInstructions"].as_array() {
        for group in inner_groups {
            if let Some(inner) = group["instructions"].as_array() {
                scan_instructions(inner, token_account, &mut raw_total);
            }
        }
    }

    if raw_total == 0 {
        return Ok(None);
    }

    let block_time = tx["blockTime"]
        .as_i64()
        .ok_or_else(|| "missing blockTime".to_string())?;
    let date = DateTime::from_timestamp(block_time, 0)
        .ok_or_else(|| format!("out-of-range blockTime: {block_time}"))?;
    let quantity = scale_amount(raw_total, decimals)?;

    Ok(Some(DatedReward {
        date,
        asset: symbol.to_string(),
        quantity,
        signature: signature.to_string(),
    }))
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

        // Staked SOL lives in separate stake accounts, not the wallet's liquid
        // balance. Include it in the SOL holding so the balance reflects total
        // SOL owned (liquid + staked); the per-account detail stays in extras.
        let staked_lamports: u64 = stake_accounts.iter().map(|s| s.lamports).sum();
        let staked_sol = Decimal::from(staked_lamports) / Decimal::from(1_000_000_000u64);
        let total_sol = sol_balance + staked_sol;

        let mut balances = vec![WalletBalance {
            asset: "SOL".to_string(),
            asset_id: None,
            quantity: total_sol,
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

    async fn get_dated_rewards(
        &self,
        address: &str,
        since_block: Option<u64>,
    ) -> Result<DatedRewardBatch> {
        self.scan_dated_rewards(address, since_block).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const GEOD_MINT: &str = "7JA5eZdCzztSfQbJvS8aVVxMFfd81Rs9VvwnocV1mKHu";
    const LEGACY_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

    #[test]
    fn test_client_creation() {
        let client = SolanaRpcClient::new("https://mainnet.helius-rpc.com".to_string());
        assert_eq!(client.provider_name(), "Solana RPC");
        assert_eq!(client.rpc_url, "https://mainnet.helius-rpc.com");
    }

    /// A JSON-RPC success response with `result` set to `value`.
    fn rpc_ok(value: serde_json::Value) -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_json(json!({"jsonrpc": "2.0", "id": 1, "result": value}))
    }

    /// Real-world-shaped `getTokenAccountsByOwner` response for one GEOD ATA.
    fn token_accounts_body(owner: &str, ata: &str) -> serde_json::Value {
        json!({
            "context": {"slot": 100},
            "value": [{
                "pubkey": ata,
                "account": {
                    "owner": LEGACY_PROGRAM,
                    "data": {
                        "parsed": {
                            "info": {
                                "owner": owner,
                                "mint": GEOD_MINT,
                                "tokenAmount": {
                                    "amount": "20000000000",
                                    "decimals": 9,
                                    "uiAmount": 20.0,
                                    "uiAmountString": "20"
                                }
                            },
                            "type": "account"
                        },
                        "program": "spl-token"
                    }
                }
            }]
        })
    }

    /// A distributor reward paid through a CPI `transferChecked` — the transfer
    /// instruction lives in `meta.innerInstructions`, not the outer message.
    fn reward_tx_body(ata: &str, raw_amount: &str, block_time: i64) -> serde_json::Value {
        json!({
            "slot": 100,
            "blockTime": block_time,
            "meta": {
                "err": null,
                "fee": 5000,
                "innerInstructions": [{
                    "index": 0,
                    "instructions": [{
                        "program": "spl-token",
                        "programId": LEGACY_PROGRAM,
                        "parsed": {
                            "type": "transferChecked",
                            "info": {
                                "source": "DistributorTokenAccount1111111111111111111",
                                "destination": ata,
                                "authority": "DistributorAuthority111111111111111111",
                                "mint": GEOD_MINT,
                                "tokenAmount": {
                                    "amount": raw_amount,
                                    "decimals": 9,
                                    "uiAmount": 12.5,
                                    "uiAmountString": "12.5"
                                }
                            }
                        }
                    }]
                }]
            },
            "transaction": {
                "signatures": [],
                "message": {
                    "accountKeys": [],
                    "instructions": [{
                        "programId": "DistributorProgram111111111111111111111111",
                        "accounts": [],
                        "data": "3Bxs4h24hBtQy"
                    }]
                }
            }
        })
    }

    #[tokio::test]
    async fn decodes_dated_depin_rewards_and_reports_skips() {
        let server = MockServer::start().await;
        let owner = "MinerWalletOwner1111111111111111111111111111";
        let ata = "GeoDTokenAccount11111111111111111111111111111";
        let good_sig = "5GoodRewardSignature11111111111111111111111111111111111111111";
        let malformed_sig = "4MalformedSignature111111111111111111111111111111111111111";
        let outgoing_sig = "3OutgoingSignature1111111111111111111111111111111111111111";
        let failed_sig = "2FailedSignature111111111111111111111111111111111111111111";

        // Token-account discovery (both programs; Token-2022 empty here).
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains(
                "\"method\":\"getTokenAccountsByOwner\"",
            ))
            .and(body_string_contains(LEGACY_PROGRAM))
            .respond_with(rpc_ok(token_accounts_body(owner, ata)))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains(
                "\"method\":\"getTokenAccountsByOwner\"",
            ))
            .and(body_string_contains(TOKEN_2022_PROGRAM))
            .respond_with(rpc_ok(json!({"context": {"slot": 100}, "value": []})))
            .mount(&server)
            .await;

        // Token-account signature history: one good, one malformed, one
        // outgoing, and one FAILED tx (which must never be fetched).
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("\"method\":\"getSignaturesForAddress\""))
            .respond_with(rpc_ok(json!([
                {"signature": good_sig, "slot": 100, "blockTime": 1_728_000_000i64, "err": null},
                {"signature": malformed_sig, "slot": 101, "blockTime": 1_728_003_600i64, "err": null},
                {"signature": outgoing_sig, "slot": 102, "blockTime": 1_728_007_200i64, "err": null},
                {"signature": failed_sig, "slot": 103, "blockTime": 1_728_010_800i64,
                 "err": {"InstructionError": [0, "Custom"]}}
            ])))
            .mount(&server)
            .await;

        // 12.5 GEOD credited through an inner (CPI) transferChecked.
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("\"method\":\"getTransaction\""))
            .and(body_string_contains(good_sig))
            .respond_with(rpc_ok(reward_tx_body(ata, "12500000000", 1_728_000_000)))
            .mount(&server)
            .await;

        // A successful tx whose body is truncated → decode skip, not a crash.
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("\"method\":\"getTransaction\""))
            .and(body_string_contains(malformed_sig))
            .respond_with(rpc_ok(json!({
                "slot": 101,
                "blockTime": 1_728_003_600i64,
                "meta": {"err": null}
            })))
            .mount(&server)
            .await;

        // A transfer FROM the token account to someone else → not a reward.
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("\"method\":\"getTransaction\""))
            .and(body_string_contains(outgoing_sig))
            .respond_with(rpc_ok(json!({
                "slot": 102,
                "blockTime": 1_728_007_200i64,
                "meta": {"err": null},
                "transaction": {
                    "signatures": [],
                    "message": {
                        "accountKeys": [],
                        "instructions": [{
                            "program": "spl-token",
                            "programId": LEGACY_PROGRAM,
                            "parsed": {
                                "type": "transfer",
                                "info": {
                                    "source": ata,
                                    "destination": "SomeoneElseTokenAccount11111111111111111",
                                    "amount": "5000000000",
                                    "authority": owner
                                }
                            }
                        }]
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = SolanaRpcClient::new(server.uri());
        let batch = client
            .get_dated_rewards(owner, None)
            .await
            .expect("reward scan");

        assert_eq!(batch.rewards.len(), 1, "rewards: {:?}", batch.rewards);
        let reward = &batch.rewards[0];
        assert_eq!(reward.signature, good_sig);
        assert_eq!(reward.asset, "GEOD");
        assert_eq!(reward.quantity, Decimal::from_str_exact("12.5").unwrap());
        assert_eq!(
            reward.date,
            DateTime::from_timestamp(1_728_000_000, 0).unwrap()
        );

        // The malformed tx is reported; the failed and outgoing txs are not
        // skips (one never moved tokens, the other is simply not a reward).
        assert_eq!(batch.skipped.len(), 1, "skips: {:?}", batch.skipped);
        assert_eq!(batch.skipped[0].signature, malformed_sig);
        assert!(batch.skipped[0].reason.contains("transaction.message"));
    }

    #[tokio::test]
    async fn since_block_filters_old_reward_slots() {
        let server = MockServer::start().await;
        let owner = "MinerWalletOwner2222222222222222222222222222";
        let ata = "GeoDTokenAccount22222222222222222222222222222";
        let old_sig = "1OldRewardSignature1111111111111111111111111111111111111111";

        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains(
                "\"method\":\"getTokenAccountsByOwner\"",
            ))
            .and(body_string_contains(LEGACY_PROGRAM))
            .respond_with(rpc_ok(token_accounts_body(owner, ata)))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains(
                "\"method\":\"getTokenAccountsByOwner\"",
            ))
            .and(body_string_contains(TOKEN_2022_PROGRAM))
            .respond_with(rpc_ok(json!({"value": []})))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains(
                "\"method\":\"getSignaturesForAddress\"",
            ))
            .respond_with(rpc_ok(json!([
                {"signature": old_sig, "slot": 40, "blockTime": 1_728_000_000i64, "err": null}
            ])))
            .mount(&server)
            .await;
        // Deliberately no getTransaction mock: if the slot filter fails, the
        // client fetches this signature and the 404 surfaces as an error.

        let client = SolanaRpcClient::new(server.uri());
        let batch = client
            .get_dated_rewards(owner, Some(41))
            .await
            .expect("reward scan with watermark");

        assert!(batch.rewards.is_empty());
        assert!(batch.skipped.is_empty());
    }

    #[test]
    fn decode_incoming_reward_handles_directions_and_variants() {
        let ata = "GeoDTokenAccount11111111111111111111111111111";
        let tx = json!({
            "blockTime": 1_728_000_000i64,
            "meta": {
                "innerInstructions": [{
                    "index": 1,
                    "instructions": [
                        // Token-2022 style with-fee transfer → counts.
                        {"parsed": {"type": "transferCheckedWithFee", "info": {
                            "source": "S", "destination": ata,
                            "tokenAmount": {"amount": "250000000", "decimals": 9}
                        }}},
                        // To a different account → ignored.
                        {"parsed": {"type": "transfer", "info": {
                            "source": "S", "destination": "Other11111111111111111111111111111111111111",
                            "amount": "999999999999"
                        }}}
                    ]
                }]
            },
            "transaction": {"message": {"instructions": [
                // Plain transfer to us → counts.
                {"parsed": {"type": "transfer", "info": {
                    "source": "S", "destination": ata, "amount": "1000000000"
                }}},
                // Our own outgoing transfer → ignored.
                {"parsed": {"type": "transfer", "info": {
                    "source": ata, "destination": "Other11111111111111111111111111111111111111",
                    "amount": "7777777777"
                }}},
                // Non-transfer instruction → ignored.
                {"parsed": {"type": "approve", "info": {"source": ata, "amount": "1"}}}
            ]}}
        });

        let decoded = decode_incoming_reward(&tx, "sig", ata, "GEOD", 9)
            .expect("well-formed tx must decode")
            .expect("an incoming transfer exists");
        // 1.0 + 0.25 = 1.25 (the outgoing and other-account amounts are excluded).
        assert_eq!(decoded.quantity, Decimal::from_str_exact("1.25").unwrap());
        assert_eq!(decoded.asset, "GEOD");
        assert_eq!(decoded.signature, "sig");
    }

    #[test]
    fn decode_incoming_reward_reports_malformed_shape() {
        let err = decode_incoming_reward(&json!({"blockTime": 1}), "sig", "ata", "GEOD", 9)
            .expect_err("missing transaction.message must not silently decode");
        assert!(err.contains("transaction.message"));

        // A well-shaped tx with no matching transfer is `Ok(None)`, not an error.
        let none = decode_incoming_reward(
            &json!({
                "blockTime": 1,
                "transaction": {"message": {"instructions": []}}
            }),
            "sig",
            "ata",
            "GEOD",
            9,
        )
        .expect("no incoming transfer is not an error");
        assert!(none.is_none());

        // An incoming transfer without a block time cannot be dated → skip.
        let err = decode_incoming_reward(
            &json!({
                "transaction": {"message": {"instructions": [
                    {"parsed": {"type": "transfer", "info": {
                        "destination": "ata", "amount": "1000000000"
                    }}}
                ]}}
            }),
            "sig",
            "ata",
            "GEOD",
            9,
        )
        .expect_err("an undated reward must be reported, not booked at Utc::now()");
        assert!(err.contains("blockTime"));
    }

    #[test]
    fn scale_amount_rejects_implausible_decimals() {
        assert_eq!(
            scale_amount(1_250_000_000, 9).expect("9 decimals is fine"),
            Decimal::from_str_exact("1.25").unwrap()
        );
        // 10^20 overflows u64 → fail closed instead of panicking.
        assert!(scale_amount(1, 20).is_err());
    }
}
