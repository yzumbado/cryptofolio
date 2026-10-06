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

    /// GET a URL and return the raw body, retrying on Etherscan's per-second
    /// rate-limit response. The free tier caps at ~3–5 calls/sec and returns
    /// `{"status":"0","message":"NOTOK","result":"Max calls per sec rate limit
    /// reached (N/sec)"}` when exceeded — a transient, retryable condition, not
    /// a real error. Backs off (350ms → 2.8s) up to 4 retries.
    async fn fetch_text_with_retry(&self, url: &str) -> Result<String> {
        let mut delay_ms = 350u64;
        let mut last_err = String::new();
        for attempt in 0..5 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(2800);
            }
            let resp = reqwest::get(url)
                .await
                .map_err(|e| CryptofolioError::Network(format!("Request failed: {}", e)))?;
            if !resp.status().is_success() {
                last_err = format!("HTTP {}", resp.status());
                continue;
            }
            let body = resp
                .text()
                .await
                .map_err(|e| CryptofolioError::Network(format!("Read body failed: {}", e)))?;
            // Retry only on the rate-limit signal; every other body is returned as-is.
            if body.contains("rate limit reached") || body.contains("Max calls per sec") {
                last_err = "Etherscan rate limit".to_string();
                continue;
            }
            return Ok(body);
        }
        Err(CryptofolioError::Network(format!(
            "Etherscan API error after retries: {}",
            last_err
        )))
    }

    /// Read a liquid-staking token's ETH exchange rate via `eth_call`.
    ///
    /// `contract` is the LST token contract; `selector` is the 4-byte method
    /// selector for its rate getter (rETH `getExchangeRate()` = `0xe6aa216c`,
    /// wstETH `stEthPerToken()` = `0x035faf82`). Both return a `uint256`
    /// 18-decimal fixed-point value = how many ETH (stETH≈ETH) one LST token is
    /// worth. Returns that ratio as a Decimal (e.g. ~1.17 for rETH). Fetched
    /// LIVE because LST:ETH ratios drift upward as staking rewards accrue.
    pub async fn get_lst_eth_rate(&self, contract: &str, selector: &str) -> Result<Decimal> {
        let mut url = format!(
            "{}?chainid={}&module=proxy&action=eth_call&to={}&data={}&tag=latest",
            self.base_url, self.chain_id, contract, selector
        );
        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let body = self.fetch_text_with_retry(&url).await?;

        #[derive(serde::Deserialize)]
        struct EthCallResponse {
            result: Option<String>,
        }
        let data: EthCallResponse = serde_json::from_str(&body)
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse eth_call: {}", e)))?;

        let hex = data
            .result
            .ok_or_else(|| CryptofolioError::Network("eth_call returned no result".into()))?;
        let hex = hex.trim_start_matches("0x");
        // uint256 fits in u128 for any realistic exchange rate (< 2^128 wei-scaled).
        let raw = u128::from_str_radix(hex, 16)
            .map_err(|e| CryptofolioError::Other(format!("Invalid eth_call rate: {}", e)))?;
        let rate = Decimal::from(raw) / Decimal::from(1_000_000_000_000_000_000u64);
        Ok(rate)
    }

    /// Generic read-only `eth_call` via Etherscan's proxy module. Returns the
    /// raw `result` hex (with the `0x` prefix). Callers decode it.
    pub async fn eth_call(&self, to: &str, calldata: &str) -> Result<String> {
        let mut url = format!(
            "{}?chainid={}&module=proxy&action=eth_call&to={}&data={}&tag=latest",
            self.base_url, self.chain_id, to, calldata
        );
        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let body = self.fetch_text_with_retry(&url).await?;

        #[derive(serde::Deserialize)]
        struct EthCallResponse {
            result: Option<String>,
        }
        let resp: EthCallResponse = serde_json::from_str(&body)
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse eth_call: {}", e)))?;

        resp.result
            .ok_or_else(|| CryptofolioError::Network("eth_call returned no result".into()))
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

        let body = self.fetch_text_with_retry(&url).await?;
        let data: EtherscanResponse = serde_json::from_str(&body)
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

        let body = self.fetch_text_with_retry(&url).await?;
        let data: EtherscanTokenResponse = serde_json::from_str(&body)
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

    /// Fetch ERC-20 token transfers since `start_block` as ledger transactions.
    async fn fetch_token_transfers_since(
        &self,
        address: &str,
        start_block: u64,
    ) -> Result<Vec<WalletTransaction>> {
        let mut url = format!(
            "{}?chainid={}&module=account&action=tokentx&address={}&startblock={}&endblock=99999999&sort=asc",
            self.base_url, self.chain_id, address, start_block
        );

        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let body = self.fetch_text_with_retry(&url).await?;
        let data: EtherscanTokenResponse = serde_json::from_str(&body)
            .map_err(|e| CryptofolioError::Network(format!("Failed to parse tokentx: {}", e)))?;

        if data.status != "1" {
            // "No transactions found" is a valid empty result.
            if data.message.contains("No transactions found")
                || data.message.contains("No matching")
            {
                return Ok(Vec::new());
            }
            return Err(CryptofolioError::Network(format!(
                "Etherscan API error: {}",
                data.message
            )));
        }

        Ok(data
            .result
            .iter()
            .filter_map(|t| token_transfer_to_wallet_tx(t, address))
            .collect())
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

        let body = self.fetch_text_with_retry(&url).await?;
        let data: EtherscanTxResponse = serde_json::from_str(&body)
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

    /// Get internal transactions (contract-originated ETH transfers) since a block.
    async fn fetch_internal_transactions_since(
        &self,
        address: &str,
        start_block: u64,
    ) -> Result<Vec<EthereumTransaction>> {
        let mut url = format!(
            "{}?chainid={}&module=account&action=txlistinternal&address={}&startblock={}&endblock=99999999&sort=asc",
            self.base_url, self.chain_id, address, start_block
        );
        if let Some(key) = &self.api_key {
            url.push_str(&format!("&apikey={}", key));
        }

        let data: EtherscanTxResponse = match self.fetch_text_with_retry(&url).await {
            Ok(body) => match serde_json::from_str(&body) {
                Ok(d) => d,
                Err(_) => return Ok(Vec::new()),
            },
            Err(_) => return Ok(Vec::new()), // Non-fatal: internal txs may not exist
        };

        if data.status != "1" {
            return Ok(Vec::new()); // "No transactions found" is normal
        }

        let mut result = Vec::new();
        for tx in data.result {
            let value_wei = Decimal::from_str(&tx.value).unwrap_or(Decimal::ZERO);
            let value_eth = value_wei / Decimal::from(1_000_000_000_000_000_000u64);

            result.push(EthereumTransaction {
                hash: tx.hash,
                from: tx.from,
                to: tx.to,
                value: value_eth,
                gas_used: tx.gas_used.parse().unwrap_or(0),
                gas_price: Decimal::ZERO, // internal txs don't have their own gas price
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

        let body = self.fetch_text_with_retry(&url).await?;
        let data: EtherscanTxResponse = serde_json::from_str(&body)
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
            // Skip tokens flagged as scams (Unicode-lookalike symbols, empty
            // symbol, known scam contracts) — they are airdropped to poison the
            // ledger and have no real value. They are not persisted as holdings.
            if is_likely_scam_token(&token.symbol, &token.contract_address) {
                continue;
            }
            // A token must never masquerade as the native asset. A scam ERC-20
            // reporting symbol "ETH" would otherwise OVERWRITE the real native
            // ETH balance (holdings are keyed by symbol). Qualify any collision
            // with its contract address so native "ETH" is always genuine.
            let asset = native_safe_asset_key(&token.symbol, &token.contract_address);
            balances.push(WalletBalance {
                asset,
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
        let start_block = since_block.unwrap_or(0);

        // Fetch normal then internal transactions SEQUENTIALLY. The Etherscan
        // free tier caps at ~3–5 calls/sec; firing both in parallel (tokio::join)
        // reliably trips "Max calls per sec rate limit reached", whose string
        // `result` body would otherwise be dropped as an empty list — silently
        // losing internal txs. A short spacer keeps us under the limit.
        let normal_result = self.fetch_transactions_since(address, start_block).await;
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let internal_result = self
            .fetch_internal_transactions_since(address, start_block)
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let token_result = self.fetch_token_transfers_since(address, start_block).await;

        let mut raw = normal_result?;
        // Internal txs are best-effort; ignore errors (e.g. no API key)
        if let Ok(internal) = internal_result {
            raw.extend(internal);
        }

        // Sort merged list by block number then hash for deterministic ordering
        raw.sort_by(|a, b| {
            a.block_number
                .cmp(&b.block_number)
                .then(a.hash.cmp(&b.hash))
        });

        // Deduplicate by hash (same tx can appear in both lists)
        raw.dedup_by(|a, b| a.hash == b.hash);

        let mut txs: Vec<WalletTransaction> = raw
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

        // ERC-20 token transfers are best-effort (like internal txs): a missing
        // API key or a token-transfer failure must not drop the native history.
        if let Ok(tokens) = token_result {
            txs.extend(tokens);
        }

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

/// Deserialize an Etherscan `result` that is normally an array but becomes a
/// bare string on error (e.g. `"Max calls per sec rate limit reached (3/sec)"`
/// or `"No transactions found"`). Yields an empty Vec for the non-array case so
/// a rate-limit/error body never fails JSON decoding; callers gate on `status`.
fn de_result_array_or_empty<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Array(_) => serde_json::from_value(v).map_err(serde::de::Error::custom),
        _ => Ok(Vec::new()),
    }
}

#[derive(Debug, Deserialize)]
struct EtherscanTokenResponse {
    status: String,
    message: String,
    #[serde(deserialize_with = "de_result_array_or_empty")]
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
    #[serde(default)]
    hash: String,
    #[serde(default)]
    time_stamp: String,
    #[serde(default)]
    block_number: String,
}

#[derive(Debug, Deserialize)]
struct EtherscanTxResponse {
    status: String,
    message: String,
    #[serde(deserialize_with = "de_result_array_or_empty")]
    result: Vec<EthTransaction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthTransaction {
    hash: String,
    from: String,
    // Absent on contract-creation internal txs.
    #[serde(default)]
    to: String,
    value: String,
    #[allow(dead_code)] // Received from API but not used
    #[serde(default)]
    gas: String,
    // Etherscan V2 `txlistinternal` omits gasPrice (internal txs have no own gas price).
    #[serde(default)]
    gas_price: String,
    #[serde(default)]
    gas_used: String,
    time_stamp: String,
    #[serde(default)]
    is_error: String,
    block_number: String,
}

/// Build the holdings `asset` key for an ERC-20, guarding the native asset.
///
/// Holdings are keyed by symbol, so a scam ERC-20 that reports its symbol as
/// "ETH" (or an upper/lower-case variant) would overwrite the real native ETH
/// balance. Any such collision is disambiguated by qualifying it with the
/// token's contract address; every other token keeps its symbol.
fn native_safe_asset_key(symbol: &str, contract_address: &str) -> String {
    if symbol.eq_ignore_ascii_case("ETH") {
        format!("ETH:{contract_address}")
    } else {
        symbol.to_string()
    }
}

/// Map one ERC-20 token transfer to a ledger transaction, or `None` to skip it
/// (scam token, zero value, or a row that doesn't touch this address).
///
/// `external_id` is `{hash}-{contract}` — a single transaction can move several
/// ERC-20 tokens, and this suffix keeps each token's transfer unique under the
/// ledger's UNIQUE(external_id) constraint (the caller prefixes `ethereum-`).
fn token_transfer_to_wallet_tx(t: &TokenTransaction, address: &str) -> Option<WalletTransaction> {
    if is_likely_scam_token(&t.token_symbol, &t.contract_address) {
        return None;
    }
    let decimals = t.token_decimal.parse::<u32>().unwrap_or(18);
    let value = i128::from_str(&t.value).unwrap_or(0);
    if value == 0 {
        return None;
    }
    let direction = if t.to.eq_ignore_ascii_case(address) {
        TransactionDirection::Incoming
    } else if t.from.eq_ignore_ascii_case(address) {
        TransactionDirection::Outgoing
    } else {
        return None;
    };
    let asset = native_safe_asset_key(&t.token_symbol, &t.contract_address);
    let external_id = format!("{}-{}", t.hash, t.contract_address);
    let timestamp =
        DateTime::from_timestamp(t.time_stamp.parse().unwrap_or(0), 0).unwrap_or_else(Utc::now);
    let block_height = t.block_number.parse::<u64>().ok();

    Some(WalletTransaction {
        external_id,
        direction,
        amount: Decimal::from_i128_with_scale(value, decimals),
        asset,
        fee: None,
        fee_asset: None,
        block_height,
        timestamp,
        counterparty: Some(match direction {
            TransactionDirection::Incoming => t.from.clone(),
            _ => t.to.clone(),
        }),
        memo: None,
    })
}

/// Heuristic scam-token detector for ERC-20 holdings.
///
/// Address-poisoning and fake-airdrop scams send worthless tokens to a watched
/// address so they pollute the portfolio. They are identified by:
///   1. a non-ASCII symbol — Unicode look-alikes impersonating a real ticker
///      (e.g. "U5DТ", "ÚЅDТ", "Тoken" using Cyrillic/other scripts);
///   2. an empty symbol;
///   3. a control/zero-width character anywhere in the symbol;
///   4. an ASCII symbol that impersonates a major ticker (USDT/USDC/WETH/DAI)
///      from a contract that is NOT that token's canonical address.
///
/// Legit tokens use plain-ASCII tickers from their canonical contract, so this
/// is conservative — it flags look-alike impostors, not real holdings. The
/// native "ETH" collision is handled separately by `native_safe_asset_key`.
fn is_likely_scam_token(symbol: &str, contract_address: &str) -> bool {
    let s = symbol.trim();
    if s.is_empty() {
        return true;
    }
    // (1-3) Any non-ASCII or control character => look-alike / poisoned symbol.
    if s.chars()
        .any(|c| !c.is_ascii() || c.is_control() || c == '\u{200b}')
    {
        return true;
    }
    // (4) ASCII impersonation of a major stablecoin/ticker from a non-canonical
    // contract. "eth" has NO canonical ERC-20 (it is the native asset), so ANY
    // token calling itself ETH is an impostor and is dropped here.
    let sym = s.to_ascii_lowercase();
    if sym == "eth" {
        return true;
    }
    let canonical: &[(&str, &str)] = &[
        ("usdt", "0xdac17f958d2ee523a2206206994597c13d831ec7"),
        ("usdc", "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
        ("weth", "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"),
        ("dai", "0x6b175474e89094c44da98b954eedeac495271d0f"),
    ];
    for (ticker, addr) in canonical {
        if sym == *ticker && !contract_address.eq_ignore_ascii_case(addr) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_eth_symbol_collision_is_disambiguated() {
        // A scam token reporting symbol "ETH" must NOT keep the bare "ETH" key
        // (which would clobber the native balance in the symbol-keyed upsert).
        let scam = native_safe_asset_key("ETH", "0x2fc618b4e3a29bed734e6b2364e3497eb7370302");
        assert_eq!(scam, "ETH:0x2fc618b4e3a29bed734e6b2364e3497eb7370302");
        // Case-insensitive: "eth" collides too.
        assert_eq!(native_safe_asset_key("eth", "0xabc"), "ETH:0xabc");
        // A normal token is untouched.
        assert_eq!(native_safe_asset_key("RPL", "0xd33"), "RPL");
    }

    #[test]
    fn test_scam_token_detection() {
        // Unicode look-alikes impersonating USDT / token (seen in the wild).
        assert!(is_likely_scam_token("U5DТ", "0xd9a3")); // Cyrillic Т
        assert!(is_likely_scam_token("ÚЅDТ", "0xb802"));
        assert!(is_likely_scam_token("Тoken", "0xd017"));
        assert!(is_likely_scam_token("꒤5DT", "0xc0d6"));
        // Empty symbol is a scam signal.
        assert!(is_likely_scam_token("", "0xf839"));
        // ASCII "USDT" from a NON-canonical contract is an impostor.
        assert!(is_likely_scam_token(
            "USDT",
            "0x91fb15708f32603dfd6d22e04a27b034e56836f7"
        ));
        // Any token calling itself ETH is an impostor (ETH is native, no ERC-20).
        assert!(is_likely_scam_token(
            "ETH",
            "0x2fc618b4e3a29bed734e6b2364e3497eb7370302"
        ));
        // Real USDT from its canonical contract is NOT flagged.
        assert!(!is_likely_scam_token(
            "USDT",
            "0xdac17f958d2ee523a2206206994597c13d831ec7"
        ));
        // Legit ASCII tickers are NEVER flagged.
        for good in ["RPL", "rETH", "wstETH", "GHO", "aEthUSDT", "HEX"] {
            assert!(
                !is_likely_scam_token(good, "0x0"),
                "{good} must not be scam"
            );
        }
    }

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

    // Regression: Etherscan's per-second rate-limit response returns `result`
    // as a bare STRING, not an array. The tolerant deserializer must yield an
    // empty Vec (callers gate on `status`) instead of failing JSON decode and
    // aborting the whole address sync.
    #[test]
    fn test_ratelimit_string_result_decodes_to_empty() {
        let body = r#"{"status":"0","message":"NOTOK","result":"Max calls per sec rate limit reached (3/sec)"}"#;
        let data: EtherscanTxResponse =
            serde_json::from_str(body).expect("rate-limit body must not fail decode");
        assert_eq!(data.status, "0");
        assert!(data.result.is_empty());

        let tok: EtherscanTokenResponse =
            serde_json::from_str(body).expect("rate-limit body must not fail token decode");
        assert!(tok.result.is_empty());
    }

    // Regression: V2 `txlistinternal` rows omit gasPrice (and some omit `to`);
    // the shared EthTransaction struct must tolerate their absence.
    #[test]
    fn test_internal_tx_without_gasprice_decodes() {
        let body = r#"{"status":"1","message":"OK","result":[
            {"blockNumber":"123","timeStamp":"1700000000","hash":"0xabc",
             "from":"0xfrom","to":"0xto","value":"1000","contractAddress":"",
             "input":"","type":"call","gas":"21000","gasUsed":"21000",
             "traceId":"0","isError":"0","errCode":""}
        ]}"#;
        let data: EtherscanTxResponse =
            serde_json::from_str(body).expect("internal tx (no gasPrice) must decode");
        assert_eq!(data.result.len(), 1);
        assert_eq!(data.result[0].gas_price, ""); // defaulted
        assert_eq!(data.result[0].hash, "0xabc");
    }

    fn sample_token_tx() -> TokenTransaction {
        TokenTransaction {
            contract_address: "0xaaa".to_string(),
            token_symbol: "aEthUSDT".to_string(),
            token_name: "Aave Ethereum USDT".to_string(),
            token_decimal: "18".to_string(),
            value: "12413775463000000000000".to_string(), // 12413.775463 aEthUSDT
            from: "0xsender".to_string(),
            to: "0xme".to_string(),
            hash: "0xhash".to_string(),
            time_stamp: "1700000000".to_string(),
            block_number: "12345".to_string(),
        }
    }

    #[test]
    fn test_token_transfer_to_wallet_tx_maps_direction_and_decimals() {
        let tx = token_transfer_to_wallet_tx(&sample_token_tx(), "0xme").unwrap();
        assert_eq!(tx.direction, TransactionDirection::Incoming);
        assert_eq!(tx.asset, "aEthUSDT");
        assert_eq!(tx.amount, Decimal::from_str("12413.775463").unwrap());
        assert_eq!(tx.external_id, "0xhash-0xaaa");
        assert_eq!(tx.block_height, Some(12345));
        assert_eq!(tx.counterparty.as_deref(), Some("0xsender"));
        assert_eq!(tx.fee, None); // tokentx carries no gas; native tx gas is separate

        // Outgoing from the sender's perspective.
        let out = token_transfer_to_wallet_tx(&sample_token_tx(), "0xsender").unwrap();
        assert_eq!(out.direction, TransactionDirection::Outgoing);
        assert_eq!(out.counterparty.as_deref(), Some("0xme"));

        // An unrelated address yields None (row doesn't touch it).
        assert!(token_transfer_to_wallet_tx(&sample_token_tx(), "0xother").is_none());

        // Zero value is skipped.
        let mut zero = sample_token_tx();
        zero.value = "0".to_string();
        assert!(token_transfer_to_wallet_tx(&zero, "0xme").is_none());

        // Empty symbol is a scam token — skipped.
        let mut scam = sample_token_tx();
        scam.token_symbol = "".to_string();
        assert!(token_transfer_to_wallet_tx(&scam, "0xme").is_none());
    }
}
