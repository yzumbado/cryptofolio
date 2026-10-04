use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Supported blockchain networks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chain {
    Bitcoin,
    Ethereum,
    Cardano,
    Solana,
    Bittensor,
}

impl Chain {
    /// Native asset ticker for the chain.
    pub fn native_asset(&self) -> &str {
        match self {
            Chain::Bitcoin => "BTC",
            Chain::Ethereum => "ETH",
            Chain::Cardano => "ADA",
            Chain::Solana => "SOL",
            Chain::Bittensor => "TAO",
        }
    }

    /// Decimal precision for the native asset.
    pub fn decimals(&self) -> u8 {
        match self {
            Chain::Bitcoin => 8,
            Chain::Ethereum => 18,
            Chain::Cardano => 6,
            Chain::Solana => 9,
            Chain::Bittensor => 9,
        }
    }

    /// Canonical lowercase name, used in DB and logs.
    pub fn as_str(&self) -> &str {
        match self {
            Chain::Bitcoin => "bitcoin",
            Chain::Ethereum => "ethereum",
            Chain::Cardano => "cardano",
            Chain::Solana => "solana",
            Chain::Bittensor => "bittensor",
        }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Chain {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bitcoin" | "btc" => Ok(Chain::Bitcoin),
            "ethereum" | "eth" => Ok(Chain::Ethereum),
            "cardano" | "ada" => Ok(Chain::Cardano),
            "solana" | "sol" => Ok(Chain::Solana),
            "bittensor" | "tao" => Ok(Chain::Bittensor),
            other => Err(format!("Unknown chain: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Balance
// ---------------------------------------------------------------------------

/// Balance of a single asset at an address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    /// Ticker symbol, e.g. "BTC", "ETH", "USDC", "ADA"
    pub asset: String,
    /// Canonical asset identifier (contract address, policy ID+name, mint pubkey).
    /// `None` for the chain's native asset.
    pub asset_id: Option<String>,
    /// Human-readable balance (already divided by `decimals`).
    pub quantity: Decimal,
    /// Number of decimal places for this asset.
    pub decimals: u8,
}

// ---------------------------------------------------------------------------
// Transactions
// ---------------------------------------------------------------------------

/// Direction of a transaction as seen by the queried address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionDirection {
    /// Funds received at the queried address.
    Incoming,
    /// Funds sent from the queried address.
    Outgoing,
    /// Self-transfer, contract interaction, or otherwise ambiguous.
    Internal,
}

/// A single transaction as seen by a wallet address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    /// Chain-specific transaction identifier (txid, hash, signature).
    pub external_id: String,
    pub direction: TransactionDirection,
    /// Net amount transferred in the native asset (positive = received).
    pub amount: Decimal,
    pub asset: String,
    pub fee: Option<Decimal>,
    pub fee_asset: Option<String>,
    pub block_height: Option<u64>,
    pub timestamp: DateTime<Utc>,
    /// Counterparty address (sender for incoming, recipient for outgoing).
    pub counterparty: Option<String>,
    pub memo: Option<String>,
}

// ---------------------------------------------------------------------------
// Dated on-chain rewards
// ---------------------------------------------------------------------------

/// A dated on-chain reward credit — one transfer of an earned token (e.g. a
/// DePIN daily distribution) with the block time it landed at.
///
/// This is the dated income stream the mining P&L consumes: earned tokens are
/// revenue at $0 cost basis, valued at fair value on the receipt date
/// (`docs/MINING_ASSET_ACCOUNTING.md`, procedure P5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatedReward {
    /// Block time of the transfer — the receipt date for accounting.
    pub date: DateTime<Utc>,
    /// Token symbol resolved from the mint (e.g. "GEOD", "WINGS").
    pub asset: String,
    /// Human-readable quantity received (chain decimals applied).
    pub quantity: Decimal,
    /// Chain transaction identifier (signature) — the ledger dedup key.
    pub signature: String,
}

/// A reward candidate that could not be decoded. Kept so a sync records a
/// skip with a reason instead of silently losing income.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewardDecodeSkip {
    /// Signature of the transaction that could not be decoded.
    pub signature: String,
    /// Human-readable reason (never empty).
    pub reason: String,
}

/// Result of scanning an address for dated rewards.
///
/// `rewards` are the decoded credits; `skipped` are per-row decode failures.
/// A skip never aborts the scan — the other rows are still returned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DatedRewardBatch {
    pub rewards: Vec<DatedReward>,
    pub skipped: Vec<RewardDecodeSkip>,
}

// ---------------------------------------------------------------------------
// Address summary
// ---------------------------------------------------------------------------

/// Full address summary returned by `BlockchainClient::get_address_summary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressSummary {
    pub address: String,
    pub chain: Chain,
    /// Asset balances. The first entry is always the native asset.
    pub balances: Vec<WalletBalance>,
    pub transaction_count: u64,
    /// Chain-specific extras (stake info, DeFi positions, etc.).
    pub extras: Option<ChainExtras>,
}

// ---------------------------------------------------------------------------
// Chain-specific extras
// ---------------------------------------------------------------------------

/// Chain-specific information that doesn't fit the common model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "chain")]
pub enum ChainExtras {
    Cardano {
        stake_address: Option<String>,
        pool_id: Option<String>,
        pool_ticker: Option<String>,
        pool_name: Option<String>,
        active_stake: Option<Decimal>,
        margin_cost: Option<f64>,
    },
    Solana {
        stake_accounts: Vec<SolanaStakeAccount>,
    },
    Bittensor {
        /// Free (liquid) TAO balance.
        free: Decimal,
        /// Total staked TAO (root + alpha-as-tao), valued in TAO.
        staked: Decimal,
    },
    Bitcoin {},
    Ethereum {},
}

/// A Solana stake account delegated from a wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaStakeAccount {
    pub pubkey: String,
    pub lamports: u64,
    pub validator_vote_account: Option<String>,
    pub activation_epoch: Option<u64>,
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

/// Result of a provider health check.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Human-readable provider name (matches `BlockchainClient::provider_name`).
    pub provider: String,
    pub reachable: bool,
    pub block_height: Option<u64>,
    pub latency_ms: u64,
}

impl HealthStatus {
    pub fn reachable(provider: &str, block_height: Option<u64>, latency_ms: u64) -> Self {
        Self {
            provider: provider.to_string(),
            reachable: true,
            block_height,
            latency_ms,
        }
    }

    pub fn unreachable(provider: &str, latency_ms: u64) -> Self {
        Self {
            provider: provider.to_string(),
            reachable: false,
            block_height: None,
            latency_ms,
        }
    }
}
